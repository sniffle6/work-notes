import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

import type {
  JiraAttachment,
  JiraChatMessage,
  JiraChatThread,
  JiraIssueDetail,
  JiraIssueSummary,
  JiraStatus,
  JiraWriting,
} from "$lib/types";
import { createJiraStore, type JiraApi } from "./jira";

function status(overrides: Partial<JiraStatus> = {}): JiraStatus {
  return {
    configured: true,
    missing: [],
    siteUrl: "https://example.atlassian.net",
    projectKey: "AB",
    filters: [
      { name: "My open tickets", jql: "assignee = currentUser()" },
      { name: "All open", jql: "project = AB" },
    ],
    ...overrides,
  };
}

function issue(overrides: Partial<JiraIssueSummary> = {}): JiraIssueSummary {
  return {
    key: "AB-7",
    summary: "Printer jams on reboot",
    status: "To Do",
    statusCategory: "new",
    issueType: "Bug",
    priority: "High",
    assignee: null,
    updated: "2026-11-01T10:00:00.000+0000",
    labels: [],
    url: "https://example.atlassian.net/browse/AB-7",
    ...overrides,
  };
}

function detail(overrides: Partial<JiraIssueDetail> = {}): JiraIssueDetail {
  return {
    ...issue(),
    description: "Jams after reboot.",
    descriptionEditableLosslessly: true,
    reporter: "Rina Q",
    parentKey: null,
    subtasks: [],
    linkedIssues: [],
    attachments: [],
    comments: [],
    transitions: [{ id: "11", name: "Start work", toStatus: "In Progress" }],
    ...overrides,
  };
}

const ATTACHMENTS: JiraAttachment[] = [
  {
    id: "10001",
    filename: "bench.log",
    mimeType: "text/plain",
    size: 2048,
    author: "Maya Lopez",
    created: "2026-11-02T09:00:00.000Z",
    hasThumbnail: false,
  },
  { id: "10002", filename: "photo.png", mimeType: null, size: 0, author: null, created: null, hasThumbnail: true },
];

function thread(overrides: Partial<JiraChatThread> = {}): JiraChatThread {
  return {
    id: "thread-1",
    title: "New chat",
    createdAt: "2026-11-10T09:00:00.000Z",
    updatedAt: "2026-11-10T09:00:00.000Z",
    ...overrides,
  };
}

function message(overrides: Partial<JiraChatMessage> = {}): JiraChatMessage {
  return {
    id: "message-1",
    threadId: "thread-1",
    role: "assistant",
    content: "Done.",
    status: null,
    result: null,
    createdAt: "2026-11-10T09:00:01.000Z",
    ...overrides,
  };
}

