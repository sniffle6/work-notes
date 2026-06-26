import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

import type {
  ActionItem,
  ActionReviewItem,
  AppSettings,
  FollowupItem,
  FollowupState,
  InboxFilters,
  NoteDetail,
  NoteListItem,
} from "$lib/types";
import { createWorkNotesStore } from "./inbox";
import { createInboxFilters, matchesNoteFilters } from "./filters";

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Pricing export mismatch",
    rawText: "Jordan said finance sees a different CSV total.",
    summary: "Finance needs the filtered query checked.",
    captureSource: "quick_capture",
    createdAt: "2026-05-20T13:00:00.000Z",
    updatedAt: "2026-05-20T13:00:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [
      {
        id: "tag-1",
        name: "Finance",
        kind: "topic",
        source: "ai",
        confidence: 0.92,
      },
    ],
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    ...overrides,
  };
}

describe("createInboxFilters", () => {
  it("creates empty filters that do not exclude notes", () => {
    const filters = createInboxFilters();

    expect(filters).toEqual({
      search: "",
      tagIds: [],
      parseStatuses: [],
      reviewStatuses: [],
    });
    expect(matchesNoteFilters(note(), filters)).toBe(true);
  });
});

describe("matchesNoteFilters", () => {
  it("matches notes by case-insensitive text across title, raw text, and summary", () => {
    const filters = createInboxFilters({ search: "FILTERED query" });

    expect(matchesNoteFilters(note(), filters)).toBe(true);
    expect(matchesNoteFilters(note({ summary: "No related wording." }), filters)).toBe(false);
  });

  it("requires at least one matching selected tag", () => {
    const filters = createInboxFilters({ tagIds: ["tag-2", "tag-3"] });

    expect(
      matchesNoteFilters(
        note({
          tags: [
            { id: "tag-2", name: "Maya", kind: "person", source: "user", confidence: 1 },
          ],
        }),
        filters,
      ),
    ).toBe(true);
    expect(matchesNoteFilters(note(), filters)).toBe(false);
  });

  it("matches notes by selected parse statuses", () => {
    const filters = createInboxFilters({ parseStatuses: ["failed", "queued"] });

    expect(matchesNoteFilters(note({ parseStatus: "failed" }), filters)).toBe(true);
    expect(matchesNoteFilters(note({ parseStatus: "parsed" }), filters)).toBe(false);
  });

  it("matches notes by selected review statuses", () => {
    const filters = createInboxFilters({ reviewStatuses: ["needs_review"] });

    expect(matchesNoteFilters(note({ reviewStatus: "needs_review" }), filters)).toBe(true);
    expect(matchesNoteFilters(note({ reviewStatus: "reviewed" }), filters)).toBe(false);
  });

  it("combines text, tag, parse status, and review status filters", () => {
    const filters = createInboxFilters({
      search: "pricing",
      tagIds: ["tag-1"],
      parseStatuses: ["parsed"],
      reviewStatuses: ["needs_review"],
    });

    expect(matchesNoteFilters(note(), filters)).toBe(true);
    expect(matchesNoteFilters(note({ parseStatus: "failed" }), filters)).toBe(false);
  });
});

