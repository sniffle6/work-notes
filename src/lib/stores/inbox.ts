import { derived, get, writable } from "svelte/store";

import {
  acceptActionItem,
  completeActionItem,
  createManualFollowup,
  dismissActionItem,
  getNote,
  getSettings,
  listFollowups,
  listInbox,
  listSuggestedActions,
  reopenActionItem,
  retryParse,
  retryParseWithFeedback,
  deleteNote,
  restoreNote,
  permanentlyDeleteNote,
  saveCaptureNote,
  saveSettings,
  updateFollowupLane as updateFollowupLaneApi,
  updateFollowupState as updateFollowupStateApi,
} from "$lib/api";
import type {
  ActionReviewItem,
  AppSettings,
  FollowupItem,
  FollowupState,
  InboxFilters,
  NoteDetail,
  NoteListItem,
} from "$lib/types";
export { createInboxFilters, matchesNoteFilters } from "./filters";
import { createInboxFilters, matchesNoteFilters } from "./filters";

export type InboxViewMode = "inbox" | "archive" | "actions" | "today" | "people" | "followups";

type WorkNotesApi = {
  saveCaptureNote: typeof saveCaptureNote;
  listInbox: typeof listInbox;
  getNote: typeof getNote;
  retryParse: typeof retryParse;
  retryParseWithFeedback: typeof retryParseWithFeedback;
  deleteNote: typeof deleteNote;
  restoreNote: typeof restoreNote;
  permanentlyDeleteNote: typeof permanentlyDeleteNote;
  acceptActionItem: typeof acceptActionItem;
  dismissActionItem: typeof dismissActionItem;
  completeActionItem: typeof completeActionItem;
  reopenActionItem: typeof reopenActionItem;
  listFollowups: typeof listFollowups;
  createManualFollowup: typeof createManualFollowup;
  updateFollowupState: typeof updateFollowupStateApi;
  updateFollowupLane: typeof updateFollowupLaneApi;
  listSuggestedActions: typeof listSuggestedActions;
  getSettings: typeof getSettings;
  saveSettings: typeof saveSettings;
};

const defaultApi: WorkNotesApi = {
  saveCaptureNote,
  listInbox,
  getNote,
  retryParse,
  retryParseWithFeedback,
  deleteNote,
  restoreNote,
  permanentlyDeleteNote,
  acceptActionItem,
  dismissActionItem,
  completeActionItem,
  reopenActionItem,
  listFollowups,
  createManualFollowup,
  updateFollowupState: updateFollowupStateApi,
  updateFollowupLane: updateFollowupLaneApi,
  listSuggestedActions,
  getSettings,
  saveSettings,
};