function fakeApi(overrides: Partial<JiraApi> = {}): JiraApi {
  return {
    loadJiraWriting: vi.fn(async () => ({ draft: null, tickets: {} })),
    saveJiraWriting: vi.fn(async () => undefined),
    getJiraStatus: vi.fn(async () => status()),
    listJiraIssues: vi.fn(async () => [issue(), issue({ key: "AB-8", summary: "Second" })]),
    getJiraIssue: vi.fn(async (key: string) => detail({ key })),
    getSavedJiraIssues: vi.fn(async () => null),
    getSavedJiraIssue: vi.fn(async () => null),
    listJiraIssueTypes: vi.fn(async () => ["Bug", "Task"]),
    createJiraIssue: vi.fn(async () => detail({ key: "AB-9", summary: "Created" })),
    updateJiraIssue: vi.fn(async (key: string) => detail({ key, summary: "Edited" })),
    transitionJiraIssue: vi.fn(async (key: string) =>
      detail({ key, status: "In Progress", statusCategory: "indeterminate" }),
    ),
    addJiraComment: vi.fn(async (key: string) =>
      detail({ key, comments: [{ id: "c1", author: "Me", created: "now", body: "Verified" }] }),
    ),
    assignJiraIssueToMe: vi.fn(async (key: string) => detail({ key, assignee: "Sam Dev" })),
    downloadJiraAttachments: vi.fn(async (key: string, attachmentIds: string[] | null) => ({
      folder: `C:\\Downloads\\Work Notes\\${key}`,
      files: ATTACHMENTS.filter((item) => !attachmentIds || attachmentIds.includes(item.id)).map((item) => ({
        id: item.id,
        filename: item.filename,
        path: `C:\\Downloads\\Work Notes\\${key}\\${item.filename}`,
      })),
    })),
    revealJiraAttachment: vi.fn(async () => undefined),
    getJiraAttachmentThumbnail: vi.fn(async (attachmentId: string) => `blob:thumbnail-${attachmentId}`),
    readJiraAttachmentMedia: vi.fn(async (_key: string, attachmentId: string) => ({
      url: `blob:image-${attachmentId}`,
      kind: "image" as const,
    })),
    listJiraChatThreads: vi.fn(async () => [thread()]),
    createJiraChatThread: vi.fn(async () => thread({ id: "thread-new" })),
    deleteJiraChatThread: vi.fn(async () => undefined),
    listJiraPendingProposals: vi.fn(async () => []),
    listJiraChatMessages: vi.fn(async () => [message()]),
    sendJiraChatMessage: vi.fn(async () => [message({ role: "user", content: "hi" }), message()]),
    resolveJiraChatProposal: vi.fn(async () => [message({ role: "proposal", status: "approved" })]),
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("createJiraStore", () => {
  it("loads pending proposals across threads and opens the exact review", async () => {
    const proposal = message({ id: "p2", threadId: "thread-2", role: "proposal", status: "pending" });
    const api = fakeApi({ listJiraPendingProposals: vi.fn(async () => [proposal]),
      listJiraChatMessages: vi.fn(async () => [proposal]) });
    const store = createJiraStore(api);
    await store.open();
    expect(get(store.pendingProposals)).toEqual([proposal]);
    await store.openProposal("p2");
    expect(get(store.activeThreadId)).toBe("thread-2");
    expect(get(store.focusedProposalId)).toBe("p2");
    expect(api.resolveJiraChatProposal).not.toHaveBeenCalled();
    vi.mocked(api.listJiraPendingProposals).mockResolvedValue([]);
    await store.handleThreadUpdated("another-thread");
    expect(get(store.pendingProposals)).toEqual([]);
  });

  it("retains the last queue on failure and ignores an older refresh", async () => {
    const proposal = message({ role: "proposal", status: "pending" });
    const api = fakeApi({ listJiraPendingProposals: vi.fn(async () => [proposal]) });
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.listJiraPendingProposals).mockRejectedValueOnce(new Error("Queue unavailable"));
    await store.refreshPendingProposals();
    expect(get(store.pendingProposals)).toEqual([proposal]);
    expect(get(store.pendingProposalsError)).toBe("Queue unavailable");
    const older = deferred<JiraChatMessage[]>();
    vi.mocked(api.listJiraPendingProposals).mockReturnValueOnce(older.promise).mockResolvedValueOnce([]);
    const first = store.refreshPendingProposals();
    await store.refreshPendingProposals();
    older.resolve([proposal]);
    await first;
    expect(get(store.pendingProposals)).toEqual([]);
    expect(get(store.pendingProposalsError)).toBeNull();
  });

  it("keeps the latest queue selection when conversation loads finish out of order", async () => {
    const a = message({ id: "a", role: "proposal", status: "pending" });
    const b = message({ id: "b", role: "proposal", status: "pending" });
    const older = deferred<JiraChatMessage[]>();
    const api = fakeApi({ listJiraPendingProposals: vi.fn(async () => [a, b]),
      listJiraChatMessages: vi.fn().mockReturnValueOnce(older.promise).mockResolvedValue([a, b]) });
    const store = createJiraStore(api);
    await store.open();
    const first = store.openProposal("a");
    await store.openProposal("b");
    older.resolve([a]);
    await first;
    expect(get(store.focusedProposalId)).toBe("b");
    expect(get(store.messages)).toEqual([a, b]);
    await store.sendChat("Continue");
    expect(get(store.focusedProposalId)).toBeNull();
  });
  it("drops undo history when restoring a different Jira workspace", async () => {
    let setup = status();
    const store = createJiraStore(fakeApi({ getJiraStatus: vi.fn(async () => setup) }));
    await store.open();
    await store.startDraft({ summary: "Original project" });
    await store.applyViewAction({ kind: "setDraft", summary: "Agent revision", description: null, issueType: null });
    expect(get(store.draftBeforeAgent)).not.toBeNull();
    setup = status({ projectKey: "OTHER" });
    await store.open();
    store.undoDraftRewrite();
    expect(get(store.draft)).toBeNull();
    expect(get(store.draftBeforeAgent)).toBeNull();
  });

  it("keeps saves ordered and restores the correct writing after changing projects", async () => {
    let setup = status();
    const disk = new Map<string, JiraWriting>();
    const slow = deferred<void>();
    let firstSave = true;
    const api = fakeApi({
      getJiraStatus: vi.fn(async () => setup),
      loadJiraWriting: vi.fn(async (scope) => structuredClone(disk.get(scope) ?? { draft: null, tickets: {} })),
      saveJiraWriting: vi.fn(async (scope, writing) => {
        if (firstSave) { firstSave = false; await slow.promise; }
        disk.set(scope, structuredClone(writing));
      }),
    });
    const store = createJiraStore(api);
    await store.open();
    await store.startDraft({ summary: "First revision" });
    store.editDraft({ issueType: "Bug", summary: "Latest revision", description: "Persist last" });
    expect(api.saveJiraWriting).toHaveBeenCalledTimes(1);
    slow.resolve();
    await vi.waitFor(() => expect(get(store.writingSaving)).toBe(false));
    setup = status({ projectKey: "OTHER" });
    await store.open();
    expect(get(store.draft)).toBeNull();
    await store.startDraft({ summary: "Other project" });
    setup = status();
    await store.open();
    expect(get(store.draft)?.summary).toBe("Latest revision");
  });

  it("retains failed submissions and clears successfully submitted writing on disk", async () => {
    let saved: JiraWriting = { draft: null, tickets: {} };
    const api = fakeApi({
      saveJiraWriting: vi.fn(async (_scope, writing) => { saved = structuredClone(writing); }),
      addJiraComment: vi.fn().mockRejectedValueOnce(new Error("Offline")).mockResolvedValue(detail()),
    });
    const store = createJiraStore(api);
    await store.open();
    store.rememberWriting("AB-7", { editing: true, summary: "Changed", description: "Edited text", comment: "Hello" });
    await store.commentOnSelected("Hello");
    await vi.waitFor(() => expect(saved.tickets["AB-7"].comment).toBe("Hello"));
    await store.commentOnSelected("Hello");
    await vi.waitFor(() => expect(saved.tickets["AB-7"].comment).toBe(""));
    await store.saveSelectedEdit({ summary: "Changed", description: "Edited text" });
    await vi.waitFor(() => expect(saved.tickets["AB-7"]).toBeUndefined());
  });

  it("restores unfinished writing after recreating the store and removes discarded drafts", async () => {
    let saved: JiraWriting = { draft: null, tickets: {} };
    const api = fakeApi({
      loadJiraWriting: vi.fn(async () => structuredClone(saved)),
      saveJiraWriting: vi.fn(async (_scope, writing) => { saved = structuredClone(writing); }),
    });
    const first = createJiraStore(api);
    await first.open();
    await first.startDraft({ summary: "Restart test", description: "Keep this", noteId: "note-1" });
    first.rememberWriting("AB-7", { editing: true, summary: "Edited", description: "Unsent edit", comment: "Unsent comment" });
    await vi.waitFor(() => expect(saved.tickets["AB-7"]?.comment).toBe("Unsent comment"));
    const restarted = createJiraStore(api);
    await restarted.open();
    expect(get(restarted.draft)?.summary).toBe("Restart test");
    expect(get(restarted.draftVisible)).toBe(false);
    expect(get(restarted.selectedWriting)?.description).toBe("Unsent edit");
    restarted.cancelDraft();
    await vi.waitFor(() => expect(saved.draft).toBeNull());
  });

  it("reports failed saves, retains writing, and retries the latest revision", async () => {
    const save = vi.fn().mockRejectedValue(new Error("Disk full"));
    const store = createJiraStore(fakeApi({ saveJiraWriting: save }));
    await store.open();
    await store.startDraft({ summary: "Still here" });
    await vi.waitFor(() => expect(get(store.writingError)).toContain("Disk full"));
    expect(get(store.draft)?.summary).toBe("Still here");
    save.mockResolvedValue(undefined);
    await store.retryWriting();
    expect(get(store.writingError)).toBeNull();
    expect(save.mock.lastCall?.[1].draft.summary).toBe("Still here");
  });

  it("does not overwrite stored writing when loading fails", async () => {
    const api = fakeApi({ loadJiraWriting: vi.fn().mockRejectedValue(new Error("Unreadable")) });
    const store = createJiraStore(api);
    await store.open();
    expect(await store.startDraft()).toBe(false);
    expect(api.saveJiraWriting).not.toHaveBeenCalled();
    expect(get(store.writingError)).toContain("Unreadable");
    expect(get(store.writingReady)).toBe(false);
    expect(api.listJiraIssues).not.toHaveBeenCalled();
  });

  it("does not apply a running agent's rewrite to a replacement draft", async () => {
    const pending = deferred<JiraChatMessage[]>();
    const api = fakeApi({ sendJiraChatMessage: vi.fn(() => pending.promise) });
    const store = createJiraStore(api);
    await store.open();
    await store.startDraft({ summary: "First draft" });
    const sending = store.sendChat("Improve it");
    await vi.waitFor(() => expect(api.sendJiraChatMessage).toHaveBeenCalled());
    store.cancelDraft();
    await store.startDraft({ summary: "Replacement draft" });
    await store.applyViewAction({ kind: "setDraft", summary: "Late rewrite", description: null, issueType: null });
    expect(get(store.draft)?.summary).toBe("Replacement draft");
    pending.resolve([]);
    await sending;
  });

  it("does not replace unfinished writing when another note requests a ticket", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.startDraft({ summary: "My unfinished ticket", description: "Evidence", noteId: "note-old" });
    store.hideDraft();
    await store.draftFromNote({ id: "note-new", title: "Another note", text: "Other evidence" });
    expect(get(store.draft)).toMatchObject({ summary: "My unfinished ticket", noteId: "note-old" });
    expect(get(store.draftVisible)).toBe(true);
    expect(get(store.error)).toContain("Finish or discard");
    expect(api.sendJiraChatMessage).not.toHaveBeenCalled();
  });

  describe("attachments", () => {
    async function openWithAttachments(overrides: Partial<JiraApi> = {}) {
      const api = fakeApi({
        getJiraIssue: vi.fn(async (key: string) => detail({ key, attachments: ATTACHMENTS })),
        ...overrides,
      });
      const store = createJiraStore(api);
      await store.open();
      return { api, store };
    }

    it("loads a thumbnail for each attachment of the open ticket that has one", async () => {
      const { api, store } = await openWithAttachments();

      await vi.waitFor(() =>
        expect(get(store.attachmentThumbnails)).toEqual({ "10002": "blob:thumbnail-10002" }),
      );
      expect(api.getJiraAttachmentThumbnail).toHaveBeenCalledTimes(1);
      expect(api.getJiraAttachmentThumbnail).toHaveBeenCalledWith("10002");
    });

    it("does not load a thumbnail again when the ticket is shown again", async () => {
      const { api, store } = await openWithAttachments();
      await vi.waitFor(() => expect(get(store.attachmentThumbnails)).toHaveProperty("10002"));

      await store.commentOnSelected("Verified");
      await store.selectIssue("AB-7");

      expect(api.getJiraAttachmentThumbnail).toHaveBeenCalledTimes(1);
    });

    it("leaves out a thumbnail that fails to load without reporting an error", async () => {
      const getJiraAttachmentThumbnail = vi.fn(async () => {
        throw new Error("Jira returned 404 for attachment/thumbnail/10002");
      });
      const { store } = await openWithAttachments({
        getJiraAttachmentThumbnail,
        addJiraComment: vi.fn(async (key: string) => detail({ key, attachments: ATTACHMENTS })),
      });
      await vi.waitFor(() => expect(getJiraAttachmentThumbnail).toHaveBeenCalledTimes(1));

      await store.commentOnSelected("Verified");

      expect(get(store.attachmentThumbnails)).toEqual({});
      expect(get(store.error)).toBeNull();
      // A failed thumbnail is not asked for again.
      expect(getJiraAttachmentThumbnail).toHaveBeenCalledTimes(1);
    });

    it("opens an attachment in the viewer after downloading it", async () => {
      const { api, store } = await openWithAttachments();

      await store.openAttachment("10002");

      expect(api.downloadJiraAttachments).toHaveBeenCalledWith("AB-7", ["10002"]);
      expect(api.readJiraAttachmentMedia).toHaveBeenCalledWith("AB-7", "10002");
      expect(get(store.attachmentViewer)).toEqual({
        attachmentId: "10002",
        filename: "photo.png",
        url: "blob:image-10002",
        kind: "image",
        path: "C:\\Downloads\\Work Notes\\AB-7\\photo.png",
      });
      expect(get(store.attachmentDownload)?.files.map((file) => file.id)).toEqual(["10002"]);
      expect(get(store.openingAttachmentId)).toBeNull();
    });

    it("opens a video in the viewer as a video", async () => {
      const { store } = await openWithAttachments({
        readJiraAttachmentMedia: vi.fn(async () => ({ url: "blob:video-10002", kind: "video" as const })),
      });

      await store.openAttachment("10002");

      expect(get(store.attachmentViewer)).toMatchObject({ url: "blob:video-10002", kind: "video" });
    });

    it("reports an attachment the viewer cannot show and opens nothing", async () => {
      const { store } = await openWithAttachments({
        readJiraAttachmentMedia: vi.fn(async () => {
          throw new Error("photo.png is too large to show in the app");
        }),
      });

      await store.openAttachment("10002");

      expect(get(store.attachmentViewer)).toBeNull();
      expect(get(store.error)).toBe("photo.png is too large to show in the app");
      expect(get(store.openingAttachmentId)).toBeNull();
    });

    it("does not read the image when its download fails", async () => {
      const { api, store } = await openWithAttachments({
        downloadJiraAttachments: vi.fn(async () => {
          throw new Error("could not reach Jira: timed out");
        }),
      });

      await store.openAttachment("10002");

      expect(api.readJiraAttachmentMedia).not.toHaveBeenCalled();
      expect(get(store.attachmentViewer)).toBeNull();
      expect(get(store.error)).toBe("could not reach Jira: timed out");
    });

    it("closes the viewer on request and when another ticket is opened", async () => {
      const { store } = await openWithAttachments();
      await store.openAttachment("10002");

      store.closeAttachmentViewer();
      expect(get(store.attachmentViewer)).toBeNull();

      await store.openAttachment("10002");
      await store.selectIssue("AB-8");
      expect(get(store.attachmentViewer)).toBeNull();
    });

    it("downloads one attachment, then the rest, and keeps both results for the open ticket", async () => {
      const { api, store } = await openWithAttachments();

      await store.downloadAttachments("AB-7", "10002");

      expect(api.downloadJiraAttachments).toHaveBeenCalledWith("AB-7", ["10002"]);
      expect(get(store.attachmentDownload)?.files.map((file) => file.id)).toEqual(["10002"]);

      await store.downloadAttachments("AB-7", "10001");

      expect(get(store.attachmentDownload)?.files.map((file) => file.id)).toEqual(["10002", "10001"]);
      expect(get(store.attachmentDownload)?.folder).toBe("C:\\Downloads\\Work Notes\\AB-7");
      expect(get(store.downloadingAttachments)).toBe(false);
    });

    it("downloads every attachment when no id is given", async () => {
      const { api, store } = await openWithAttachments();

      const files = await store.downloadAttachments("AB-7");

      expect(api.downloadJiraAttachments).toHaveBeenCalledWith("AB-7", null);
      expect(files?.map((file) => file.filename)).toEqual(["bench.log", "photo.png"]);
    });

    it("shows downloads only for the ticket they belong to", async () => {
      const { store } = await openWithAttachments();
      await store.downloadAttachments("AB-7");

      await store.selectIssue("AB-8");

      expect(get(store.attachmentDownload)).toBeNull();
    });

    it("reports a failed download and stops showing it as running", async () => {
      const { store } = await openWithAttachments({
        downloadJiraAttachments: vi.fn(async () => {
          throw new Error("attachment download failed: disk full");
        }),
      });

      const files = await store.downloadAttachments("AB-7");

      expect(files).toBeUndefined();
      expect(get(store.error)).toBe("attachment download failed: disk full");
      expect(get(store.downloadingAttachments)).toBe(false);
      expect(get(store.attachmentDownload)).toBeNull();
    });

    it("builds the ticket prompt with the downloaded attachment paths", async () => {
      const { api, store } = await openWithAttachments();
      const open = get(store.selectedIssue);

      const prompt = await store.ticketPrompt(open!);

      expect(api.downloadJiraAttachments).toHaveBeenCalledWith("AB-7", null);
      expect(prompt).toContain("- bench.log (2.0 KB, text/plain): C:\\Downloads\\Work Notes\\AB-7\\bench.log");
      expect(prompt).toContain("- photo.png: C:\\Downloads\\Work Notes\\AB-7\\photo.png");
    });

    it("still builds the prompt when the download fails, and says what is missing", async () => {
      const { store } = await openWithAttachments({
        downloadJiraAttachments: vi.fn(async () => {
          throw new Error("could not reach Jira: timed out");
        }),
      });

      const prompt = await store.ticketPrompt(get(store.selectedIssue)!);

      expect(prompt).toContain("The ticket has attachments that were not downloaded.");
      expect(get(store.error)).toBe("could not reach Jira: timed out");
    });

    it("builds the prompt without downloading when the ticket has no attachments", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();

      const prompt = await store.ticketPrompt(get(store.selectedIssue)!);

      expect(api.downloadJiraAttachments).not.toHaveBeenCalled();
      expect(prompt).not.toContain("## Attachments");
    });

    it("reveals a downloaded file and reports when that fails", async () => {
      const { api, store } = await openWithAttachments();

      await store.revealAttachment("C:\\Downloads\\Work Notes\\AB-7\\bench.log");

      expect(api.revealJiraAttachment).toHaveBeenCalledWith("C:\\Downloads\\Work Notes\\AB-7\\bench.log");

      vi.mocked(api.revealJiraAttachment).mockRejectedValueOnce(new Error("file is gone"));
      await store.revealAttachment("C:\\missing.log");

      expect(get(store.error)).toBe("file is gone");
    });
  });

  it("does not query tickets until Jira is configured", async () => {
    const api = fakeApi({
      getJiraStatus: vi.fn(async () =>
        status({ configured: false, missing: ["Set the Jira site URL in Settings."], filters: [] }),
      ),
    });
    const store = createJiraStore(api);

    await store.open();

    expect(api.listJiraIssues).not.toHaveBeenCalled();
    expect(api.listJiraChatThreads).not.toHaveBeenCalled();
    expect(get(store.status)?.configured).toBe(false);
    expect(get(store.issues)).toEqual([]);
  });

  it("opens on the first filter, loads its tickets, and selects the first one", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);

    await store.open();

    expect(api.listJiraIssues).toHaveBeenCalledWith("assignee = currentUser()");
    expect(get(store.activeFilter)?.name).toBe("My open tickets");
    expect(get(store.issues).map((item) => item.key)).toEqual(["AB-7", "AB-8"]);
    expect(get(store.selectedIssue)?.key).toBe("AB-7");
    expect(get(store.threads)).toHaveLength(1);
    expect(get(store.loadingIssues)).toBe(false);
  });

  it("keeps the active filter and selection when reopened", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.selectFilter(status().filters[1]);
    await store.selectIssue("AB-8");

    await store.open();

    expect(api.listJiraIssues).toHaveBeenLastCalledWith("project = AB");
    expect(get(store.activeFilter)?.name).toBe("All open");
    expect(get(store.selectedIssue)?.key).toBe("AB-8");
  });

  it("clears the selection when the selected ticket leaves the filter", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.listJiraIssues).mockResolvedValueOnce([]);

    await store.selectFilter(status().filters[1]);

    expect(get(store.issues)).toEqual([]);
    expect(get(store.selectedIssue)).toBeNull();
  });

  it("reports a ticket load failure and stops loading", async () => {
    const api = fakeApi({
      listJiraIssues: vi.fn(async () => {
        throw { code: "jira_error", message: "Jira rejected the credentials" };
      }),
    });
    const store = createJiraStore(api);

    await store.open();

    expect(get(store.error)).toBe("Jira rejected the credentials");
    expect(get(store.loadingIssues)).toBe(false);
  });

  it("ignores a ticket response that arrives after a newer selection", async () => {
    const slow = deferred<JiraIssueDetail>();
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.getJiraIssue)
      .mockReturnValueOnce(slow.promise)
      .mockResolvedValueOnce(detail({ key: "AB-8" }));

    const first = store.selectIssue("AB-7");
    await store.selectIssue("AB-8");
    slow.resolve(detail({ key: "AB-7" }));
    await first;

    expect(get(store.selectedIssue)?.key).toBe("AB-8");
  });

  it("creates a ticket, closes the draft, and selects the new ticket", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.startDraft({ summary: "From note", noteId: "note-1" });
    expect(get(store.draft)).toEqual({
      issueType: "Bug",
      summary: "From note",
      description: "",
      noteId: "note-1",
    });
    vi.mocked(api.listJiraIssues).mockResolvedValueOnce([issue({ key: "AB-9", summary: "Created" })]);

    const key = await store.submitDraft({
      issueType: "Bug",
      summary: "From note",
      description: "Body",
      noteId: "note-1",
    });

    expect(key).toBe("AB-9");
    expect(api.createJiraIssue).toHaveBeenCalledWith({
      issueType: "Bug",
      summary: "From note",
      description: "Body",
      noteId: "note-1",
    });
    expect(get(store.draft)).toBeNull();
    expect(get(store.selectedIssue)?.key).toBe("AB-9");
    expect(get(store.issues).map((item) => item.key)).toEqual(["AB-9"]);
  });

  it("keeps showing the new ticket when the active filter does not list it", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.startDraft({ summary: "From note" });
    vi.mocked(api.getJiraIssue).mockClear();

    await store.submitDraft({ issueType: "Bug", summary: "From note", description: "" });

    expect(get(store.selectedIssue)?.key).toBe("AB-9");
    expect(get(store.issues).map((item) => item.key)).toEqual(["AB-7", "AB-8"]);
    expect(api.getJiraIssue).not.toHaveBeenCalled();
  });

  it("keeps the draft open when ticket creation fails", async () => {
    const api = fakeApi({
      createJiraIssue: vi.fn(async () => {
        throw new Error("summary is required");
      }),
    });
    const store = createJiraStore(api);
    await store.open();
    await store.startDraft();

    const key = await store.submitDraft({ issueType: "Task", summary: "", description: "" });

    expect(key).toBeUndefined();
    expect(get(store.draft)).not.toBeNull();
    expect(get(store.error)).toBe("summary is required");
    expect(get(store.savingIssue)).toBe(false);
  });

  it("falls back to default issue types when Jira cannot list them", async () => {
    const api = fakeApi({
      listJiraIssueTypes: vi.fn(async () => {
        throw new Error("403");
      }),
    });
    const store = createJiraStore(api);
    await store.open();

    await store.startDraft();

    expect(get(store.issueTypes)).toEqual(["Task", "Bug", "Story"]);
    expect(get(store.draft)?.issueType).toBe("Task");
    expect(get(store.error)).toBeNull();
  });

  it("applies a transition and updates the ticket row in place", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();

    await store.transitionSelected("11");

    expect(api.transitionJiraIssue).toHaveBeenCalledWith("AB-7", "11");
    expect(get(store.selectedIssue)?.status).toBe("In Progress");
    expect(get(store.issues)[0].status).toBe("In Progress");
    expect(get(store.issues)[0].statusCategory).toBe("indeterminate");
    expect(api.listJiraIssues).toHaveBeenCalledTimes(1);
  });

  it("adds comments, saves edits, and assigns through the selected ticket", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();

    expect(await store.commentOnSelected("Verified")).toBe(true);
    expect(api.addJiraComment).toHaveBeenCalledWith("AB-7", "Verified");
    expect(get(store.selectedIssue)?.comments).toHaveLength(1);

    expect(await store.saveSelectedEdit({ summary: "Edited" })).toBe(true);
    expect(api.updateJiraIssue).toHaveBeenCalledWith("AB-7", { summary: "Edited" });
    expect(get(store.issues)[0].summary).toBe("Edited");

    expect(await store.assignSelectedToMe()).toBe(true);
    expect(get(store.issues)[0].assignee).toBe("Sam Dev");
  });

  it("reports a failed ticket change and leaves the ticket unchanged", async () => {
    const api = fakeApi({
      addJiraComment: vi.fn(async () => {
        throw new Error("comment text is required");
      }),
    });
    const store = createJiraStore(api);
    await store.open();

    expect(await store.commentOnSelected(" ")).toBe(false);

    expect(get(store.error)).toBe("comment text is required");
    expect(get(store.selectedIssue)?.comments).toEqual([]);
    expect(get(store.savingIssue)).toBe(false);
  });

  it("starts a thread on the first chat message and sends the open ticket as context", async () => {
    const api = fakeApi({ listJiraChatThreads: vi.fn(async () => []) });
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.listJiraChatThreads).mockResolvedValueOnce([thread({ id: "thread-new", title: "hi" })]);

    await store.sendChat("  hi  ");

    expect(api.createJiraChatThread).toHaveBeenCalledTimes(1);
    expect(api.sendJiraChatMessage).toHaveBeenCalledWith("thread-new", "hi", "AB-7", null);
    expect(get(store.activeThreadId)).toBe("thread-new");
    expect(get(store.messages).map((item) => item.role)).toEqual(["user", "assistant"]);
    expect(get(store.threads)[0].title).toBe("hi");
    expect(get(store.agentBusy)).toBe(false);
  });

  it("shows the user message immediately while the agent works", async () => {
    const pending = deferred<JiraChatMessage[]>();
    const api = fakeApi({ sendJiraChatMessage: vi.fn(() => pending.promise) });
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");

    const sending = store.sendChat("move AB-7 to done");
    await vi.waitFor(() => expect(get(store.agentBusy)).toBe(true));

    expect(get(store.messages).at(-1)).toMatchObject({ role: "user", content: "move AB-7 to done" });
    pending.resolve([message({ role: "user", content: "move AB-7 to done" }), message()]);
    await sending;
    expect(get(store.messages)).toHaveLength(2);
    expect(get(store.agentBusy)).toBe(false);
  });

  it("ignores blank messages and a second message while the agent is busy", async () => {
    const pending = deferred<JiraChatMessage[]>();
    const api = fakeApi({ sendJiraChatMessage: vi.fn(() => pending.promise) });
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");

    await store.sendChat("   ");
    const sending = store.sendChat("first");
    await vi.waitFor(() => expect(get(store.agentBusy)).toBe(true));
    await store.sendChat("second");
    pending.resolve([message()]);
    await sending;

    expect(api.sendJiraChatMessage).toHaveBeenCalledTimes(1);
  });

  it("reports an agent failure and reloads what was stored", async () => {
    const api = fakeApi({
      sendJiraChatMessage: vi.fn(async () => {
        throw { code: "agent_error", message: "codex command was not found: codex.cmd" };
      }),
    });
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");
    vi.mocked(api.listJiraChatMessages).mockResolvedValueOnce([message({ role: "user", content: "hi" })]);

    await store.sendChat("hi");

    expect(get(store.chatError)).toBe("codex command was not found: codex.cmd");
    expect(get(store.messages)).toEqual([message({ role: "user", content: "hi" })]);
    expect(get(store.agentBusy)).toBe(false);
  });

  it("refreshes tickets after an approved change but not after a rejection", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");
    expect(api.listJiraIssues).toHaveBeenCalledTimes(1);
    expect(api.getJiraIssue).toHaveBeenCalledTimes(1);

    await store.resolveProposal("message-9", false);

    expect(api.resolveJiraChatProposal).toHaveBeenCalledWith("message-9", false);
    expect(api.listJiraIssues).toHaveBeenCalledTimes(1);

    await store.resolveProposal("message-9", true);

    expect(api.resolveJiraChatProposal).toHaveBeenLastCalledWith("message-9", true);
    expect(get(store.messages)[0].status).toBe("approved");
    expect(api.listJiraIssues).toHaveBeenCalledTimes(2);
    expect(api.getJiraIssue).toHaveBeenCalledTimes(2);
    expect(get(store.busyProposalId)).toBeNull();
  });

  it("reloads the active thread when the backend reports progress for it", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");
    vi.mocked(api.listJiraChatMessages).mockClear();

    await store.handleThreadUpdated("other-thread");
    expect(api.listJiraChatMessages).not.toHaveBeenCalled();

    await store.handleThreadUpdated("thread-1");
    expect(api.listJiraChatMessages).toHaveBeenCalledWith("thread-1");
  });

  it("deletes a thread and leaves the conversation empty when it was open", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");
    vi.mocked(api.listJiraChatThreads).mockResolvedValueOnce([]);

    await store.deleteThread("thread-1");

    expect(api.deleteJiraChatThread).toHaveBeenCalledWith("thread-1");
    expect(get(store.activeThreadId)).toBeNull();
    expect(get(store.messages)).toEqual([]);
    expect(get(store.threads)).toEqual([]);
  });

  it("starts a blank conversation without creating a thread", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    await store.openThread("thread-1");

    store.newThread();

    expect(get(store.activeThreadId)).toBeNull();
    expect(get(store.messages)).toEqual([]);
    expect(api.createJiraChatThread).not.toHaveBeenCalled();
  });

  describe("ticket drafts and the agent", () => {
    const NOTE = { id: "note-1", title: "printer thing", text: "it jams after reboot" };
    const NOTE_DRAFT = {
      issueType: "Bug",
      summary: "printer thing",
      description: "it jams after reboot",
      noteId: "note-1",
    };

    it("prefills a draft from a note and asks the agent to write the ticket in a new conversation", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();
      await store.openThread("thread-1");

      await store.draftFromNote(NOTE);

      expect(get(store.draft)).toEqual(NOTE_DRAFT);
      expect(get(store.draftAgentOpen)).toBe(true);
      expect(get(store.activeThreadId)).toBe("thread-new");
      expect(api.sendJiraChatMessage).toHaveBeenCalledTimes(1);
      const [threadId, text, focusIssueKey, draft] = vi.mocked(api.sendJiraChatMessage).mock.calls[0];
      expect(threadId).toBe("thread-new");
      expect(text.split("\n")[0]).toBe("Draft a ticket from my note: printer thing");
      expect(focusIssueKey).toBeNull();
      expect(draft).toEqual({ ...NOTE_DRAFT, issueTypes: ["Bug", "Task"] });
    });

    it("keeps the prefilled draft when the agent cannot write the ticket", async () => {
      const api = fakeApi({
        sendJiraChatMessage: vi.fn(async () => {
          throw new Error("codex command was not found: codex.cmd");
        }),
      });
      const store = createJiraStore(api);
      await store.open();

      await store.draftFromNote(NOTE);

      expect(get(store.draft)).toEqual(NOTE_DRAFT);
      expect(get(store.chatError)).toBe("codex command was not found: codex.cmd");
    });

    it("leaves a running conversation alone and only prefills the draft", async () => {
      const pending = deferred<JiraChatMessage[]>();
      const api = fakeApi({ sendJiraChatMessage: vi.fn(() => pending.promise) });
      const store = createJiraStore(api);
      await store.open();
      await store.openThread("thread-1");
      const sending = store.sendChat("what is open?");
      await vi.waitFor(() => expect(get(store.agentBusy)).toBe(true));

      await store.draftFromNote(NOTE);

      expect(get(store.draft)).toEqual(NOTE_DRAFT);
      expect(get(store.activeThreadId)).toBe("thread-1");
      expect(api.sendJiraChatMessage).toHaveBeenCalledTimes(1);
      pending.resolve([message()]);
      await sending;
    });

    it("does not call the agent when Jira is not set up", async () => {
      const api = fakeApi({ getJiraStatus: vi.fn(async () => status({ configured: false, filters: [] })) });
      const store = createJiraStore(api);
      await store.open();

      await store.draftFromNote(NOTE);

      expect(get(store.draftAgentOpen)).toBe(false);
      expect(api.sendJiraChatMessage).not.toHaveBeenCalled();
    });

    it("opens a blank draft without asking for the agent, and lets the agent panel be dismissed", async () => {
      const store = createJiraStore(fakeApi());
      await store.open();
      await store.draftFromNote(NOTE);

      store.closeDraftAgent();
      expect(get(store.draftAgentOpen)).toBe(false);

      await store.draftFromNote(NOTE);
      await store.startDraft();
      expect(get(store.draftAgentOpen)).toBe(false);
    });

    it("sends the form as it stands, with the user's edits, instead of the ticket behind it", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();
      await store.startDraft({ summary: "From note", noteId: "note-1" });

      store.editDraft({ issueType: "Task", summary: "Edited", description: "Body" });
      await store.sendChat("tighten it");

      expect(get(store.draft)).toEqual({ issueType: "Task", summary: "Edited", description: "Body", noteId: "note-1" });
      expect(api.sendJiraChatMessage).toHaveBeenCalledWith("thread-new", "tighten it", null, {
        issueType: "Task",
        summary: "Edited",
        description: "Body",
        issueTypes: ["Bug", "Task"],
        noteId: "note-1",
      });
    });

    it("ignores form edits when no draft is open", async () => {
      const store = createJiraStore(fakeApi());
      await store.open();

      store.editDraft({ issueType: "Task", summary: "Edited", description: "" });

      expect(get(store.draft)).toBeNull();
    });

    it("applies the agent's rewrite to the fields it names", async () => {
      const store = createJiraStore(fakeApi());
      await store.open();
      await store.startDraft({ summary: "printer thing", description: "it jams", noteId: "note-1" });

      await store.applyViewAction({
        kind: "setDraft",
        issueType: "Task",
        summary: "Printer jams after reboot",
        description: null,
      });

      expect(get(store.draft)).toEqual({
        issueType: "Task",
        summary: "Printer jams after reboot",
        description: "it jams",
        noteId: "note-1",
      });
    });

    it("keeps the issue type when the agent names one the form does not offer", async () => {
      const store = createJiraStore(fakeApi());
      await store.open();
      await store.startDraft();

      await store.applyViewAction({ kind: "setDraft", issueType: "Epic", summary: null, description: "Body" });

      expect(get(store.draft)).toMatchObject({ issueType: "Bug", description: "Body" });
    });

    it("ignores the agent's rewrite when the draft was closed meanwhile", async () => {
      const store = createJiraStore(fakeApi());
      await store.open();

      await store.applyViewAction({ kind: "setDraft", issueType: null, summary: "Late", description: null });

      expect(get(store.draft)).toBeNull();
    });
  });

  describe("agent view actions", () => {
    it("shows the agent's search as a temporary filter and loads its tickets", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();

      await store.applyViewAction({ kind: "showIssues", title: "Important today", jql: "priority = High" });

      const filter = { name: "Important today", jql: "priority = High" };
      expect(get(store.agentFilter)).toEqual(filter);
      expect(get(store.activeFilter)).toEqual(filter);
      expect(api.listJiraIssues).toHaveBeenLastCalledWith("priority = High");
    });

    it("keeps the agent's filter when the view is reopened", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();
      await store.applyViewAction({ kind: "showIssues", title: "Important today", jql: "priority = High" });

      await store.open();

      expect(get(store.activeFilter)?.name).toBe("Important today");
      expect(api.listJiraIssues).toHaveBeenLastCalledWith("priority = High");
    });

    it("goes back to the first saved filter when the agent's filter is cleared", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();
      await store.applyViewAction({ kind: "showIssues", title: "Important today", jql: "priority = High" });

      await store.clearAgentFilter();

      expect(get(store.agentFilter)).toBeNull();
      expect(get(store.activeFilter)?.name).toBe("My open tickets");
      expect(api.listJiraIssues).toHaveBeenLastCalledWith("assignee = currentUser()");
    });

    it("leaves the active filter alone when a saved filter was chosen after the agent's", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();
      await store.applyViewAction({ kind: "showIssues", title: "Important today", jql: "priority = High" });
      await store.selectFilter({ name: "All open", jql: "project = AB" });
      const searches = vi.mocked(api.listJiraIssues).mock.calls.length;

      await store.clearAgentFilter();

      expect(get(store.agentFilter)).toBeNull();
      expect(get(store.activeFilter)?.name).toBe("All open");
      expect(api.listJiraIssues).toHaveBeenCalledTimes(searches);
    });

    it("opens the ticket the agent names and keeps the new-ticket draft for later", async () => {
      const api = fakeApi();
      const store = createJiraStore(api);
      await store.open();
      await store.startDraft();

      await store.applyViewAction({ kind: "openIssue", issueKey: "AB-8" });

      expect(get(store.draftVisible)).toBe(false);
      expect(get(store.draft)).not.toBeNull();
      expect(get(store.selectedIssue)?.key).toBe("AB-8");
    });
  });
});