describe("createWorkNotesStore", () => {
  it("reloads the inbox from the API when filters change", async () => {
    const api = testApi({
      listInbox: vi
        .fn()
        .mockResolvedValueOnce([note({ id: "before-filter" })])
        .mockResolvedValueOnce([note({ id: "after-filter", rawText: "needle" })]),
    });
    const store = createWorkNotesStore(api);

    await store.loadInbox();
    await store.updateFilters({ search: "needle" });

    expect(api.listInbox).toHaveBeenLastCalledWith(
      expect.objectContaining({ search: "needle" }),
    );
    expect(get(store.inbox).map((item) => item.id)).toEqual(["after-filter"]);
  });

  it("reveals an externally captured note by clearing filters and selecting it", async () => {
    const captured = note({
      id: "captured-note",
      rawText: "Robert asked about local AI feasibility.",
      parseStatus: "failed",
      reviewStatus: "none",
    });
    const api = testApi({
      listInbox: vi
        .fn()
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce([captured]),
      getNote: vi
        .fn()
        .mockResolvedValue({ ...captured, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.updateFilters({ search: "old filter" });
    await store.showCapturedNote("captured-note");

    expect(get(store.filters)).toEqual(createInboxFilters({ includeArchived: false }));
    expect(api.listInbox).toHaveBeenLastCalledWith(createInboxFilters({ includeArchived: false }));
    expect(get(store.inbox).map((item) => item.id)).toEqual(["captured-note"]);
    expect(get(store.selectedNote)?.id).toBe("captured-note");
  });

  it("retries the selected note with feedback and reloads it", async () => {
    const api = testApi();
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.retrySelectedParseWithFeedback("Tag this as research.");

    expect(api.retryParseWithFeedback).toHaveBeenCalledWith("note-1", "Tag this as research.");
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
  });

  it("saves cleaned edits and reloads the selected note", async () => {
    const api = testApi();
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.saveCleanedEdits({ title: "New title", summary: "New summary", cleanedText: "New body" });

    expect(api.updateNoteCleaned).toHaveBeenCalledWith("note-1", {
      title: "New title",
      summary: "New summary",
      cleanedText: "New body",
    });
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
  });

  it("archives the selected note and clears the selection", async () => {
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([]),
    });
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.deleteSelectedNote();

    expect(api.deleteNote).toHaveBeenCalledWith("note-1");
    expect(get(store.selectedNote)).toBeNull();
    expect(get(store.inbox)).toEqual([]);
  });

  it("loads archived notes in archive mode only", async () => {
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([
        note({ id: "active", isArchived: false }),
        note({ id: "archived", isArchived: true }),
      ]),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();

    expect(get(store.viewMode)).toBe("archive");
    expect(get(store.inbox).map((item) => item.id)).toEqual(["archived"]);
    expect(api.listInbox).toHaveBeenCalledWith(
      expect.objectContaining({ includeArchived: true }),
    );
  });

  it("enters today view with non-archived notes and suggested actions", async () => {
    const active = note({ id: "active", isArchived: false });
    const archived = note({ id: "archived", isArchived: true });
    const dueAction = reviewItem({ id: "due-action", noteId: "active", dueDate: "2026-05-22" });
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([active, archived]),
      listSuggestedActions: vi.fn().mockResolvedValue([dueAction]),
      getNote: vi.fn().mockResolvedValue({ ...active, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.updateFilters({
      search: "stale filter",
      tagIds: ["tag-stale"],
      parseStatuses: ["failed"],
      reviewStatuses: ["reviewed"],
    });
    await store.showToday();

    expect(get(store.viewMode)).toBe("today");
    expect(get(store.filters)).toEqual(createInboxFilters({ includeArchived: false }));
    expect(api.listInbox).toHaveBeenLastCalledWith(createInboxFilters({ includeArchived: false }));
    expect(get(store.inbox).map((item) => item.id)).toEqual(["active"]);
    expect(get(store.suggestedActions)).toEqual([dueAction]);
    expect(api.listSuggestedActions).toHaveBeenCalledTimes(1);
  });

  it("enters people view with non-archived notes, reset filters, and suggested actions", async () => {
    const active = note({ id: "active", isArchived: false });
    const archived = note({ id: "archived", isArchived: true });
    const personAction = reviewItem({ id: "person-action", noteId: "active", owner: "Maria" });
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([active, archived]),
      listSuggestedActions: vi.fn().mockResolvedValue([personAction]),
      getNote: vi.fn().mockResolvedValue({ ...active, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.updateFilters({
      search: "stale filter",
      tagIds: ["tag-stale"],
      parseStatuses: ["failed"],
      reviewStatuses: ["reviewed"],
    });
    await store.showPeople();

    expect(get(store.viewMode)).toBe("people");
    expect(get(store.filters)).toEqual(createInboxFilters({ includeArchived: false }));
    expect(api.listInbox).toHaveBeenLastCalledWith(
      createInboxFilters({ includeArchived: false, limit: 1000 }),
    );
    expect(get(store.inbox).map((item) => item.id)).toEqual(["active"]);
    expect(get(store.suggestedActions)).toEqual([personAction]);
    expect(api.listSuggestedActions).toHaveBeenCalledWith(500);
  });

  it("does not keep the people inbox limit when returning to normal inbox", async () => {
    const active = note({ id: "active", isArchived: false });
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([active]),
      getNote: vi.fn().mockResolvedValue({ ...active, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showPeople();
    await store.showInbox();

    expect(get(store.viewMode)).toBe("inbox");
    expect(get(store.filters)).toEqual(createInboxFilters({ includeArchived: false }));
    expect(api.listInbox).toHaveBeenLastCalledWith(createInboxFilters({ includeArchived: false }));
  });

  it("restores the selected archived note and returns to inbox mode", async () => {
    const archived = note({ id: "archived", isArchived: true });
    const restored = note({ id: "archived", isArchived: false });
    const api = testApi({
      listInbox: vi
        .fn()
        .mockResolvedValueOnce([archived])
        .mockResolvedValueOnce([restored]),
      getNote: vi
        .fn()
        .mockResolvedValueOnce({ ...archived, actionItems: [] })
        .mockResolvedValueOnce({ ...restored, actionItems: [] })
        .mockResolvedValue({ ...restored, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.selectNote("archived");
    await store.restoreSelectedNote();

    expect(get(store.viewMode)).toBe("inbox");
    expect(get(store.selectedNote)?.id).toBe("archived");
    expect(api.restoreNote).toHaveBeenCalledWith("archived");
  });

  it("permanently deletes selected archived note and stays in archive mode", async () => {
    const first = note({ id: "archived-1", isArchived: true });
    const second = note({ id: "archived-2", isArchived: true });
    const api = testApi({
      listInbox: vi
        .fn()
        .mockResolvedValueOnce([first, second])
        .mockResolvedValueOnce([second]),
      getNote: vi
        .fn()
        .mockResolvedValueOnce({ ...first, actionItems: [] })
        .mockResolvedValueOnce({ ...first, actionItems: [] })
        .mockResolvedValue({ ...second, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.selectNote("archived-1");
    await store.permanentlyDeleteSelectedNote();

    expect(get(store.viewMode)).toBe("archive");
    expect(get(store.selectedNote)?.id).toBe("archived-2");
    expect(api.permanentlyDeleteNote).toHaveBeenCalledWith("archived-1");
  });

  it("selects the next archived note after permanently deleting a middle item", async () => {
    const first = note({ id: "archived-1", isArchived: true });
    const second = note({ id: "archived-2", isArchived: true });
    const third = note({ id: "archived-3", isArchived: true });
    const api = testApi({
      listInbox: vi
        .fn()
        .mockResolvedValueOnce([first, second, third])
        .mockResolvedValueOnce([first, third]),
      getNote: vi.fn<(noteId: string) => Promise<NoteDetail>>(async (noteId) => {
        const details: Record<string, NoteDetail> = {
          "archived-1": { ...first, actionItems: [] },
          "archived-2": { ...second, actionItems: [] },
          "archived-3": { ...third, actionItems: [] },
        };

        return details[noteId];
      }),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.selectNote("archived-2");
    await store.permanentlyDeleteSelectedNote();

    expect(get(store.viewMode)).toBe("archive");
    expect(get(store.inbox).map((item) => item.id)).toEqual(["archived-1", "archived-3"]);
    expect(get(store.selectedNote)?.id).toBe("archived-3");
    expect(api.getNote).toHaveBeenLastCalledWith("archived-3");
    expect(api.permanentlyDeleteNote).toHaveBeenCalledWith("archived-2");
  });

  it("reloads archive and clears stale selection when archived note lookup is not found", async () => {
    const stale = note({ id: "archived-stale", isArchived: true });
    const next = note({ id: "archived-next", isArchived: true });
    const api = testApi({
      listInbox: vi
        .fn()
        .mockResolvedValueOnce([stale, next])
        .mockResolvedValueOnce([next]),
      getNote: vi
        .fn()
        .mockResolvedValueOnce({ ...stale, actionItems: [] })
        .mockRejectedValueOnce({ code: "not_found", message: "note not found: archived-stale" })
        .mockResolvedValue({ ...next, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.selectNote("archived-stale");

    expect(get(store.viewMode)).toBe("archive");
    expect(get(store.inbox).map((item) => item.id)).toEqual(["archived-next"]);
    expect(get(store.selectedNote)?.id).toBe("archived-next");
    expect(get(store.error)).toBeNull();
  });

  it("can save quick-capture text without refreshing the inbox", async () => {
    const api = testApi({
      listInbox: vi.fn().mockRejectedValue(new Error("refresh failed")),
    });
    const store = createWorkNotesStore(api) as ReturnType<typeof createWorkNotesStore> & {
      captureRawNote?: (rawText: string) => Promise<string | undefined>;
    };

    expect(store.captureRawNote).toBeTypeOf("function");
    await expect(store.captureRawNote?.("raw quick note")).resolves.toBe("note-1");

    expect(api.saveCaptureNote).toHaveBeenCalledWith("raw quick note");
    expect(api.listInbox).not.toHaveBeenCalled();
    expect(api.getNote).not.toHaveBeenCalled();
  });

  it("rejects settings persistence failures so callers keep the modal open", async () => {
    const api = testApi({
      saveSettings: vi.fn().mockRejectedValue(new Error("bad hotkey")),
    });
    const store = createWorkNotesStore(api);

    await expect(store.persistSettings(settings())).rejects.toThrow("bad hotkey");
    expect(get(store.error)).toBe("bad hotkey");
  });

  it("applies a theme optimistically before persistence resolves", async () => {
    let resolveSave: (value: AppSettings) => void = () => {};
    const api = testApi({
      getSettings: vi.fn().mockResolvedValue(settings()),
      saveSettings: vi.fn(
        () => new Promise<AppSettings>((resolve) => (resolveSave = resolve)),
      ),
    });
    const store = createWorkNotesStore(api);
    await store.loadSettings();

    const pending = store.setTheme("everforest-dark");
    expect(get(store.settings)?.selectedTheme).toBe("everforest-dark");

    resolveSave({ ...settings(), selectedTheme: "everforest-dark" });
    await pending;
    expect(get(store.settings)?.selectedTheme).toBe("everforest-dark");
  });

  it("persists the last saved settings merged with the new theme", async () => {
    const api = testApi({
      getSettings: vi.fn().mockResolvedValue({ ...settings(), codexCommandPath: "codex.cmd" }),
      saveSettings: vi.fn((next: AppSettings) => Promise.resolve(next)),
    });
    const store = createWorkNotesStore(api);
    await store.loadSettings();

    await store.setTheme("everforest-light");

    expect(api.saveSettings).toHaveBeenCalledWith({
      ...settings(),
      codexCommandPath: "codex.cmd",
      selectedTheme: "everforest-light",
    });
  });

  it("reverts the theme and reports an error when saving fails", async () => {
    const api = testApi({
      getSettings: vi.fn().mockResolvedValue(settings()),
      saveSettings: vi.fn().mockRejectedValue(new Error("disk full")),
    });
    const store = createWorkNotesStore(api);
    await store.loadSettings();

    await store.setTheme("everforest-dark");

    expect(get(store.settings)?.selectedTheme).toBe("dark-compact");
    expect(get(store.error)).toBe("disk full");
  });

  it("loads suggested actions into review queue state", async () => {
    const api = testApi({
      listSuggestedActions: vi.fn().mockResolvedValue([reviewItem({ id: "action-queued" })]),
    });
    const store = createWorkNotesStore(api);

    await store.loadSuggestedActions();

    expect(api.listSuggestedActions).toHaveBeenCalledTimes(1);
    expect(get(store.suggestedActions).map((item) => item.id)).toEqual(["action-queued"]);
    expect(get(store.loadingSuggestedActions)).toBe(false);
  });

  it("enters actions view, refreshes suggested actions, and selects the first source note", async () => {
    const firstAction = reviewItem({ id: "action-1", noteId: "note-action-1", noteTitle: "Action source" });
    const source = note({ id: "note-action-1", title: "Action source" });
    const api = testApi({
      listSuggestedActions: vi.fn().mockResolvedValue([firstAction]),
      getNote: vi.fn().mockResolvedValue({ ...source, actionItems: [] }),
    });
    const store = createWorkNotesStore(api);

    await store.showActions();

    expect(get(store.viewMode)).toBe("actions");
    expect(get(store.suggestedActions)).toEqual([firstAction]);
    expect(get(store.selectedNote)?.id).toBe("note-action-1");
    expect(api.listSuggestedActions).toHaveBeenCalledTimes(1);
  });

  it("enters follow-ups view and loads follow-ups", async () => {
    const item = followup({ id: "followup-1" });
    const api = testApi({
      listFollowups: vi.fn().mockResolvedValue([item]),
    });
    const store = createWorkNotesStore(api);

    await store.showArchive();
    await store.showFollowups();

    expect(get(store.viewMode)).toBe("followups");
    expect(get(store.filters)).toEqual(createInboxFilters({ includeArchived: false }));
    expect(api.listFollowups).toHaveBeenLastCalledWith(200);
    expect(get(store.followups)).toEqual([item]);
    expect(get(store.loadingFollowups)).toBe(false);
  });

  it("refreshes follow-ups after accepting when follow-ups view is active", async () => {
    const accepted = followup({ id: "action-1", status: "accepted" });
    const api = testApi({
      listFollowups: vi
        .fn()
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce([accepted]),
    });
    const store = createWorkNotesStore(api);

    await store.showFollowups();
    await store.acceptSuggestedAction("action-1");

    expect(api.acceptActionItem).toHaveBeenCalledWith("action-1");
    expect(api.listFollowups).toHaveBeenCalledTimes(2);
    expect(get(store.followups)).toEqual([accepted]);
  });

  it("creates a manual follow-up from the selected note and refreshes the board", async () => {
    const source = note({ id: "note-1" });
    const created = followup({
      id: "manual-1",
      noteId: "note-1",
      text: "Ask Jordan for the export sample.",
      source: "user",
      followupLane: "Waiting",
    });
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([source]),
      listFollowups: vi.fn().mockResolvedValue([created]),
      getNote: vi
        .fn()
        .mockResolvedValueOnce({ ...source, actionItems: [] })
        .mockResolvedValue({ ...source, actionItems: [created] }),
      createManualFollowup: vi.fn().mockResolvedValue(created),
    });
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.createFollowupFromSelectedNote("  Ask Jordan for the export sample.  ", "  Waiting  ");

    expect(api.createManualFollowup).toHaveBeenCalledWith(
      "note-1",
      "Ask Jordan for the export sample.",
      "Waiting",
    );
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
    expect(api.listInbox).toHaveBeenCalled();
    expect(api.listFollowups).toHaveBeenLastCalledWith(200);
    expect(get(store.selectedNote)?.actionItems.map((item) => item.id)).toEqual(["manual-1"]);
    expect(get(store.followups)).toEqual([created]);
  });

  it("updates follow-up state and lane", async () => {
    const current = followup({ id: "action-1", followupState: "open", followupLane: null });
    const waiting = followup({ id: "action-1", followupState: "waiting", followupLane: null });
    const assignedLane = followup({ id: "action-1", followupState: "waiting", followupLane: "Vendor" });
    const api = testApi({
      getNote: vi.fn().mockResolvedValue({ ...note(), actionItems: [current] }),
      listFollowups: vi
        .fn()
        .mockResolvedValueOnce([waiting])
        .mockResolvedValueOnce([assignedLane]),
    });
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.updateFollowupState("action-1", "waiting");
    await store.updateFollowupLane("action-1", "  Vendor  ");

    expect(api.updateFollowupState).toHaveBeenCalledWith("action-1", "waiting");
    expect(api.updateFollowupLane).toHaveBeenCalledWith("action-1", "Vendor");
    expect(api.listFollowups).toHaveBeenCalledTimes(2);
    expect(api.getNote).toHaveBeenCalledTimes(3);
    expect(get(store.followups)).toEqual([assignedLane]);
  });

  it("refreshes selected note, inbox, and suggested queue after accepting an action", async () => {
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([note({ suggestedActionItemCount: 0 })]),
      listSuggestedActions: vi.fn().mockResolvedValue([]),
    });
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.acceptSuggestedAction("action-1");

    expect(api.acceptActionItem).toHaveBeenCalledWith("action-1");
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
    expect(api.listInbox).toHaveBeenCalled();
    expect(api.listSuggestedActions).toHaveBeenCalled();
  });

  it("completes and reopens actions through the workflow store", async () => {
    const api = testApi();
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.completeAction("action-1");
    await store.reopenAction("action-1");

    expect(api.completeActionItem).toHaveBeenCalledWith("action-1");
    expect(api.reopenActionItem).toHaveBeenCalledWith("action-1");
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
  });
});

type TestApi = NonNullable<Parameters<typeof createWorkNotesStore>[0]>;

function testApi(overrides: Partial<TestApi> = {}): TestApi {
  const detail: NoteDetail = { ...note(), actionItems: [] };

  return {
    saveCaptureNote: vi.fn<(rawText: string) => Promise<NoteDetail>>().mockResolvedValue(detail),
    listInbox: vi.fn<(filters: InboxFilters) => Promise<NoteListItem[]>>().mockResolvedValue([]),
    getNote: vi.fn<(noteId: string) => Promise<NoteDetail>>().mockResolvedValue(detail),
    retryParse: vi.fn<(noteId: string) => Promise<void>>().mockResolvedValue(undefined),
    retryParseWithFeedback: vi
      .fn<(noteId: string, feedback: string) => Promise<void>>()
      .mockResolvedValue(undefined),
    updateNoteCleaned: vi
      .fn<(noteId: string, fields: { title: string; summary: string; cleanedText: string }) => Promise<NoteDetail>>()
      .mockResolvedValue(detail),
    deleteNote: vi.fn<(noteId: string) => Promise<void>>().mockResolvedValue(undefined),
    restoreNote: vi.fn<(noteId: string) => Promise<void>>().mockResolvedValue(undefined),
    permanentlyDeleteNote: vi.fn<(noteId: string) => Promise<void>>().mockResolvedValue(undefined),
    acceptActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    dismissActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    completeActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    reopenActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    listFollowups: vi.fn<(limit?: number) => Promise<FollowupItem[]>>().mockResolvedValue([]),
    createManualFollowup: vi
      .fn<(noteId: string, text: string, lane?: string | null) => Promise<ActionItem>>()
      .mockResolvedValue(followup()),
    updateFollowupState: vi
      .fn<(actionItemId: string, state: FollowupState) => Promise<void>>()
      .mockResolvedValue(undefined),
    updateFollowupLane: vi
      .fn<(actionItemId: string, lane?: string | null) => Promise<void>>()
      .mockResolvedValue(undefined),
    listSuggestedActions: vi.fn<(limit?: number) => Promise<ActionReviewItem[]>>().mockResolvedValue([]),
    getSettings: vi.fn<() => Promise<AppSettings>>().mockResolvedValue(settings()),
    saveSettings: vi.fn<(settings: AppSettings) => Promise<AppSettings>>().mockResolvedValue(settings()),
    ...overrides,
  };
}

function reviewItem(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Kiosk 7 telemetry IDs",
    text: "Bring serial list into the Tuesday sync.",
    owner: "Maya",
    dueDate: null,
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Kiosk 7 telemetry IDs",
    text: "Bring serial list into the Tuesday sync.",
    owner: "Maya",
    dueDate: null,
    status: "accepted",
    source: "parser",
    confidence: 0.82,
    followupState: "open",
    followupLane: null,
    tags: [{ id: "tag-maya", name: "Maya", kind: "person", source: "ai", confidence: 0.94 }],
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

function settings(): AppSettings {
  return {
    hotkey: "Ctrl+Shift+Space",
    parserTimeoutSeconds: 30,
    parserMaxRetries: 3,
    codexCommandPath: "codex",
    linkedWorkspacePaths: [],
    selectedTheme: "dark-compact",
  };
}
