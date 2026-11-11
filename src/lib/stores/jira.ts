import { derived, get, writable } from "svelte/store";

import {
  addJiraComment,
  assignJiraIssueToMe,
  createJiraChatThread,
  createJiraIssue,
  deleteJiraChatThread,
  downloadJiraAttachments,
  getJiraAttachmentThumbnail,
  getJiraIssue,
  getJiraStatus,
  listJiraChatMessages,
  listJiraChatThreads,
  listJiraIssues,
  listJiraIssueTypes,
  readJiraAttachmentMedia,
  resolveJiraChatProposal,
  revealJiraAttachment,
  sendJiraChatMessage,
  transitionJiraIssue,
  updateJiraIssue,
} from "$lib/api";
import { buildTicketPrompt } from "$lib/jira-prompt";
import type {
  JiraAttachmentDownload,
  JiraAttachmentFile,
  JiraAttachmentView,
  JiraChatMessage,
  JiraChatThread,
  JiraFilter,
  JiraIssueDetail,
  JiraIssueDraft,
  JiraIssueEdit,
  JiraIssueSummary,
  JiraStatus,
  JiraViewAction,
} from "$lib/types";

export type JiraApi = {
  getJiraStatus: typeof getJiraStatus;
  listJiraIssues: typeof listJiraIssues;
  getJiraIssue: typeof getJiraIssue;
  listJiraIssueTypes: typeof listJiraIssueTypes;
  createJiraIssue: typeof createJiraIssue;
  updateJiraIssue: typeof updateJiraIssue;
  transitionJiraIssue: typeof transitionJiraIssue;
  addJiraComment: typeof addJiraComment;
  assignJiraIssueToMe: typeof assignJiraIssueToMe;
  downloadJiraAttachments: typeof downloadJiraAttachments;
  getJiraAttachmentThumbnail: typeof getJiraAttachmentThumbnail;
  readJiraAttachmentMedia: typeof readJiraAttachmentMedia;
  revealJiraAttachment: typeof revealJiraAttachment;
  listJiraChatThreads: typeof listJiraChatThreads;
  createJiraChatThread: typeof createJiraChatThread;
  deleteJiraChatThread: typeof deleteJiraChatThread;
  listJiraChatMessages: typeof listJiraChatMessages;
  sendJiraChatMessage: typeof sendJiraChatMessage;
  resolveJiraChatProposal: typeof resolveJiraChatProposal;
};

const defaultApi: JiraApi = {
  getJiraStatus,
  listJiraIssues,
  getJiraIssue,
  listJiraIssueTypes,
  createJiraIssue,
  updateJiraIssue,
  transitionJiraIssue,
  addJiraComment,
  assignJiraIssueToMe,
  downloadJiraAttachments,
  getJiraAttachmentThumbnail,
  readJiraAttachmentMedia,
  revealJiraAttachment,
  listJiraChatThreads,
  createJiraChatThread,
  deleteJiraChatThread,
  listJiraChatMessages,
  sendJiraChatMessage,
  resolveJiraChatProposal,
};

// Offered when Jira cannot list a project's issue types.
const FALLBACK_ISSUE_TYPES = ["Task", "Bug", "Story"];