export function createWorkNotesStore(api: WorkNotesApi = defaultApi) {
  const inbox = writable<NoteListItem[]>([]);
  const filters = writable<InboxFilters>(createInboxFilters());
  const viewMode = writable<InboxViewMode>("inbox");
  const selectedNote = writable<NoteDetail | null>(null);
  const suggestedActions = writable<ActionReviewItem[]>([]);
  const followups = writable<FollowupItem[]>([]);
  const settings = writable<AppSettings | null>(null);
  const loadingInbox = writable(false);
  const loadingNote = writable(false);
  const loadingSuggestedActions = writable(false);
  const loadingFollowups = writable(false);
  const savingCapture = writable(false);
  const savingSettings = writable(false);
  const busyActionId = writable<string | null>(null);
  const error = writable<string | null>(null);

  const filteredInbox = derived([inbox, filters, viewMode], ([$inbox, $filters, $viewMode]) =>
    $inbox.filter((note) =>
      matchesNoteFilters(
        note,
        createInboxFilters({
          ...$filters,
          includeArchived: $viewMode === "archive",
        }),
      ),
    ),
  );

  type LoadInboxOptions = {
    preferredSelectionIndex?: number;
    limit?: number;
  };

  async function loadInbox(options: LoadInboxOptions = {}): Promise<void> {
    loadingInbox.set(true);
    error.set(null);

    try {
      const mode = get(viewMode);
      const currentFilters = get(filters);
      const backendFilters = createInboxFilters({
        ...currentFilters,
        includeArchived: mode === "archive",
        limit: options.limit,
      });
      const items = await api.listInbox(backendFilters);
      const visibleItems =
        mode === "archive"
          ? items.filter((note) => note.isArchived)
          : items.filter((note) => !note.isArchived);

      inbox.set(visibleItems);
      await ensureSelectionAfterLoad(visibleItems, options.preferredSelectionIndex);
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load inbox."));
    } finally {
      loadingInbox.set(false);
    }
  }

  async function selectNote(noteId: string): Promise<void> {
    loadingNote.set(true);
    error.set(null);

    try {
      selectedNote.set(await api.getNote(noteId));
    } catch (unknownError) {
      if (isNotFoundError(unknownError)) {
        selectedNote.set(null);
        await loadInbox();
        return;
      }

      error.set(errorMessage(unknownError, "Could not load note."));
    } finally {
      loadingNote.set(false);
    }
  }

  async function captureRawNote(rawText: string): Promise<string | undefined> {
    const trimmed = rawText.trim();
    if (!trimmed) {
      return undefined;
    }

    savingCapture.set(true);
    error.set(null);

    try {
      const saved = await api.saveCaptureNote(trimmed);
      return saved.id;
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not save note."));
      throw unknownError;
    } finally {
      savingCapture.set(false);
    }
  }

  async function loadSuggestedActions(limit?: number): Promise<void> {
    loadingSuggestedActions.set(true);
    error.set(null);

    try {
      suggestedActions.set(await api.listSuggestedActions(limit));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load suggested actions."));
    } finally {
      loadingSuggestedActions.set(false);
    }
  }

  async function loadFollowups(limit = 200): Promise<void> {
    loadingFollowups.set(true);
    error.set(null);

    try {
      followups.set(await api.listFollowups(limit));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load follow-ups."));
    } finally {
      loadingFollowups.set(false);
    }
  }

  async function saveCapture(rawText: string): Promise<string | undefined> {
    const noteId = await captureRawNote(rawText);
    if (noteId) {
      await showCapturedNote(noteId);
    }
    return noteId;
  }

  async function retrySelectedParse(): Promise<void> {
    const note = get(selectedNote);
    if (!note) {
      return;
    }

    loadingNote.set(true);
    error.set(null);

    try {
      await api.retryParse(note.id);
      await loadInbox();
      selectedNote.set(await api.getNote(note.id));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not retry parse."));
    } finally {
      loadingNote.set(false);
    }
  }

  async function retrySelectedParseWithFeedback(feedback: string): Promise<void> {
    const note = get(selectedNote);
    const trimmed = feedback.trim();
    if (!note || !trimmed) {
      return;
    }

    loadingNote.set(true);
    error.set(null);

    try {
      await api.retryParseWithFeedback(note.id, trimmed);
      await loadInbox();
      selectedNote.set(await api.getNote(note.id));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not reparse note."));
    } finally {
      loadingNote.set(false);
    }
  }

  async function deleteSelectedNote(): Promise<void> {
    const note = get(selectedNote);
    if (!note) {
      return;
    }

    loadingNote.set(true);
    error.set(null);

    try {
      await api.deleteNote(note.id);
      selectedNote.set(null);
      await loadInbox();
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not delete note."));
    } finally {
      loadingNote.set(false);
    }
  }

  async function acceptSuggestedAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.acceptActionItem);
  }

  async function dismissSuggestedAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.dismissActionItem);
  }

  async function showInbox(): Promise<void> {
    viewMode.set("inbox");
    filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
    await loadInbox();
  }

  async function showArchive(): Promise<void> {
    viewMode.set("archive");
    filters.update((current) => createInboxFilters({ ...current, includeArchived: true }));
    await loadInbox();
  }

  async function showActions(): Promise<void> {
    viewMode.set("actions");
    filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
    await loadSuggestedActions();

    const actions = get(suggestedActions);
    const currentSelected = get(selectedNote);
    if (
      actions.length > 0 &&
      (!currentSelected || !actions.some((action) => action.noteId === currentSelected.id))
    ) {
      await selectNote(actions[0].noteId);
    }
  }

  async function showToday(): Promise<void> {
    viewMode.set("today");
    filters.set(createInboxFilters({ includeArchived: false }));
    await loadInbox();
    await loadSuggestedActions();
  }

  async function showPeople(): Promise<void> {
    viewMode.set("people");
    filters.set(createInboxFilters({ includeArchived: false }));
    await loadInbox({ limit: 1000 });
    await loadSuggestedActions(500);
  }

  async function showFollowups(): Promise<void> {
    viewMode.set("followups");
    filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
    await loadFollowups();
  }

  async function loadArchive(): Promise<void> {
    await showArchive();
  }

  async function restoreSelectedNote(): Promise<void> {
    const note = get(selectedNote);
    if (!note) {
      return;
    }

    loadingNote.set(true);
    error.set(null);

    try {
      await api.restoreNote(note.id);
      viewMode.set("inbox");
      filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
      await loadInbox();
      await selectNote(note.id);
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not restore note."));
    } finally {
      loadingNote.set(false);
    }
  }

  async function permanentlyDeleteSelectedNote(): Promise<void> {
    const note = get(selectedNote);
    if (!note) {
      return;
    }
    const archivedItems = get(inbox);
    const deletedIndex = archivedItems.findIndex((item) => item.id === note.id);

    loadingNote.set(true);
    error.set(null);

    try {
      await api.permanentlyDeleteNote(note.id);
      selectedNote.set(null);
      await loadInbox({
        preferredSelectionIndex: deletedIndex >= 0 ? deletedIndex : undefined,
      });
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not permanently delete note."));
    } finally {
      loadingNote.set(false);
    }
  }

  async function completeAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.completeActionItem, false);
  }

  async function reopenAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.reopenActionItem, false);
  }

  async function createFollowupFromSelectedNote(
    text: string,
    lane?: string | null,
  ): Promise<void> {
    const note = get(selectedNote);
    const trimmed = text.trim();
    if (!note || !trimmed) {
      return;
    }

    loadingNote.set(true);
    error.set(null);

    try {
      await api.createManualFollowup(note.id, trimmed, normalizeLane(lane));
      selectedNote.set(await api.getNote(note.id));
      await loadInbox();
      await loadFollowups();
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not create follow-up."));
      throw unknownError;
    } finally {
      loadingNote.set(false);
    }
  }

  async function updateFollowupState(
    actionItemId: string,
    state: FollowupState,
  ): Promise<void> {
    await updateFollowup(actionItemId, () => api.updateFollowupState(actionItemId, state));
  }

  async function updateFollowupLane(actionItemId: string, lane?: string | null): Promise<void> {
    const nextLane = normalizeLane(lane);
    await updateFollowup(actionItemId, () => api.updateFollowupLane(actionItemId, nextLane));
  }

  async function loadSettings(): Promise<void> {
    try {
      settings.set(await api.getSettings());
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load settings."));
    }
  }

  async function persistSettings(nextSettings: AppSettings): Promise<void> {
    savingSettings.set(true);
    error.set(null);

    try {
      settings.set(await api.saveSettings(nextSettings));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not save settings."));
      throw unknownError;
    } finally {
      savingSettings.set(false);
    }
  }

  async function updateFilters(nextFilters: Partial<InboxFilters>): Promise<void> {
    filters.update((current) => createInboxFilters({ ...current, ...nextFilters }));
    await loadInbox();
  }

  async function showCapturedNote(noteId: string): Promise<void> {
    viewMode.set("inbox");
    filters.set(createInboxFilters({ includeArchived: false }));
    await loadInbox();
    await selectNote(noteId);
  }

  async function ensureSelectionAfterLoad(
    items: NoteListItem[],
    preferredSelectionIndex?: number,
  ): Promise<void> {
    const currentSelected = get(selectedNote);
    if (items.length === 0) {
      selectedNote.set(null);
      return;
    }

    if (!currentSelected || !items.some((item) => item.id === currentSelected.id)) {
      const index =
        preferredSelectionIndex === undefined
          ? 0
          : Math.min(Math.max(preferredSelectionIndex, 0), items.length - 1);
      await selectNote(items[index].id);
    }
  }

  async function updateAction(
    actionItemId: string,
    update:
      | WorkNotesApi["acceptActionItem"]
      | WorkNotesApi["dismissActionItem"]
      | WorkNotesApi["completeActionItem"]
      | WorkNotesApi["reopenActionItem"],
    refreshSuggestedActions = true,
  ): Promise<void> {
    busyActionId.set(actionItemId);
    error.set(null);

    try {
      await update(actionItemId);
      const note = get(selectedNote);
      if (note) {
        selectedNote.set(await api.getNote(note.id));
      }
      await loadInbox();
      if (refreshSuggestedActions) {
        await loadSuggestedActions();
      }
      if (get(viewMode) === "followups") {
        await loadFollowups();
      }
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not update action."));
    } finally {
      busyActionId.set(null);
    }
  }

  async function updateFollowup(actionItemId: string, update: () => Promise<void>): Promise<void> {
    const shouldRefreshSelectedNote = selectedNoteHasAction(actionItemId);

    busyActionId.set(actionItemId);
    error.set(null);

    try {
      await update();
      await loadFollowups();
      if (shouldRefreshSelectedNote) {
        const note = get(selectedNote);
        if (note) {
          selectedNote.set(await api.getNote(note.id));
        }
      }
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not update follow-up."));
    } finally {
      busyActionId.set(null);
    }
  }

  function selectedNoteHasAction(actionItemId: string): boolean {
    const note = get(selectedNote);
    if (!note) {
      return false;
    }

    return (
      note.actionItems.some((action) => action.id === actionItemId) ||
      get(followups).some((item) => item.id === actionItemId && item.noteId === note.id)
    );
  }

  return {
    inbox,
    filters,
    viewMode,
    filteredInbox,
    selectedNote,
    suggestedActions,
    followups,
    settings,
    loadingInbox,
    loadingNote,
    loadingSuggestedActions,
    loadingFollowups,
    savingCapture,
    savingSettings,
    busyActionId,
    error,
    captureRawNote,
    loadInbox,
    loadArchive,
    loadSuggestedActions,
    loadFollowups,
    selectNote,
    saveCapture,
    showCapturedNote,
    retrySelectedParse,
    retrySelectedParseWithFeedback,
    deleteSelectedNote,
    showInbox,
    showArchive,
    showActions,
    showToday,
    showPeople,
    showFollowups,
    restoreSelectedNote,
    permanentlyDeleteSelectedNote,
    acceptSuggestedAction,
    dismissSuggestedAction,
    completeAction,
    reopenAction,
    createFollowupFromSelectedNote,
    updateFollowupState,
    updateFollowupLane,
    loadSettings,
    persistSettings,
    updateFilters,
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

function normalizeLane(lane?: string | null): string | null {
  return lane?.trim() || null;
}

function isNotFoundError(error: unknown): boolean {
  if (error instanceof Error) {
    return error.message.toLocaleLowerCase().includes("not found");
  }

  if (!error || typeof error !== "object") {
    return false;
  }

  const record = error as Record<string, unknown>;
  if (record.code === "not_found") {
    return true;
  }

  return typeof record.message === "string" && record.message.toLocaleLowerCase().includes("not found");
}
