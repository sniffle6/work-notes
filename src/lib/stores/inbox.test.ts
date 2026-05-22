import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

import type { ActionReviewItem, AppSettings, InboxFilters, NoteDetail, NoteListItem } from "$lib/types";
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

    expect(get(store.filters)).toEqual(createInboxFilters());
    expect(api.listInbox).toHaveBeenLastCalledWith(createInboxFilters());
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
    deleteNote: vi.fn<(noteId: string) => Promise<void>>().mockResolvedValue(undefined),
    acceptActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    dismissActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    completeActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    reopenActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    listSuggestedActions: vi.fn<() => Promise<ActionReviewItem[]>>().mockResolvedValue([]),
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

function settings(): AppSettings {
  return {
    hotkey: "Ctrl+Shift+Space",
    parserTimeoutSeconds: 30,
    parserMaxRetries: 3,
    codexCommandPath: "codex",
    selectedTheme: "dark-compact",
  };
}