describe("saved copies", () => {
  const SAVED_AT = "2026-11-16T09:30:00.000Z";

  it("shows the saved ticket list and ticket at once, then Jira's current ones", async () => {
    const list = deferred<JiraIssueSummary[]>();
    const ticket = deferred<JiraIssueDetail>();
    const api = fakeApi({
      listJiraIssues: vi.fn(() => list.promise),
      getJiraIssue: vi.fn(() => ticket.promise),
      getSavedJiraIssues: vi.fn(async () => ({ value: [issue({ key: "AB-5", summary: "Saved" })], savedAt: SAVED_AT })),
      getSavedJiraIssue: vi.fn(async (key: string) => ({ value: detail({ key, summary: "Saved ticket" }), savedAt: SAVED_AT })),
    });
    const store = createJiraStore(api);

    const opening = store.open();
    await vi.waitFor(() => expect(get(store.selectedIssue)?.summary).toBe("Saved ticket"));
    expect(get(store.issues).map((item) => item.key)).toEqual(["AB-5"]);
    expect(get(store.issuesSavedAt)).toBe(SAVED_AT);
    expect(get(store.issueSavedAt)).toBe(SAVED_AT);
    expect(api.getSavedJiraIssues).toHaveBeenCalledWith("assignee = currentUser()");

    list.resolve([issue({ key: "AB-5", summary: "Current" }), issue({ key: "AB-6" })]);
    ticket.resolve(detail({ key: "AB-5", summary: "Current ticket" }));
    await opening;

    expect(get(store.issues).map((item) => item.summary)).toEqual(["Current", "Printer jams on reboot"]);
    expect(get(store.issuesSavedAt)).toBeNull();
    expect(get(store.selectedIssue)?.summary).toBe("Current ticket");
    expect(get(store.issueSavedAt)).toBeNull();
    expect(api.getJiraIssue).toHaveBeenCalledTimes(1);
  });

  it("refuses changes to a saved copy until Jira sends the current ticket", async () => {
    const ticket = deferred<JiraIssueDetail>();
    const api = fakeApi({
      getSavedJiraIssue: vi.fn(async (key: string) => ({ value: detail({ key }), savedAt: SAVED_AT })),
    });
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.getJiraIssue).mockReturnValueOnce(ticket.promise);

    const selecting = store.selectIssue("AB-8");
    await vi.waitFor(() => expect(get(store.issueSavedAt)).toBe(SAVED_AT));

    expect(await store.transitionSelected("11")).toBe(false);
    expect(await store.commentOnSelected("Verified")).toBe(false);
    expect(api.transitionJiraIssue).not.toHaveBeenCalled();
    expect(api.addJiraComment).not.toHaveBeenCalled();
    expect(get(store.error)).toMatch(/saved copy/i);

    ticket.resolve(detail({ key: "AB-8" }));
    await selecting;
    expect(await store.transitionSelected("11")).toBe(true);
    expect(api.transitionJiraIssue).toHaveBeenCalledWith("AB-8", "11");
  });

  it("keeps the saved copies on screen when Jira cannot be reached", async () => {
    const api = fakeApi({
      listJiraIssues: vi.fn(async () => { throw new Error("Jira is offline"); }),
      getJiraIssue: vi.fn(async () => { throw new Error("Jira is offline"); }),
      getSavedJiraIssues: vi.fn(async () => ({ value: [issue()], savedAt: SAVED_AT })),
      getSavedJiraIssue: vi.fn(async (key: string) => ({ value: detail({ key }), savedAt: SAVED_AT })),
    });
    const store = createJiraStore(api);

    await store.open();
    await vi.waitFor(() => expect(get(store.selectedIssue)?.key).toBe("AB-7"));

    expect(get(store.issues).map((item) => item.key)).toEqual(["AB-7"]);
    expect(get(store.issuesSavedAt)).toBe(SAVED_AT);
    expect(get(store.issueSavedAt)).toBe(SAVED_AT);
    expect(get(store.error)).toBe("Jira is offline");
  });

  it("never replaces what is shown with an older saved copy of the same list or ticket", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.getSavedJiraIssues).mockResolvedValue({ value: [issue({ summary: "Old" })], savedAt: SAVED_AT });
    vi.mocked(api.getSavedJiraIssue).mockResolvedValue({ value: detail({ summary: "Old" }), savedAt: SAVED_AT });
    const list = deferred<JiraIssueSummary[]>();
    const ticket = deferred<JiraIssueDetail>();
    vi.mocked(api.listJiraIssues).mockReturnValueOnce(list.promise);
    vi.mocked(api.getJiraIssue).mockReturnValueOnce(ticket.promise);

    const refreshing = store.refreshIssues();
    const reopening = store.selectIssue("AB-7");
    await Promise.resolve();
    await Promise.resolve();

    expect(get(store.issues)[0].summary).toBe("Printer jams on reboot");
    expect(get(store.selectedIssue)?.summary).toBe("Printer jams on reboot");
    expect(get(store.issuesSavedAt)).toBeNull();
    expect(get(store.issueSavedAt)).toBeNull();
    list.resolve([issue()]);
    ticket.resolve(detail());
    await Promise.all([refreshing, reopening]);
  });

  it("ignores a saved copy that arrives after Jira's answer", async () => {
    const savedList = deferred<{ value: JiraIssueSummary[]; savedAt: string } | null>();
    const savedTicket = deferred<{ value: JiraIssueDetail; savedAt: string } | null>();
    const api = fakeApi({
      getSavedJiraIssues: vi.fn(() => savedList.promise),
      getSavedJiraIssue: vi.fn(() => savedTicket.promise),
    });
    const store = createJiraStore(api);

    const opening = store.open();
    await vi.waitFor(() => expect(api.getSavedJiraIssues).toHaveBeenCalled());
    await vi.waitFor(() => expect(api.listJiraIssues).toHaveBeenCalled());
    await Promise.resolve();
    savedList.resolve({ value: [issue({ key: "AB-1", summary: "Old" })], savedAt: SAVED_AT });
    await vi.waitFor(() => expect(api.getSavedJiraIssue).toHaveBeenCalled());
    await Promise.resolve();
    savedTicket.resolve({ value: detail({ summary: "Old" }), savedAt: SAVED_AT });
    await opening;

    expect(get(store.issues).map((item) => item.key)).toEqual(["AB-7", "AB-8"]);
    expect(get(store.issuesSavedAt)).toBeNull();
    expect(get(store.selectedIssue)?.summary).toBe("Printer jams on reboot");
    expect(get(store.issueSavedAt)).toBeNull();
  });
});
describe("searching Jira beyond the loaded list", () => {
  it("lists Jira's text matches as a temporary filter and drops it again", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();

    await store.searchJira("printer jam");

    const search = get(store.searchFilter);
    expect(search).toEqual({ name: "Search: printer jam", jql: 'project = AB AND text ~ "printer jam" ORDER BY updated DESC' });
    expect(get(store.activeFilter)).toBe(search);
    expect(api.listJiraIssues).toHaveBeenLastCalledWith(search?.jql);

    await store.open();
    expect(get(store.activeFilter)).toBe(search);

    await store.clearSearchFilter();
    expect(get(store.searchFilter)).toBeNull();
    expect(get(store.activeFilter)?.name).toBe("My open tickets");
  });

  it("opens a ticket typed by key without changing the list", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.listJiraIssues).mockClear();

    await store.searchJira(" ab-42 ");

    expect(api.getJiraIssue).toHaveBeenLastCalledWith("AB-42");
    expect(get(store.selectedIssue)?.key).toBe("AB-42");
    expect(api.listJiraIssues).not.toHaveBeenCalled();
    expect(get(store.searchFilter)).toBeNull();
  });

  it("does nothing for a query with no searchable words", async () => {
    const api = fakeApi();
    const store = createJiraStore(api);
    await store.open();
    vi.mocked(api.listJiraIssues).mockClear();

    await store.searchJira('"" ()');

    expect(api.listJiraIssues).not.toHaveBeenCalled();
    expect(get(store.searchFilter)).toBeNull();
  });
});