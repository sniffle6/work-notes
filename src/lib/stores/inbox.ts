import { derived, get, writable } from "svelte/store";

import {
  acceptActionItem,
  completeActionItem,
  dismissActionItem,
  getNote,
  getSettings,
  listInbox,
  listSuggestedActions,
  reopenActionItem,
  retryParse,
  retryParseWithFeedback,
  deleteNote,
  saveCaptureNote,
  saveSettings,
} from "$lib/api";
import type { ActionReviewItem, AppSettings, InboxFilters, NoteDetail, NoteListItem } from "$lib/types";
export { createInboxFilters, matchesNoteFilters } from "./filters";
import { createInboxFilters, matchesNoteFilters } from "./filters";

type WorkNotesApi = {
  saveCaptureNote: typeof saveCaptureNote;
  listInbox: typeof listInbox;
  getNote: typeof getNote;
  retryParse: typeof retryParse;
  retryParseWithFeedback: typeof retryParseWithFeedback;
  deleteNote: typeof deleteNote;
  acceptActionItem: typeof acceptActionItem;
  dismissActionItem: typeof dismissActionItem;
  completeActionItem: typeof completeActionItem;
  reopenActionItem: typeof reopenActionItem;
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
  acceptActionItem,
  dismissActionItem,
  completeActionItem,
  reopenActionItem,
  listSuggestedActions,
  getSettings,
  saveSettings,
};

export function createWorkNotesStore(api: WorkNotesApi = defaultApi) {
  const inbox = writable<NoteListItem[]>([]);
  const filters = writable<InboxFilters>(createInboxFilters());
  const selectedNote = writable<NoteDetail | null>(null);
  const suggestedActions = writable<ActionReviewItem[]>([]);
  const settings = writable<AppSettings | null>(null);
  const loadingInbox = writable(false);
  const loadingNote = writable(false);
  const loadingSuggestedActions = writable(false);
  const savingCapture = writable(false);
  const savingSettings = writable(false);
  const busyActionId = writable<string | null>(null);
  const error = writable<string | null>(null);

  const filteredInbox = derived([inbox, filters], ([$inbox, $filters]) =>
    $inbox.filter((note) => matchesNoteFilters(note, $filters)),
  );

  async function loadInbox(): Promise<void> {
    loadingInbox.set(true);
    error.set(null);

    try {
      const currentFilters = get(filters);
      const items = await api.listInbox(currentFilters);
      inbox.set(items);

      const currentSelected = get(selectedNote);
      if (items.length === 0) {
        selectedNote.set(null);
        return;
      }

      if (!currentSelected || !items.some((item) => item.id === currentSelected.id)) {
        await selectNote(items[0].id);
      }
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

  async function loadSuggestedActions(): Promise<void> {
    loadingSuggestedActions.set(true);
    error.set(null);

    try {
      suggestedActions.set(await api.listSuggestedActions());
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load suggested actions."));
    } finally {
      loadingSuggestedActions.set(false);
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

  async function completeAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.completeActionItem, false);
  }

  async function reopenAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.reopenActionItem, false);
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
    filters.set(createInboxFilters());
    await loadInbox();
    await selectNote(noteId);
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
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not update action."));
    } finally {
      busyActionId.set(null);
    }
  }

  return {
    inbox,
    filters,
    filteredInbox,
    selectedNote,
    suggestedActions,
    settings,
    loadingInbox,
    loadingNote,
    loadingSuggestedActions,
    savingCapture,
    savingSettings,
    busyActionId,
    error,
    captureRawNote,
    loadInbox,
    loadSuggestedActions,
    selectNote,
    saveCapture,
    showCapturedNote,
    retrySelectedParse,
    retrySelectedParseWithFeedback,
    deleteSelectedNote,
    acceptSuggestedAction,
    dismissSuggestedAction,
    completeAction,
    reopenAction,
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
