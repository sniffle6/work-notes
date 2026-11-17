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
  getSavedJiraIssue,
  getSavedJiraIssues,
  loadJiraWriting,
  saveJiraWriting,
  listJiraChatMessages,
  listJiraPendingProposals,
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
import { issueKeyInQuery, jiraTextSearch } from "$lib/jira-list";
import { buildTicketPrompt } from "$lib/jira-prompt";
import type {
  JiraAttachmentDownload,
  JiraAttachmentFile,
  JiraAttachmentView,
  JiraChatMessage,
  JiraChatThread,
  JiraDraftContext,
  JiraFilter,
  JiraIssueDetail,
  JiraIssueDraft,
  JiraIssueEdit,
  JiraIssueSummary,
  JiraStatus,
  JiraViewAction,
  JiraTicketWriting,
  JiraWriting,
} from "$lib/types";

export type { JiraTicketWriting } from "$lib/types";

export type JiraApi = {
  loadJiraWriting: typeof loadJiraWriting;
  saveJiraWriting: typeof saveJiraWriting;
  getJiraStatus: typeof getJiraStatus;
  listJiraIssues: typeof listJiraIssues;
  getJiraIssue: typeof getJiraIssue;
  getSavedJiraIssues: typeof getSavedJiraIssues;
  getSavedJiraIssue: typeof getSavedJiraIssue;
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
  listJiraPendingProposals: typeof listJiraPendingProposals;
  sendJiraChatMessage: typeof sendJiraChatMessage;
  resolveJiraChatProposal: typeof resolveJiraChatProposal;
};

const defaultApi: JiraApi = {
  loadJiraWriting,
  saveJiraWriting,
  getJiraStatus,
  listJiraIssues,
  getJiraIssue,
  getSavedJiraIssues,
  getSavedJiraIssue,
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
  listJiraPendingProposals,
  sendJiraChatMessage,
  resolveJiraChatProposal,
};

// Offered when Jira cannot list a project's issue types.
const FALLBACK_ISSUE_TYPES = ["Task", "Bug", "Story"];

/** The note a ticket draft is started from. */
export type JiraDraftSourceNote = {
  id: string;
  title: string;
  text: string;
};

/** The fields of the new-ticket form the user can edit. */
export type JiraDraftFields = Pick<JiraIssueDraft, "issueType" | "summary" | "description">;