/** Workflow store for the Jira view: ticket list, ticket detail, and agent chat. */
export function createJiraStore(api: JiraApi = defaultApi) {
  const status = writable<JiraStatus | null>(null);
  const activeFilter = writable<JiraFilter | null>(null);
  // The ticket search the agent last put in the list, offered beside the saved filters.
  const agentFilter = writable<JiraFilter | null>(null);
  const issues = writable<JiraIssueSummary[]>([]);
  const selectedIssue = writable<JiraIssueDetail | null>(null);
  const issueTypes = writable<string[]>(FALLBACK_ISSUE_TYPES);
  const draft = writable<JiraIssueDraft | null>(null);
  const loadingIssues = writable(false);
  const loadingIssue = writable(false);
  const savingIssue = writable(false);
  const error = writable<string | null>(null);

  // Attachments saved so far for one ticket; a download for another ticket replaces it.
  const savedAttachments = writable<(JiraAttachmentDownload & { issueKey: string }) | null>(null);
  const downloadingAttachments = writable(false);
  /** The open ticket's attachments that are saved to disk, if any. */
  const attachmentDownload = derived([savedAttachments, selectedIssue], ([saved, issue]) =>
    saved && saved.issueKey === issue?.key ? saved : null,
  );
  /** Preview image URLs by attachment id, kept for every ticket opened so far. */
  const attachmentThumbnails = writable<Record<string, string>>({});
  // Each thumbnail is asked for once, whether or not it loads.
  const requestedThumbnails = new Set<string>();
  /** The image or video open in the in-app viewer, if any. */
  const attachmentViewer = writable<JiraAttachmentView | null>(null);
  const openingAttachmentId = writable<string | null>(null);
  // The ticket the viewer's file belongs to.
  let viewerIssueKey: string | null = null;
  selectedIssue.subscribe((issue) => {
    if (issue?.key !== viewerIssueKey) {
      closeAttachmentViewer();
    }
    if (issue) {
      void loadThumbnails(issue);
    }
  });

  function closeAttachmentViewer(): void {
    const open = get(attachmentViewer);
    if (open) {
      URL.revokeObjectURL(open.url);
      attachmentViewer.set(null);
    }
    viewerIssueKey = null;
  }

  /**
   * Opens one of the open ticket's image or video attachments in the viewer. The file
   * is downloaded to the ticket folder first when it is not saved yet.
   */
  async function openAttachment(attachmentId: string): Promise<void> {
    const issue = get(selectedIssue);
    const attachment = issue?.attachments.find((candidate) => candidate.id === attachmentId);
    if (!issue || !attachment || get(openingAttachmentId)) {
      return;
    }

    openingAttachmentId.set(attachmentId);
    try {
      const file = (await downloadAttachments(issue.key, attachmentId))?.[0];
      if (!file) {
        return;
      }
      const { url, kind } = await api.readJiraAttachmentMedia(issue.key, attachmentId);
      if (get(selectedIssue)?.key !== issue.key) {
        URL.revokeObjectURL(url);
        return;
      }
      closeAttachmentViewer();
      viewerIssueKey = issue.key;
      attachmentViewer.set({ attachmentId, filename: attachment.filename, url, kind, path: file.path });
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not open the attachment."));
    } finally {
      openingAttachmentId.set(null);
    }
  }

  /** Loads the previews Jira offers for a ticket's attachments, one at a time. */
  async function loadThumbnails(issue: JiraIssueDetail): Promise<void> {
    for (const attachment of issue.attachments) {
      if (!attachment.hasThumbnail || requestedThumbnails.has(attachment.id)) {
        continue;
      }
      requestedThumbnails.add(attachment.id);
      try {
        const url = await api.getJiraAttachmentThumbnail(attachment.id);
        attachmentThumbnails.update((current) => ({ ...current, [attachment.id]: url }));
      } catch {
        // A preview is optional; the attachment is listed without one.
      }
    }
  }

  const threads = writable<JiraChatThread[]>([]);
  const activeThreadId = writable<string | null>(null);
  const messages = writable<JiraChatMessage[]>([]);
  const agentBusy = writable(false);
  const busyProposalId = writable<string | null>(null);
  const chatError = writable<string | null>(null);

  // Guards against a slow ticket response replacing a newer selection.
  let issueRequestSequence = 0;

  /** Loads setup status, then tickets and conversations when Jira is usable. */
  async function open(): Promise<void> {
    error.set(null);

    try {
      const nextStatus = await api.getJiraStatus();
      status.set(nextStatus);
      if (!nextStatus.configured) {
        issues.set([]);
        selectedIssue.set(null);
        return;
      }

      const current = get(activeFilter);
      if (!current || current !== get(agentFilter)) {
        activeFilter.set(
          nextStatus.filters.find((candidate) => candidate.name === current?.name) ??
            nextStatus.filters[0] ??
            null,
        );
      }
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not read Jira settings."));
      return;
    }

    await Promise.all([refreshIssues(), loadThreads()]);
  }

  async function selectFilter(filter: JiraFilter): Promise<void> {
    activeFilter.set(filter);
    await refreshIssues();
  }

  /** Reloads the active filter and keeps the selection when it is still listed. */
  async function refreshIssues(): Promise<void> {
    const filter = get(activeFilter);
    if (!filter) {
      issues.set([]);
      return;
    }

    loadingIssues.set(true);
    error.set(null);

    try {
      const items = await api.listJiraIssues(filter.jql);
      issues.set(items);

      const selectedKey = get(selectedIssue)?.key;
      if (selectedKey && items.some((item) => item.key === selectedKey)) {
        return;
      }
      if (items.length === 0) {
        issueRequestSequence += 1;
        selectedIssue.set(null);
        return;
      }
      await selectIssue(items[0].key);
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load Jira tickets."));
    } finally {
      loadingIssues.set(false);
    }
  }

  async function selectIssue(key: string): Promise<void> {
    const request = ++issueRequestSequence;
    loadingIssue.set(true);
    error.set(null);

    try {
      const issue = await api.getJiraIssue(key);
      if (request === issueRequestSequence) {
        selectedIssue.set(issue);
      }
    } catch (unknownError) {
      if (request === issueRequestSequence) {
        error.set(errorMessage(unknownError, "Could not load the ticket."));
      }
    } finally {
      if (request === issueRequestSequence) {
        loadingIssue.set(false);
      }
    }
  }

  /** Opens the new-ticket form, optionally prefilled (for example from a note). */
  async function startDraft(prefill: Partial<JiraIssueDraft> = {}): Promise<void> {
    let types = FALLBACK_ISSUE_TYPES;
    try {
      const listed = await api.listJiraIssueTypes();
      if (listed.length > 0) {
        types = listed;
      }
    } catch {
      // The form still works with the common issue types.
    }
    issueTypes.set(types);

    draft.set({
      issueType: prefill.issueType && types.includes(prefill.issueType) ? prefill.issueType : types[0],
      summary: prefill.summary ?? "",
      description: prefill.description ?? "",
      noteId: prefill.noteId ?? null,
    });
  }

  function cancelDraft(): void {
    draft.set(null);
  }

  /** Creates the ticket and selects it. Returns its key, or undefined on failure. */
  async function submitDraft(nextDraft: JiraIssueDraft): Promise<string | undefined> {
    savingIssue.set(true);
    error.set(null);

    try {
      const created = await api.createJiraIssue(nextDraft);
      draft.set(null);
      issueRequestSequence += 1;
      selectedIssue.set(created);
      await refreshIssues();
      return created.key;
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not create the ticket."));
      return undefined;
    } finally {
      savingIssue.set(false);
    }
  }

  async function changeSelected(
    change: (key: string) => Promise<JiraIssueDetail>,
    fallbackMessage: string,
  ): Promise<boolean> {
    const key = get(selectedIssue)?.key;
    if (!key) {
      return false;
    }

    savingIssue.set(true);
    error.set(null);

    try {
      const updated = await change(key);
      if (get(selectedIssue)?.key === key) {
        selectedIssue.set(updated);
      }
      issues.update((items) => items.map((item) => (item.key === key ? toSummary(updated) : item)));
      return true;
    } catch (unknownError) {
      error.set(errorMessage(unknownError, fallbackMessage));
      return false;
    } finally {
      savingIssue.set(false);
    }
  }

  function transitionSelected(transitionId: string): Promise<boolean> {
    return changeSelected((key) => api.transitionJiraIssue(key, transitionId), "Could not move the ticket.");
  }

  function commentOnSelected(text: string): Promise<boolean> {
    return changeSelected((key) => api.addJiraComment(key, text), "Could not add the comment.");
  }

  function saveSelectedEdit(edit: JiraIssueEdit): Promise<boolean> {
    return changeSelected((key) => api.updateJiraIssue(key, edit), "Could not save the ticket.");
  }

  function assignSelectedToMe(): Promise<boolean> {
    return changeSelected((key) => api.assignJiraIssueToMe(key), "Could not assign the ticket.");
  }

  /**
   * Saves a ticket's attachments to disk: one when `attachmentId` is given,
   * otherwise all. Returns the saved files, or undefined on failure.
   */
  async function downloadAttachments(
    issueKey: string,
    attachmentId: string | null = null,
  ): Promise<JiraAttachmentFile[] | undefined> {
    downloadingAttachments.set(true);
    error.set(null);

    try {
      const download = await api.downloadJiraAttachments(issueKey, attachmentId ? [attachmentId] : null);
      savedAttachments.update((saved) => {
        const newIds = new Set(download.files.map((file) => file.id));
        const kept = saved?.issueKey === issueKey ? saved.files.filter((file) => !newIds.has(file.id)) : [];
        return { issueKey, folder: download.folder, files: [...kept, ...download.files] };
      });
      return download.files;
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not download the attachments."));
      return undefined;
    } finally {
      downloadingAttachments.set(false);
    }
  }

  async function revealAttachment(path: string): Promise<void> {
    try {
      await api.revealJiraAttachment(path);
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not show the file."));
    }
  }

  /**
   * Builds the coding-agent prompt for a ticket, downloading its attachments
   * first so the prompt can name local files. A failed download is reported
   * and the prompt lists the attachments as not downloaded.
   */
  async function ticketPrompt(issue: JiraIssueDetail): Promise<string> {
    const files = issue.attachments.length > 0 ? await downloadAttachments(issue.key) : [];
    return buildTicketPrompt(issue, files ?? []);
  }

  async function loadThreads(): Promise<void> {
    try {
      threads.set(await api.listJiraChatThreads());
    } catch (unknownError) {
      chatError.set(errorMessage(unknownError, "Could not load conversations."));
    }
  }

  async function loadMessages(threadId: string): Promise<void> {
    const loaded = await api.listJiraChatMessages(threadId);
    if (get(activeThreadId) === threadId) {
      messages.set(loaded);
    }
  }

  async function openThread(threadId: string): Promise<void> {
    activeThreadId.set(threadId);
    messages.set([]);
    chatError.set(null);

    try {
      await loadMessages(threadId);
    } catch (unknownError) {
      chatError.set(errorMessage(unknownError, "Could not load the conversation."));
    }
  }

  /** Shows an empty conversation. The thread is created with its first message. */
  function newThread(): void {
    activeThreadId.set(null);
    messages.set([]);
    chatError.set(null);
  }

  async function deleteThread(threadId: string): Promise<void> {
    chatError.set(null);

    try {
      await api.deleteJiraChatThread(threadId);
      if (get(activeThreadId) === threadId) {
        newThread();
      }
      await loadThreads();
    } catch (unknownError) {
      chatError.set(errorMessage(unknownError, "Could not delete the conversation."));
    }
  }

  /** Sends a message to the agent, creating the thread when none is open. */
  async function sendChat(text: string): Promise<void> {
    const trimmed = text.trim();
    if (!trimmed || get(agentBusy)) {
      return;
    }

    agentBusy.set(true);
    chatError.set(null);
    let threadId = get(activeThreadId);

    try {
      if (!threadId) {
        threadId = (await api.createJiraChatThread()).id;
        activeThreadId.set(threadId);
      }

      messages.update((current) => [...current, optimisticUserMessage(threadId as string, trimmed)]);
      const updated = await api.sendJiraChatMessage(threadId, trimmed, get(selectedIssue)?.key ?? null);
      if (get(activeThreadId) === threadId) {
        messages.set(updated);
      }
    } catch (unknownError) {
      chatError.set(errorMessage(unknownError, "The Jira agent could not answer."));
      if (threadId) {
        await loadMessages(threadId).catch(() => undefined);
      }
    } finally {
      agentBusy.set(false);
    }

    await loadThreads();
  }

  /** Approves (runs) or rejects a change the agent proposed. */
  async function resolveProposal(messageId: string, approve: boolean): Promise<void> {
    busyProposalId.set(messageId);
    chatError.set(null);

    try {
      const updated = await api.resolveJiraChatProposal(messageId, approve);
      const threadId = updated[0]?.threadId;
      if (!threadId || get(activeThreadId) === threadId) {
        messages.set(updated);
      }
      if (approve) {
        // Jira changed, so the ticket list and the open ticket may be stale.
        const selectedKey = get(selectedIssue)?.key;
        await refreshIssues();
        if (selectedKey && get(selectedIssue)?.key === selectedKey) {
          await selectIssue(selectedKey);
        }
      }
    } catch (unknownError) {
      chatError.set(errorMessage(unknownError, "Could not apply the change."));
    } finally {
      busyProposalId.set(null);
    }
  }

  /** Applies a change the agent made to what the Jira view shows. */
  async function applyViewAction(action: JiraViewAction): Promise<void> {
    if (action.kind === "openIssue") {
      draft.set(null);
      await selectIssue(action.issueKey);
      return;
    }

    const filter: JiraFilter = { name: action.title, jql: action.jql };
    agentFilter.set(filter);
    await selectFilter(filter);
  }

  /** Drops the agent's ticket search and returns to the first saved filter. */
  async function clearAgentFilter(): Promise<void> {
    const cleared = get(agentFilter);
    agentFilter.set(null);
    if (!cleared || get(activeFilter) !== cleared) {
      return;
    }

    const fallback = get(status)?.filters[0] ?? null;
    activeFilter.set(fallback);
    await refreshIssues();
  }

  /** Refreshes the open conversation when the backend stored new messages. */
  async function handleThreadUpdated(threadId: string): Promise<void> {
    if (get(activeThreadId) !== threadId) {
      return;
    }

    try {
      await loadMessages(threadId);
    } catch {
      // The final response of the running turn replaces the messages anyway.
    }
  }

  return {
    status,
    activeFilter,
    agentFilter,
    issues,
    selectedIssue,
    issueTypes,
    draft,
    loadingIssues,
    loadingIssue,
    savingIssue,
    error,
    attachmentDownload,
    attachmentThumbnails,
    attachmentViewer,
    openingAttachmentId,
    downloadingAttachments,
    threads,
    activeThreadId,
    messages,
    agentBusy,
    busyProposalId,
    chatError,
    open,
    selectFilter,
    refreshIssues,
    selectIssue,
    startDraft,
    cancelDraft,
    submitDraft,
    transitionSelected,
    commentOnSelected,
    saveSelectedEdit,
    assignSelectedToMe,
    downloadAttachments,
    openAttachment,
    closeAttachmentViewer,
    revealAttachment,
    ticketPrompt,
    loadThreads,
    openThread,
    newThread,
    deleteThread,
    sendChat,
    resolveProposal,
    applyViewAction,
    clearAgentFilter,
    handleThreadUpdated,
  };
}

export type JiraStore = ReturnType<typeof createJiraStore>;

function toSummary(issue: JiraIssueDetail): JiraIssueSummary {
  return {
    key: issue.key,
    summary: issue.summary,
    status: issue.status,
    statusCategory: issue.statusCategory,
    issueType: issue.issueType,
    priority: issue.priority,
    assignee: issue.assignee,
    updated: issue.updated,
    labels: issue.labels,
    url: issue.url,
  };
}

function optimisticUserMessage(threadId: string, content: string): JiraChatMessage {
  return {
    id: `pending-${Date.now()}`,
    threadId,
    role: "user",
    content,
    status: null,
    result: null,
    createdAt: new Date().toISOString(),
  };
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") {
    return error.message;
  }

  return fallback;
}