/** Workflow store for the Jira view: ticket list, ticket detail, and agent chat. */
export function createJiraStore(api: JiraApi = defaultApi) {
  const status = writable<JiraStatus | null>(null);
  const activeFilter = writable<JiraFilter | null>(null);
  // The ticket search the agent last put in the list, offered beside the saved filters.
  const agentFilter = writable<JiraFilter | null>(null);
  // The user's last search of Jira itself, offered beside the saved filters.
  const searchFilter = writable<JiraFilter | null>(null);
  const issues = writable<JiraIssueSummary[]>([]);
  const selectedIssue = writable<JiraIssueDetail | null>(null);
  /** When the shown list was saved, while it is a saved copy instead of Jira's current answer. */
  const issuesSavedAt = writable<string | null>(null);
  /** When the open ticket was saved, while it is a saved copy. Changes wait for Jira's current ticket. */
  const issueSavedAt = writable<string | null>(null);
  // The JQL of the list on screen, so a refresh never swaps it for an older saved copy.
  let listedJql: string | null = null;
  let listRequestSequence = 0;
  // The ticket being opened, so a list arriving meanwhile does not open another.
  let openingIssueKey: string | null = null;
  // Tickets changed from this view, with the change count when each was made. A list
  // requested before a change shows that ticket as changed, not as Jira listed it then.
  let changeCount = 0;
  const changedTickets = new Map<string, { summary: JiraIssueSummary; change: number }>();

  function withLaterChanges(items: JiraIssueSummary[], changesBefore: number): JiraIssueSummary[] {
    return items.map((item) => {
      const changed = changedTickets.get(item.key);
      return changed && changed.change > changesBefore ? changed.summary : item;
    });
  }

  function showIssues(items: JiraIssueSummary[], jql: string | null, savedAt: string | null = null): void {
    issues.set(items);
    listedJql = jql;
    issuesSavedAt.set(savedAt);
  }

  function showIssue(issue: JiraIssueDetail | null, savedAt: string | null = null): void {
    selectedIssue.set(issue);
    issueSavedAt.set(savedAt);
  }
  const ticketWriting = writable<Record<string, JiraTicketWriting>>({});
  const selectedWriting = derived([selectedIssue, ticketWriting], ([$issue, $writing]) =>
    $issue ? $writing[$issue.key] : undefined,
  );
  const issueTypes = writable<string[]>(FALLBACK_ISSUE_TYPES);
  const draft = writable<JiraIssueDraft | null>(null);
  const draftVisible = writable(false);
  const draftBeforeAgent = writable<JiraIssueDraft | null>(null);
  /** The agent panel was opened for the draft, whatever the user's saved panel choice. */
  const draftAgentOpen = writable(false);
  const loadingIssues = writable(false);
  const loadingIssue = writable(false);
  const savingIssue = writable(false);
  const error = writable<string | null>(null);
  const writingError = writable<string | null>(null);
  const writingSaving = writable(false);
  const writingReady = writable(false);
  let writingScope: string | null = null;
  let restoringWriting = false;
  let writingLoad: Promise<boolean> | null = null;
  let writingSaves = Promise.resolve();
  let writingRevision = 0;

  function persistWriting(): Promise<void> {
    if (!writingScope || restoringWriting) {
      return writingSaves;
    }
    const scope = writingScope;
    const revision = ++writingRevision;
    const writing: JiraWriting = structuredClone({ draft: get(draft), tickets: get(ticketWriting) });
    writingSaving.set(true);
    // Serialize saves: an older disk write must never finish after a newer one.
    writingSaves = writingSaves.then(async () => {
      try {
        await api.saveJiraWriting(scope, writing);
        if (revision === writingRevision) {
          writingError.set(null);
        }
      } catch (failure) {
        writingError.set(`Writing is not saved on this device: ${errorMessage(failure, "Storage unavailable.")}`);
      } finally {
        if (revision === writingRevision) {
          writingSaving.set(false);
        }
      }
    });
    return writingSaves;
  }

  async function restoreWriting(): Promise<boolean> {
    if (writingLoad) {
      return writingLoad;
    }
    writingLoad = (async () => {
      try {
        const setup = get(status) ?? await api.getJiraStatus();
        status.set(setup);
        const scope = JSON.stringify([setup.siteUrl.replace(/\/$/, "").toLowerCase(), setup.projectKey.toUpperCase()]);
        if (scope === writingScope) {
          writingReady.set(true);
          return true;
        }
        writingReady.set(false);
        await writingSaves;
        if (writingScope && get(writingError)) {
          throw new Error("Save the current project's writing before switching projects.");
        }
        const saved = await api.loadJiraWriting(scope);
        restoringWriting = true;
        draftGeneration += 1;
        draftBeforeAgent.set(null);
        draftAgentOpen.set(false);
        if (writingScope !== null) {
          showIssue(null);
          showIssues([], null);
          changedTickets.clear();
        }
        draft.set(saved.draft);
        draftVisible.set(false);
        // A restored custom issue type must remain selectable before Jira loads its types.
        if (saved.draft && !get(issueTypes).includes(saved.draft.issueType)) {
          issueTypes.update((types) => [...types, saved.draft!.issueType]);
        }
        ticketWriting.set(saved.tickets);
        writingScope = scope;
        writingReady.set(true);
        writingError.set(null);
        return true;
      } catch (failure) {
        writingError.set(`Saved writing could not be loaded: ${errorMessage(failure, "Storage unavailable.")} Retry before starting a draft.`);
        return false;
      } finally {
        restoringWriting = false;
      }
    })();
    try {
      return await writingLoad;
    } finally {
      writingLoad = null;
    }
  }

  async function retryWriting(): Promise<void> {
    if (writingScope) {
      await persistWriting();
    }
    if (await restoreWriting()) {
      await open();
    }
  }

  draft.subscribe(() => { void persistWriting(); });
  ticketWriting.subscribe(() => { void persistWriting(); });

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

  const pendingProposals = writable<JiraChatMessage[]>([]);
  const pendingProposalsError = writable<string | null>(null);
  const pendingProposalsLoading = writable(false);
  const focusedProposalId = writable<string | null>(null);
  let pendingRequest = 0;
  let threadNavigation = 0;
  const ticketDrafts = derived(ticketWriting, (writing) => Object.fromEntries(
    Object.entries(writing).filter(([, value]) => value.editing || value.comment.trim()),
  ));
  const threads = writable<JiraChatThread[]>([]);
  const activeThreadId = writable<string | null>(null);
  const messages = writable<JiraChatMessage[]>([]);
  const agentBusy = writable(false);
  const busyProposalId = writable<string | null>(null);
  const chatError = writable<string | null>(null);

  // Guards against a slow ticket response replacing a newer selection.
  let issueRequestSequence = 0;
  let draftGeneration = 0;
  let agentDraftGeneration: number | null = null;

  /** Loads setup status, then tickets and conversations when Jira is usable. */
  async function open(): Promise<void> {
    error.set(null);

    try {
      const nextStatus = await api.getJiraStatus();
      status.set(nextStatus);
      if (!await restoreWriting()) {
        return;
      }
      if (!nextStatus.configured) {
        showIssues([], null);
        showIssue(null);
        return;
      }

      const current = get(activeFilter);
      if (!current || (current !== get(agentFilter) && current !== get(searchFilter))) {
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
  function refreshIssues(): Promise<void> {
    return reloadIssues(null);
  }

  /**
   * Reloads the active filter. The open ticket stays open when it is listed or
   * is `pinnedKey`; otherwise the first listed ticket opens.
   */
  async function reloadIssues(pinnedKey: string | null): Promise<void> {
    const filter = get(activeFilter);
    if (!filter) {
      showIssues([], null);
      return;
    }

    const request = ++listRequestSequence;
    const changesBefore = changeCount;
    loadingIssues.set(true);
    error.set(null);
    const current = trackSettled(api.listJiraIssues(filter.jql));

    // A saved copy stands in only for a list that is not on screen yet.
    if (listedJql !== filter.jql) {
      const saved = await api.getSavedJiraIssues(filter.jql).catch(() => null);
      if (saved && request === listRequestSequence && !current.resolved) {
        showIssues(saved.value, filter.jql, saved.savedAt);
        void keepOrOpenFirst(saved.value, pinnedKey);
      }
    }

    try {
      const items = withLaterChanges(await current.promise, changesBefore);
      if (request !== listRequestSequence) {
        return;
      }
      showIssues(items, filter.jql);
      await keepOrOpenFirst(items, pinnedKey);
    } catch (unknownError) {
      if (request === listRequestSequence) {
        error.set(errorMessage(unknownError, "Could not load Jira tickets."));
      }
    } finally {
      if (request === listRequestSequence) {
        loadingIssues.set(false);
      }
    }
  }

  /** Keeps the open ticket when it is listed or pinned; otherwise opens the first listed ticket. */
  async function keepOrOpenFirst(items: JiraIssueSummary[], pinnedKey: string | null): Promise<void> {
    const selectedKey = openingIssueKey ?? get(selectedIssue)?.key;
    if (selectedKey && (selectedKey === pinnedKey || items.some((item) => item.key === selectedKey))) {
      // A saved copy left on screen by a failed load is asked for again.
      if (!openingIssueKey && get(issueSavedAt)) {
        await loadIssue(selectedKey);
      }
      return;
    }
    if (items.length === 0) {
      issueRequestSequence += 1;
      openingIssueKey = null;
      showIssue(null);
      return;
    }
    await selectIssue(items[0].key);
  }

  /**
   * Opens a ticket. A saved copy shows at once when another ticket, or none, is
   * open; Jira's current ticket replaces it when it arrives.
   */
  async function selectIssue(key: string): Promise<void> {
    hideDraft();
    await loadIssue(key);
  }

  async function loadIssue(key: string): Promise<void> {
    const request = ++issueRequestSequence;
    openingIssueKey = key;
    loadingIssue.set(true);
    error.set(null);
    const current = trackSettled(api.getJiraIssue(key));

    if (get(selectedIssue)?.key !== key) {
      const saved = await api.getSavedJiraIssue(key).catch(() => null);
      if (saved && request === issueRequestSequence && !current.resolved) {
        showIssue(saved.value, saved.savedAt);
      }
    }

    try {
      const issue = await current.promise;
      if (request === issueRequestSequence) {
        showIssue(issue);
      }
    } catch (unknownError) {
      if (request === issueRequestSequence) {
        // Jira no longer returns the ticket, so no copy of it may stay on screen.
        if (isJiraNotFound(unknownError) && get(selectedIssue)?.key === key) {
          showIssue(null);
        }
        error.set(errorMessage(unknownError, "Could not load the ticket."));
      }
    } finally {
      if (request === issueRequestSequence) {
        openingIssueKey = null;
        loadingIssue.set(false);
      }
    }
  }

  /**
   * Opens or resumes the new-ticket form. Returns false when a prefilled draft
   * cannot replace existing writing; callers must not ask the agent to rewrite it.
   */
  async function startDraft(prefill: Partial<JiraIssueDraft> = {}): Promise<boolean> {
    if (!await restoreWriting()) {
      return false;
    }
    const current = get(draft);
    if (current) {
      draftAgentOpen.set(false);
      draftVisible.set(true);
      const requestsAnotherDraft = Object.keys(prefill).length > 0;
      if (requestsAnotherDraft && current.noteId !== prefill.noteId) {
        error.set("Finish or discard the current draft before creating a ticket from another note.");
      }
      return !requestsAnotherDraft;
    }
    error.set(null);
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

    draftAgentOpen.set(false);
    draftBeforeAgent.set(null);
    draftGeneration += 1;
    draft.set({
      issueType: prefill.issueType && types.includes(prefill.issueType) ? prefill.issueType : types[0],
      summary: prefill.summary ?? "",
      description: prefill.description ?? "",
      noteId: prefill.noteId ?? null,
    });
    draftVisible.set(true);
    return true;
  }

  /** Hides the saved unfinished ticket while browsing. */
  function hideDraft(): void {
    draftVisible.set(false);
    draftAgentOpen.set(false);
  }

  function cancelDraft(): void {
    draftGeneration += 1;
    draftBeforeAgent.set(null);
    draft.set(null);
    hideDraft();
    error.set(null);
  }

  /** Records what the user typed into the new-ticket form. */
  function editDraft(fields: JiraDraftFields): void {
    draftBeforeAgent.set(null);
    draft.update((current) => (current ? { ...current, ...fields } : current));
  }

  /** Restores the form as it stood immediately before the last agent rewrite. */
  function undoDraftRewrite(): void {
    const previous = get(draftBeforeAgent);
    if (previous) {
      draft.set(previous);
      draftBeforeAgent.set(null);
    }
  }

  /**
   * Opens a draft holding the note's own text, then asks the agent in a new
   * conversation to write the ticket from it. The note text stays in the form
   * when the agent is busy, fails, or Jira is not set up.
   */
  async function draftFromNote(note: JiraDraftSourceNote): Promise<void> {
    if (!await startDraft({ summary: note.title, description: note.text, noteId: note.id })) {
      return;
    }
    if (!get(status)?.configured || get(agentBusy)) {
      return;
    }

    draftAgentOpen.set(true);
    newThread();
    await sendChat(draftRequest(note.title));
  }

  /** Stops showing the agent panel just because a draft is open. */
  function closeDraftAgent(): void {
    draftAgentOpen.set(false);
  }

  // The agent is shown the form in place of the ticket behind it.
  function draftContext(): JiraDraftContext | null {
    const current = get(draft);
    if (!current || !get(draftVisible)) {
      return null;
    }
    return {
      issueType: current.issueType,
      summary: current.summary,
      description: current.description,
      issueTypes: get(issueTypes),
      noteId: current.noteId ?? null,
    };
  }

  /** Creates the ticket and selects it. Returns its key, or undefined on failure. */
  async function submitDraft(nextDraft: JiraIssueDraft): Promise<string | undefined> {
    savingIssue.set(true);
    error.set(null);

    try {
      const created = await api.createJiraIssue(nextDraft);
      draftBeforeAgent.set(null);
      draft.set(null);
      hideDraft();
      issueRequestSequence += 1;
      openingIssueKey = null;
      showIssue(created);
      // The new ticket stays open even when the filter does not match it or
      // Jira's search has not caught up with it yet.
      await reloadIssues(created.key);
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
    if (get(issueSavedAt)) {
      error.set("This is a saved copy of the ticket. Wait for Jira to send the current one before changing it.");
      return false;
    }

    savingIssue.set(true);
    error.set(null);

    try {
      const updated = await change(key);
      changeCount += 1;
      changedTickets.set(key, { summary: toSummary(updated), change: changeCount });
      // A load of this ticket that began before the change would show it as it was.
      if (openingIssueKey === null || openingIssueKey === key) {
        issueRequestSequence += 1;
        openingIssueKey = null;
        loadingIssue.set(false);
      }
      if (get(selectedIssue)?.key === key) {
        showIssue(updated);
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

  function rememberWriting(key: string, writing: JiraTicketWriting): void {
    ticketWriting.update((current) => {
      const next = { ...current };
      if (!writing.editing && !writing.comment) {
        delete next[key];
      } else {
        next[key] = writing;
      }
      return next;
    });
  }

  async function commentOnSelected(text: string): Promise<boolean> {
    const key = get(selectedIssue)?.key;
    const success = await changeSelected((key) => api.addJiraComment(key, text), "Could not add the comment.");
    const writing = key ? get(ticketWriting)[key] : undefined;
    if (success && key && writing && writing.comment.trim() === text.trim()) {
      rememberWriting(key, { ...writing, comment: "" });
    }
    return success;
  }

  async function saveSelectedEdit(edit: JiraIssueEdit): Promise<boolean> {
    const key = get(selectedIssue)?.key;
    const success = await changeSelected((key) => api.updateJiraIssue(key, edit), "Could not save the ticket.");
    const writing = key ? get(ticketWriting)[key] : undefined;
    if (success && key && writing) {
      rememberWriting(key, { ...writing, editing: false });
    }
    return success;
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

  async function refreshPendingProposals(): Promise<void> {
    const request = ++pendingRequest;
    pendingProposalsLoading.set(true);
    try {
      const pending = await api.listJiraPendingProposals();
      if (request === pendingRequest) {
        pendingProposals.set(pending);
        pendingProposalsError.set(null);
      }
    } catch (unknownError) {
      if (request === pendingRequest) {
        pendingProposalsError.set(errorMessage(unknownError, "Could not load pending approvals."));
      }
    } finally {
      if (request === pendingRequest) {
        pendingProposalsLoading.set(false);
      }
    }
  }

  async function openProposal(messageId: string): Promise<void> {
    const proposal = get(pendingProposals).find((message) => message.id === messageId);
    if (!proposal || get(agentBusy) || get(busyProposalId)) {
      return;
    }
    const navigation = threadNavigation + 1;
    await openThread(proposal.threadId);
    if (navigation === threadNavigation && get(activeThreadId) === proposal.threadId && get(messages).some((message) => message.id === messageId)) {
      focusedProposalId.set(messageId);
    }
  }

  async function loadThreads(): Promise<void> {
    try {
      threads.set(await api.listJiraChatThreads());
    } catch (unknownError) {
      chatError.set(errorMessage(unknownError, "Could not load conversations."));
    }
    await refreshPendingProposals();
  }

  async function loadMessages(threadId: string): Promise<void> {
    const navigation = threadNavigation;
    const loaded = await api.listJiraChatMessages(threadId);
    if (navigation === threadNavigation && get(activeThreadId) === threadId) {
      messages.set(loaded);
    }
  }

  async function openThread(threadId: string): Promise<void> {
    const navigation = ++threadNavigation;
    focusedProposalId.set(null);
    activeThreadId.set(threadId);
    messages.set([]);
    chatError.set(null);

    try {
      await loadMessages(threadId);
    } catch (unknownError) {
      if (navigation === threadNavigation) {
        chatError.set(errorMessage(unknownError, "Could not load the conversation."));
      }
    }
  }

  /** Shows an empty conversation. The thread is created with its first message. */
  function newThread(): void {
    threadNavigation++;
    focusedProposalId.set(null);
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
    focusedProposalId.set(null);
    chatError.set(null);
    let threadId = get(activeThreadId);

    try {
      if (!threadId) {
        threadId = (await api.createJiraChatThread()).id;
        activeThreadId.set(threadId);
      }

      messages.update((current) => [...current, optimisticUserMessage(threadId as string, trimmed)]);
      const openDraft = draftContext();
      agentDraftGeneration = openDraft ? draftGeneration : null;
      const focusIssueKey = openDraft ? null : (get(selectedIssue)?.key ?? null);
      const updated = await api.sendJiraChatMessage(threadId, trimmed, focusIssueKey, openDraft);
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
      await refreshPendingProposals();
    }
  }

  /** Applies a change the agent made to what the Jira view shows. */
  async function applyViewAction(action: JiraViewAction): Promise<void> {
    if (action.kind === "openIssue") {
      await selectIssue(action.issueKey);
      return;
    }
    if (action.kind === "setDraft") {
      rewriteDraft(action);
      return;
    }

    const filter: JiraFilter = { name: action.title, jql: action.jql };
    agentFilter.set(filter);
    await selectFilter(filter);
  }

  // A draft closed while the agent worked stays closed.
  function rewriteDraft(rewrite: Extract<JiraViewAction, { kind: "setDraft" }>): void {
    if (!get(draftVisible) || (get(agentBusy) && agentDraftGeneration !== draftGeneration)) {
      return;
    }
    const offered = get(issueTypes);
    draft.update((current) => {
      if (!current) {
        return current;
      }
      draftBeforeAgent.set(current);
      return {
        ...current,
        issueType: rewrite.issueType && offered.includes(rewrite.issueType) ? rewrite.issueType : current.issueType,
        summary: rewrite.summary ?? current.summary,
        description: rewrite.description ?? current.description,
      };
    });
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

  /**
   * Looks beyond the loaded list: opens the ticket when the query is a key,
   * otherwise lists Jira's text matches as a temporary filter.
   */
  async function searchJira(query: string): Promise<void> {
    const key = issueKeyInQuery(query);
    if (key) {
      await selectIssue(key);
      return;
    }
    const search = jiraTextSearch(query, get(status)?.projectKey ?? "");
    if (!search) {
      return;
    }
    searchFilter.set(search);
    await selectFilter(search);
  }

  /** Drops the Jira search and returns to the first saved filter when it was shown. */
  async function clearSearchFilter(): Promise<void> {
    const cleared = get(searchFilter);
    searchFilter.set(null);
    if (!cleared || get(activeFilter) !== cleared) {
      return;
    }

    activeFilter.set(get(status)?.filters[0] ?? null);
    await refreshIssues();
  }

  /** Refreshes the open conversation when the backend stored new messages. */
  async function handleThreadUpdated(threadId: string): Promise<void> {
    await refreshPendingProposals();
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
    ticketDrafts,
    pendingProposals,
    pendingProposalsError,
    pendingProposalsLoading,
    focusedProposalId,
    refreshPendingProposals,
    openProposal,
    writingError,
    writingSaving,
    writingReady,
    retryWriting,
    status,
    activeFilter,
    agentFilter,
    searchFilter,
    searchJira,
    clearSearchFilter,
    issues,
    issuesSavedAt,
    selectedIssue,
    issueSavedAt,
    selectedWriting,
    rememberWriting,
    issueTypes,
    draft,
    draftVisible,
    draftBeforeAgent,
    undoDraftRewrite,
    draftAgentOpen,
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
    hideDraft,
    editDraft,
    draftFromNote,
    closeDraftAgent,
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

/** True for the error the backend reports when Jira answers 404: the ticket is gone or hidden. */
function isJiraNotFound(error: unknown): boolean {
  return Boolean(error) && typeof error === "object" && (error as { code?: unknown }).code === "jira_not_found";
}

/** Wraps a promise so callers can check whether it has resolved yet. */
function trackSettled<T>(promise: Promise<T>): { promise: Promise<T>; readonly resolved: boolean } {
  let resolved = false;
  // Rejections are handled where the promise is awaited.
  promise.then(() => { resolved = true; }, () => undefined);
  return {
    promise,
    get resolved() {
      return resolved;
    },
  };
}

function draftRequest(noteTitle: string): string {
  const title = noteTitle.trim();
  return [
    title ? `Draft a ticket from my note: ${title}` : "Draft a ticket from my note",
    "Write the summary and description from the source note, and pick the issue type that fits.",
  ].join("\n");
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
