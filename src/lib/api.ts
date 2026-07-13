import { invoke } from "@tauri-apps/api/core";

import { withRetry } from "$lib/retry";
import type {
  ActionItem,
  ActionReviewItem,
  AppSettings,
  CardNote,
  FollowupItem,
  FollowupState,
  InboxFilters,
  NoteDetail,
  NoteListItem,
  ParseStatus,
  ReviewStatus,
  Tag,
} from "$lib/types";

type UnknownRecord = Record<string, unknown>;

const fallbackNow = previewTimestamp(0, 13, 42);

function previewDateKey(dayOffset: number): string {
  const date = new Date();
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() + dayOffset);
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

function previewTimestamp(dayOffset: number, hour: number, minute: number): string {
  const date = new Date();
  date.setDate(date.getDate() + dayOffset);
  date.setHours(hour, minute, 0, 0);
  return date.toISOString();
}

let fallbackNotes: NoteDetail[] = [
  {
    id: "fallback-archived-1",
    title: "Archived workspace cleanup note",
    rawText: "Archived workspace cleanup note",
    cleanedText: "Archived workspace cleanup note",
    summary: "Archived workspace cleanup note",
    captureSource: "quick_capture",
    createdAt: previewTimestamp(-30, 13, 39),
    updatedAt: previewTimestamp(-30, 13, 39),
    parseStatus: "parsed",
    reviewStatus: "reviewed",
    isArchived: true,
    tags: [{ id: "fallback-tag-archive", name: "archive", kind: "topic", source: "user" }],
    actionItemCount: 0,
    suggestedActionItemCount: 0,
    actionItems: [],
    cardNotes: [],
  },
  {
    id: "n-1024",
    title: "Kiosk 7 telemetry IDs",
    rawText: "Maya: bring serial list into the Tuesday sync and flag missing asset tags before the vendor call.",
    cleanedText:
      "Maya needs the kiosk 7 serial list brought into the Tuesday sync, with missing asset tags flagged before the vendor call.",
    summary: "Bring kiosk 7 serials to Tuesday sync and flag missing asset tags.",
    captureSource: "quick_capture",
    createdAt: previewTimestamp(-5, 9, 42),
    updatedAt: previewTimestamp(-5, 9, 45),
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [
      { id: "tag-maya", name: "Maya", kind: "person", source: "ai", confidence: 0.94 },
      { id: "tag-kiosk", name: "Kiosk", kind: "topic", source: "ai", confidence: 0.88 },
    ],
    actionItemCount: 4,
    suggestedActionItemCount: 2,
    cardNotes: [],
    actionItems: [
      {
        id: "a-1024",
        noteId: "n-1024",
        text: "Bring serial list into the Tuesday sync.",
        owner: "Maya",
        dueDate: previewDateKey(-4),
        status: "suggested",
        source: "parser",
        confidence: 0.82,
        noteTitle: "Kiosk 7 telemetry IDs",
      },
      {
        id: "a-1025",
        noteId: "n-1024",
        text: "Confirm replacement asset tags with the vendor.",
        owner: "Maya",
        dueDate: previewDateKey(3),
        status: "accepted",
        source: "parser",
        confidence: 0.88,
        noteTitle: "Kiosk 7 telemetry IDs",
        followupState: "waiting",
        followupLane: "Kiosk rollout",
      },
      {
        id: "a-1026",
        noteId: "n-1024",
        text: "Collect the kiosk serial numbers from facilities.",
        owner: "Maya",
        dueDate: previewDateKey(-2),
        status: "done",
        source: "parser",
        confidence: 0.91,
        noteTitle: "Kiosk 7 telemetry IDs",
        followupState: "open",
        followupLane: "Kiosk rollout",
        completedAt: previewTimestamp(-1, 16, 20),
      },
      {
        id: "a-1027",
        noteId: "n-1024",
        text: "Schedule the post-install telemetry review.",
        owner: "Maya",
        dueDate: previewDateKey(12),
        status: "suggested",
        source: "parser",
        confidence: 0.76,
        noteTitle: "Kiosk 7 telemetry IDs",
      },
    ],
  },
  {
    id: "n-1023",
    title: "Pricing export mismatch",
    rawText: "Jordan said Finance sees different totals in the CSV than the dashboard. Needs last filtered query and owner.",
    cleanedText: null,
    summary: "Finance sees CSV totals that do not match the dashboard.",
    captureSource: "quick_capture",
    createdAt: previewTimestamp(-2, 9, 18),
    updatedAt: previewTimestamp(-2, 9, 18),
    parseStatus: "failed",
    reviewStatus: "none",
    tags: [{ id: "tag-finance", name: "Finance", kind: "topic", source: "ai", confidence: 0.91 }],
    actionItemCount: 0,
    suggestedActionItemCount: 0,
    actionItems: [],
    cardNotes: [],
    parseError: "Codex parser timed out.",
  },
  {
    id: "n-1022",
    title: "Visitor badge printer",
    rawText: "Rina: front desk can print test labels, but badge names are shifted one line on real entries.",
    cleanedText: "The visitor badge printer shifts names down one line on real entries, though test labels print correctly.",
    summary: "Badge printer alignment fails for real visitor entries.",
    captureSource: "quick_capture",
    createdAt: previewTimestamp(-4, 14, 30),
    updatedAt: previewTimestamp(-4, 14, 35),
    parseStatus: "parsed",
    reviewStatus: "reviewed",
    tags: [{ id: "tag-front-desk", name: "Front desk", kind: "project", source: "ai", confidence: 0.86 }],
    actionItemCount: 4,
    suggestedActionItemCount: 0,
    cardNotes: [],
    actionItems: [
      {
        id: "a-1022",
        noteId: "n-1022",
        text: "Check badge printer template alignment for real entries.",
        owner: "Rina",
        dueDate: previewDateKey(0),
        status: "accepted",
        source: "parser",
        confidence: 0.78,
        noteTitle: "Visitor badge printer",
        followupState: "open",
        followupLane: null,
      },
      {
        id: "a-1022-2",
        noteId: "n-1022",
        text: "Print a calibration sheet on the front desk printer.",
        owner: "Rina",
        dueDate: previewDateKey(0),
        status: "done",
        source: "user",
        confidence: null,
        noteTitle: "Visitor badge printer",
        followupState: "open",
        followupLane: "Front desk",
        completedAt: previewTimestamp(0, 9, 15),
      },
      {
        id: "a-1022-3",
        noteId: "n-1022",
        text: "Install the corrected badge template at reception.",
        owner: "Rina",
        dueDate: previewDateKey(7),
        status: "accepted",
        source: "parser",
        confidence: 0.84,
        noteTitle: "Visitor badge printer",
        followupState: "open",
        followupLane: "Front desk",
      },
      {
        id: "a-1022-4",
        noteId: "n-1022",
        text: "Document the production printer offset settings.",
        owner: "Rina",
        dueDate: previewDateKey(-4),
        status: "done",
        source: "user",
        confidence: null,
        noteTitle: "Visitor badge printer",
        followupState: "open",
        followupLane: "Front desk",
        completedAt: previewTimestamp(-3, 14, 40),
      },
    ],
  },
];

let fallbackSettings: AppSettings = {
  hotkey: "Ctrl+Shift+Space",
  parserTimeoutSeconds: 90,
  parserMaxRetries: 3,
  codexCommandPath: "codex.cmd",
  linkedWorkspacePaths: [],
  selectedTheme: "dark-compact",
  launchAtStartup: false,
  minimizeToTray: true,
};

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Idempotent read commands that are safe to retry. On a cold/racy startup
// (notably the relaunch after a Windows auto-update), the webview can fire
// these before the Rust backend has finished managing app state, so the first
// call rejects. Retrying lets the UI self-heal instead of sitting empty until
// a manual restart. Writes are never retried — they could apply twice.
const RETRYABLE_READ_COMMANDS = new Set([
  "list_inbox",
  "list_suggested_actions",
  "get_settings",
  "list_followups",
]);

async function invokeCommand<T>(command: string, args?: UnknownRecord): Promise<T> {
  if (!isTauriRuntime()) {
    return fallbackCommand<T>(command, args);
  }

  if (RETRYABLE_READ_COMMANDS.has(command)) {
    return withRetry(() => invoke<T>(command, args), { attempts: 5, delayMs: 400 });
  }

  return invoke<T>(command, args);
}

export async function saveCaptureNote(rawText: string): Promise<NoteDetail> {
  const saved = await invokeCommand<unknown>("save_capture_note", { rawText });
  return normalizeNoteDetail(saved);
}

export async function listInbox(filters: InboxFilters): Promise<NoteListItem[]> {
  const notes = await invokeCommand<unknown[]>("list_inbox", { filters: toBackendFilters(filters) });
  return notes.map(normalizeNoteListItem);
}

export async function getNote(noteId: string): Promise<NoteDetail> {
  const note = await invokeCommand<unknown>("get_note", { id: noteId });
  return normalizeNoteDetail(note);
}

export async function retryParse(noteId: string): Promise<void> {
  await invokeCommand<void>("retry_parse", { noteId });
}

export async function retryParseWithFeedback(noteId: string, feedback: string): Promise<void> {
  await invokeCommand<void>("retry_parse_with_feedback", { noteId, feedback });
}

export async function updateNoteCleaned(
  noteId: string,
  fields: { title: string; summary: string; cleanedText: string },
): Promise<NoteDetail> {
  const note = await invokeCommand<unknown>("update_note_cleaned", {
    noteId,
    title: fields.title,
    cleanedText: fields.cleanedText,
    summary: fields.summary,
  });
  return normalizeNoteDetail(note);
}

export async function updateNoteRaw(noteId: string, rawText: string): Promise<NoteDetail> {
  const note = await invokeCommand<unknown>("update_note_raw", { noteId, rawText });
  return normalizeNoteDetail(note);
}

export async function deleteNote(noteId: string): Promise<void> {
  await invokeCommand<void>("delete_note", { noteId });
}

export async function completeNote(noteId: string, completionNote: string | null): Promise<void> {
  await invokeCommand<void>("complete_note", { noteId, completionNote });
}

export async function addCardNote(noteId: string, text: string): Promise<NoteDetail> {
  const note = await invokeCommand<unknown>("add_card_note", { noteId, text });
  return normalizeNoteDetail(note);
}

export async function reopenNote(noteId: string): Promise<void> {
  await invokeCommand<void>("reopen_note", { noteId });
}

export async function restoreNote(noteId: string): Promise<void> {
  await invokeCommand<void>("restore_note", { noteId });
}

export async function permanentlyDeleteNote(noteId: string): Promise<void> {
  await invokeCommand<void>("permanently_delete_note", { noteId });
}

export async function acceptActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("accept_action_item", { actionId: actionItemId });
}

export async function dismissActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("dismiss_action_item", { actionId: actionItemId });
}

export async function completeActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("complete_action_item", { actionId: actionItemId });
}

export async function reopenActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("reopen_action_item", { actionId: actionItemId });
}

export async function listFollowups(limit = 200): Promise<FollowupItem[]> {
  const followups = await invokeCommand<unknown[]>("list_followups", { limit });
  return followups.map(normalizeFollowupItem);
}

export async function createManualFollowup(
  noteId: string,
  text: string,
  lane?: string | null,
): Promise<ActionItem> {
  const action = await invokeCommand<unknown>("create_manual_followup", {
    input: { noteId, text, laneOverride: lane?.trim() || null },
  });
  return normalizeActionItem(action);
}

export async function updateFollowupState(actionItemId: string, state: FollowupState): Promise<void> {
  await invokeCommand<void>("update_followup_state", { input: { id: actionItemId, state } });
}

export async function updateFollowupLane(actionItemId: string, lane?: string | null): Promise<void> {
  await invokeCommand<void>("update_followup_lane", {
    input: { id: actionItemId, laneOverride: lane?.trim() || null },
  });
}

export async function listSuggestedActions(limit = 100): Promise<ActionReviewItem[]> {
  const actions = await invokeCommand<unknown[]>("list_suggested_actions", { limit });
  return actions.map(normalizeActionReviewItem);
}

export async function getSettings(): Promise<AppSettings> {
  const settings = await invokeCommand<unknown>("get_settings");
  return normalizeSettings(settings);
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  const saved = await invokeCommand<unknown>("save_settings", { settings: toBackendSettings(settings) });
  return normalizeSettings(saved);
}

export async function selectLinkedWorkspaceDirectory(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Link repo or directory",
  });

  if (Array.isArray(selected)) {
    return typeof selected[0] === "string" ? selected[0] : null;
  }

  return typeof selected === "string" ? selected : null;
}

export async function hideQuickCapture(): Promise<void> {
  await invokeCommand<void>("hide_quick_capture");
}

export const api = {
  saveCaptureNote,
  listInbox,
  getNote,
  retryParse,
  retryParseWithFeedback,
  updateNoteCleaned,
  updateNoteRaw,
  deleteNote,
  completeNote,
  addCardNote,
  reopenNote,
  restoreNote,
  permanentlyDeleteNote,
  acceptActionItem,
  dismissActionItem,
  completeActionItem,
  reopenActionItem,
  listFollowups,
  createManualFollowup,
  updateFollowupState,
  updateFollowupLane,
  listSuggestedActions,
  getSettings,
  saveSettings,
  selectLinkedWorkspaceDirectory,
  hideQuickCapture,
};

function toBackendFilters(filters: InboxFilters): UnknownRecord {
  return {
    parseStatus: filters.parseStatuses[0] ?? null,
    reviewStatus: filters.reviewStatuses[0] ?? null,
    tagIds: filters.tagIds,
    query: filters.search.trim() || null,
    includeArchived: Boolean(filters.includeArchived),
    includeCompleted: Boolean(filters.includeCompleted),
    limit: filters.limit ?? null,
  };
}

function toBackendSettings(settings: AppSettings): UnknownRecord {
  return {
    hotkey: settings.hotkey,
    globalHotkey: settings.hotkey,
    parserTimeoutSeconds: settings.parserTimeoutSeconds,
    parserMaxRetries: settings.parserMaxRetries ?? 3,
    codexCommandPath: settings.codexCommandPath,
    linkedWorkspacePaths: settings.linkedWorkspacePaths ?? [],
    selectedTheme: settings.selectedTheme,
    theme: settings.selectedTheme,
    launchAtStartup: settings.launchAtStartup ?? false,
    minimizeToTray: settings.minimizeToTray ?? true,
  };
}

function normalizeNoteListItem(value: unknown): NoteListItem {
  const record = asRecord(value);
  const rawText = getString(record, "rawText", "raw_text") ?? "";
  const cleanedText = getNullableString(record, "cleanedText", "cleaned_text");
  const summary = getNullableString(record, "summary");
  const title = getString(record, "title") ?? makeTitle(summary ?? cleanedText ?? rawText);
  const actionItemCount = getNumber(record, "actionItemCount", "action_item_count") ?? 0;
  const suggestedActionItemCount =
    getNumber(record, "suggestedActionItemCount", "suggested_action_item_count") ??
    getArray(record, "actionItems", "action_items").filter(
      (item) => normalizeActionItem(item).status === "suggested",
    ).length;

  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    title,
    rawText,
    cleanedText,
    summary,
    captureSource: getString(record, "captureSource", "capture_source") ?? "quick_capture",
    createdAt: getString(record, "createdAt", "created_at") ?? fallbackNow,
    updatedAt: getString(record, "updatedAt", "updated_at") ?? fallbackNow,
    parseStatus: normalizeParseStatus(getString(record, "parseStatus", "parse_status")),
    reviewStatus: normalizeReviewStatus(getString(record, "reviewStatus", "review_status")),
    isArchived: getBoolean(record, "isArchived", "is_archived") ?? false,
    completedAt: getNullableString(record, "completedAt", "completed_at"),
    completionNote: getNullableString(record, "completionNote", "completion_note"),
    tags: getArray(record, "tags").map(normalizeTag),
    actionItemCount,
    suggestedActionItemCount,
  };
}

function normalizeNoteDetail(value: unknown): NoteDetail {
  const record = asRecord(value);
  const base = normalizeNoteListItem(value);
  const actionItems = getArray(record, "actionItems", "action_items").map((item) =>
    normalizeActionItem({ ...asRecord(item), noteTitle: base.title }),
  );

  return {
    ...base,
    actionItems,
    cardNotes: getArray(record, "cardNotes", "card_notes").map(normalizeCardNote),
    suggestedActionItemCount: actionItems.filter((item) => item.status === "suggested").length,
    actionItemCount: actionItems.length || base.actionItemCount,
    parseError: getNullableString(record, "parseError", "parse_error", "lastError", "last_error"),
    cleanedEdited: getBoolean(record, "cleanedEdited", "cleaned_edited") ?? false,
  };
}

function normalizeCardNote(value: unknown): CardNote {
  const record = asRecord(value);
  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    noteId: getString(record, "noteId", "note_id") ?? "",
    text: getString(record, "text") ?? "",
    createdAt: getString(record, "createdAt", "created_at") ?? fallbackNow,
  };
}

function normalizeTag(value: unknown): Tag {
  const record = asRecord(value);
  const nestedTag = asOptionalRecord(record.tag);
  const tagRecord = nestedTag ?? record;

  return {
    id: getString(tagRecord, "id") ?? crypto.randomUUID(),
    name: getString(tagRecord, "name") ?? "Untagged",
    kind: normalizeTagKind(getString(tagRecord, "kind")),
    source: normalizeTagSource(getString(record, "source")),
    confidence: getNumber(record, "confidence"),
    createdAt: getString(tagRecord, "createdAt", "created_at"),
  };
}

function normalizeActionItem(value: unknown): ActionItem {
  const record = asRecord(value);

  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    noteId: getString(record, "noteId", "note_id") ?? "",
    text: getString(record, "text") ?? "",
    owner: getNullableString(record, "owner"),
    dueDate: getNullableString(record, "dueDate", "due_date"),
    status: normalizeActionStatus(getString(record, "status")),
    source: getString(record, "source") ?? "parser",
    confidence: getNumber(record, "confidence"),
    noteTitle: getString(record, "noteTitle", "note_title"),
    followupState: normalizeFollowupState(getString(record, "followupState", "followup_state")),
    followupLane: getNullableString(record, "followupLane", "followup_lane"),
    completedAt: getNullableString(record, "completedAt", "completed_at"),
  };
}

function normalizeActionReviewItem(value: unknown): ActionReviewItem {
  const record = asRecord(value);
  const noteTitle = getString(record, "noteTitle", "note_title") ?? "Untitled note";

  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    noteId: getString(record, "noteId", "note_id") ?? "",
    noteTitle,
    text: getString(record, "text") ?? "",
    owner: getNullableString(record, "owner"),
    dueDate: getNullableString(record, "dueDate", "due_date"),
    confidence: getNumber(record, "confidence"),
    createdAt: getString(record, "createdAt", "created_at") ?? fallbackNow,
  };
}

function normalizeFollowupItem(value: unknown): FollowupItem {
  const record = asRecord(value);
  const status = normalizeActionStatus(getString(record, "status"));

  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    noteId: getString(record, "noteId", "note_id") ?? "",
    noteTitle: getString(record, "noteTitle", "note_title") ?? "Untitled note",
    text: getString(record, "text") ?? "",
    owner: getNullableString(record, "owner"),
    dueDate: getNullableString(record, "dueDate", "due_date"),
    status: status === "done" ? "done" : "accepted",
    source: getString(record, "source") ?? "parser",
    confidence: getNumber(record, "confidence"),
    followupState: normalizeFollowupState(getString(record, "followupState", "followup_state")),
    followupLane: getNullableString(record, "followupLane", "followup_lane"),
    tags: getArray(record, "tags").map(normalizeTag),
    createdAt: getString(record, "createdAt", "created_at") ?? fallbackNow,
    completedAt: getNullableString(record, "completedAt", "completed_at"),
  };
}

function normalizeSettings(value: unknown): AppSettings {
  const record = asRecord(value);

  return {
    hotkey: getString(record, "hotkey", "globalHotkey", "global_hotkey") ?? fallbackSettings.hotkey,
    parserTimeoutSeconds:
      getNumber(record, "parserTimeoutSeconds", "parser_timeout_seconds") ??
      fallbackSettings.parserTimeoutSeconds,
    parserMaxRetries:
      getNumber(record, "parserMaxRetries", "parser_max_retries") ?? fallbackSettings.parserMaxRetries,
    codexCommandPath:
      getString(record, "codexCommandPath", "codex_command_path") ?? fallbackSettings.codexCommandPath,
    linkedWorkspacePaths: normalizeStringArray(
      getArray(record, "linkedWorkspacePaths", "linked_workspace_paths"),
    ),
    selectedTheme: normalizeThemeId(
      getString(record, "selectedTheme", "selected_theme", "theme") ?? fallbackSettings.selectedTheme,
    ),
    launchAtStartup: getBoolean(record, "launchAtStartup", "launch_at_startup") ?? fallbackSettings.launchAtStartup,
    minimizeToTray: getBoolean(record, "minimizeToTray", "minimize_to_tray") ?? fallbackSettings.minimizeToTray,
  };
}

async function fallbackCommand<T>(command: string, args?: UnknownRecord): Promise<T> {
  switch (command) {
    case "save_capture_note": {
      const rawText = String(args?.rawText ?? "").trim();
      const now = new Date().toISOString();
      const note: NoteDetail = {
        id: `local-${Date.now()}`,
        title: makeTitle(rawText),
        rawText,
        cleanedText: null,
        summary: null,
        captureSource: "quick_capture",
        createdAt: now,
        updatedAt: now,
        parseStatus: "queued",
        reviewStatus: "none",
        tags: [],
        actionItemCount: 0,
        suggestedActionItemCount: 0,
        actionItems: [],
        cardNotes: [],
      };
      fallbackNotes = [note, ...fallbackNotes];
      return normalizeNoteListItem(note) as T;
    }
    case "list_inbox":
      return fallbackNotes
        .filter(
          (note) =>
            (Boolean((args?.filters as UnknownRecord | undefined)?.includeArchived) || !note.isArchived) &&
            (Boolean((args?.filters as UnknownRecord | undefined)?.includeCompleted) || !note.completedAt),
        )
        .slice(0, Number((args?.filters as UnknownRecord | undefined)?.limit ?? 200))
        .map(normalizeNoteListItem) as T;
    case "get_note": {
      const note = fallbackNotes.find((item) => item.id === args?.id) ?? fallbackNotes[0];
      return normalizeNoteDetail(note) as T;
    }
    case "retry_parse": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.parseStatus = "queued";
        note.parseError = null;
        note.updatedAt = new Date().toISOString();
      }
      return undefined as T;
    }
    case "retry_parse_with_feedback": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.parseStatus = "queued";
        note.parseError = null;
        note.summary = String(args?.feedback ?? "").trim() || note.summary;
        note.updatedAt = new Date().toISOString();
      }
      return undefined as T;
    }
    case "update_note_cleaned": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.title = String(args?.title ?? note.title);
        note.cleanedText = String(args?.cleanedText ?? note.cleanedText ?? "");
        note.summary = String(args?.summary ?? note.summary ?? "");
        note.cleanedEdited = true;
        note.updatedAt = new Date().toISOString();
      }
      return normalizeNoteDetail(note ?? fallbackNotes[0]) as T;
    }
    case "update_note_raw": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.rawText = String(args?.rawText ?? note.rawText);
        note.title = makeTitle(note.rawText);
        note.cleanedText = null;
        note.summary = null;
        note.parseStatus = "queued";
        note.parseError = null;
        note.cleanedEdited = false;
        note.tags = [];
        note.actionItems = note.actionItems.filter(
          (action) => !(action.source === "parser" && action.status === "suggested"),
        );
        note.actionItemCount = note.actionItems.length;
        note.suggestedActionItemCount = note.actionItems.filter((action) => action.status === "suggested").length;
        note.updatedAt = new Date().toISOString();
      }
      return normalizeNoteDetail(note ?? fallbackNotes[0]) as T;
    }
    case "delete_note": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.isArchived = true;
        note.updatedAt = new Date().toISOString();
      }
      return undefined as T;
    }
    case "complete_note": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.completedAt = new Date().toISOString();
        note.completionNote = typeof args?.completionNote === "string" ? args.completionNote : null;
        note.isArchived = false;
        note.updatedAt = note.completedAt;
      }
      return undefined as T;
    }
    case "add_card_note": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (!note) {
        throw new Error("note not found");
      }
      const text = String(args?.text ?? "").trim();
      note.cardNotes ??= [];
      note.cardNotes.push({
        id: crypto.randomUUID(),
        noteId: note.id,
        text,
        createdAt: new Date().toISOString(),
      });
      note.updatedAt = new Date().toISOString();
      return normalizeNoteDetail(note) as T;
    }
    case "reopen_note": {
      const note = fallbackNotes.find((item) => item.id === args?.noteId);
      if (note) {
        note.completedAt = null;
        note.updatedAt = new Date().toISOString();
      }
      return undefined as T;
    }
    case "restore_note": {
      const noteId = String(args?.noteId ?? "");
      const note = fallbackNotes.find((item) => item.id === noteId);

      if (!note) {
        throw new Error("note not found");
      }

      note.isArchived = false;
      note.updatedAt = new Date().toISOString();
      return undefined as T;
    }
    case "permanently_delete_note": {
      const noteId = String(args?.noteId ?? "");
      const index = fallbackNotes.findIndex((item) => item.id === noteId);

      if (index < 0) {
        throw new Error("note not found");
      }

      if (!fallbackNotes[index].isArchived) {
        throw new Error("note must be archived before permanent delete");
      }

      fallbackNotes.splice(index, 1);
      return undefined as T;
    }
    case "accept_action_item":
    case "dismiss_action_item": {
      const actionItemId = String(args?.actionId ?? "");
      const status = command === "accept_action_item" ? "accepted" : "dismissed";
      const note = fallbackNotes.find((note) => note.actionItems.some((item) => item.id === actionItemId));
      const action = note?.actionItems.find((item) => item.id === actionItemId);
      if (note && action && action.status === "suggested") {
        action.status = status;
        if (status === "accepted") {
          action.followupState = action.followupState ?? "open";
        }
        note.suggestedActionItemCount = note.actionItems.filter((item) => item.status === "suggested").length;
        if (note.suggestedActionItemCount === 0) {
          note.reviewStatus = "reviewed";
        }
        note.updatedAt = new Date().toISOString();
      }
      return undefined as T;
    }
    case "complete_action_item": {
      const actionItemId = String(args?.actionId ?? "");
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === actionItemId);
      if (action && action.status === "accepted") {
        action.status = "done";
        action.completedAt = new Date().toISOString();
      }
      return undefined as T;
    }
    case "reopen_action_item": {
      const actionItemId = String(args?.actionId ?? "");
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === actionItemId);
      if (action && action.status === "done") {
        action.status = "accepted";
        action.completedAt = null;
      }
      return undefined as T;
    }
    case "list_followups":
      return fallbackNotes
        .filter((note) => !note.isArchived)
        .flatMap((note) =>
          note.actionItems
            .filter((action) => action.status === "accepted" || action.status === "done")
            .map((action) =>
              normalizeFollowupItem({
                ...action,
                noteTitle: note.title,
                tags: note.tags,
                createdAt: note.createdAt,
              }),
            ),
        )
        .slice(0, Number(args?.limit ?? 200)) as T;
    case "create_manual_followup": {
      const input = asRecord(args?.input);
      const noteId = getString(input, "noteId", "note_id") ?? "";
      const note = fallbackNotes.find((item) => item.id === noteId);
      const text = String(input.text ?? "").trim();
      if (!note) {
        throw new Error("note not found");
      }
      if (!text) {
        throw new Error("follow-up text is required");
      }

      const lane = getNullableString(input, "laneOverride", "lane_override", "lane")?.trim() || null;
      const action: ActionItem = {
        id: `manual-${Date.now()}`,
        noteId: note.id,
        text,
        owner: null,
        dueDate: null,
        status: "accepted",
        source: "user",
        confidence: null,
        noteTitle: note.title,
        followupState: "open",
        followupLane: lane,
      };
      note.actionItems = [...note.actionItems, action];
      note.actionItemCount = note.actionItems.length;
      note.updatedAt = new Date().toISOString();
      return normalizeActionItem(action) as T;
    }
    case "update_followup_state": {
      const input = asRecord(args?.input);
      const actionItemId = getString(input, "id", "actionId", "action_id") ?? "";
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === actionItemId);
      if (!action || action.status !== "accepted") {
        throw new Error("action is not an active follow-up");
      }
      action.followupState = normalizeFollowupState(getString(input, "state")) ?? "open";
      return undefined as T;
    }
    case "update_followup_lane": {
      const input = asRecord(args?.input);
      const actionItemId = getString(input, "id", "actionId", "action_id") ?? "";
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === actionItemId);
      if (!action || (action.status !== "accepted" && action.status !== "done")) {
        throw new Error("action is not an active follow-up");
      }
      action.followupLane = getNullableString(input, "laneOverride", "lane_override", "lane")?.trim() || null;
      return undefined as T;
    }
    case "list_suggested_actions":
      return fallbackNotes
        .flatMap((note) =>
          note.actionItems
            .filter((action) => action.status === "suggested")
            .map((action) =>
              normalizeActionReviewItem({
                ...action,
                noteTitle: note.title,
                createdAt: note.createdAt,
              }),
            ),
        )
        .slice(0, Number(args?.limit ?? 100)) as T;
    case "get_settings":
      return normalizeSettings(fallbackSettings) as T;
    case "save_settings":
      fallbackSettings = normalizeSettings(args?.settings);
      return normalizeSettings(fallbackSettings) as T;
    case "hide_quick_capture":
      return undefined as T;
    default:
      throw new Error(`Unsupported fallback command: ${command}`);
  }
}

function asRecord(value: unknown): UnknownRecord {
  return asOptionalRecord(value) ?? {};
}

function asOptionalRecord(value: unknown): UnknownRecord | null {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as UnknownRecord) : null;
}

function getString(record: UnknownRecord, ...keys: string[]): string | undefined {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string") {
      return value;
    }
  }
  return undefined;
}

function getNullableString(record: UnknownRecord, ...keys: string[]): string | null {
  for (const key of keys) {
    const value = record[key];
    if (value === null) {
      return null;
    }
    if (typeof value === "string") {
      return value;
    }
  }
  return null;
}

function getNumber(record: UnknownRecord, ...keys: string[]): number | undefined {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "number" && Number.isFinite(value)) {
      return value;
    }
  }
  return undefined;
}

function getBoolean(record: UnknownRecord, ...keys: string[]): boolean | undefined {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "boolean") {
      return value;
    }
  }
  return undefined;
}

function getArray(record: UnknownRecord, ...keys: string[]): unknown[] {
  for (const key of keys) {
    const value = record[key];
    if (Array.isArray(value)) {
      return value;
    }
  }
  return [];
}

function normalizeStringArray(values: unknown[]): string[] {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const value of values) {
    if (typeof value !== "string") {
      continue;
    }

    const trimmed = value.trim();
    if (!trimmed || seen.has(trimmed)) {
      continue;
    }

    seen.add(trimmed);
    normalized.push(trimmed);
  }

  return normalized;
}

function makeTitle(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return "Untitled note";
  }
  const firstLine = trimmed.split(/\r?\n/, 1)[0];
  return firstLine.length > 64 ? `${firstLine.slice(0, 61)}...` : firstLine;
}

function normalizeParseStatus(value: string | undefined): ParseStatus {
  return value === "queued" || value === "parsing" || value === "parsed" || value === "failed" ? value : "queued";
}

function normalizeReviewStatus(value: string | undefined): ReviewStatus {
  return value === "none" || value === "needs_review" || value === "reviewed" ? value : "none";
}

function normalizeTagKind(value: string | undefined): Tag["kind"] {
  return value === "person" ||
    value === "project" ||
    value === "topic" ||
    value === "urgency" ||
    value === "category" ||
    value === "custom"
    ? value
    : "custom";
}

function normalizeTagSource(value: string | undefined): Tag["source"] {
  return value === "ai" || value === "user" ? value : undefined;
}

function normalizeActionStatus(value: string | undefined): ActionItem["status"] {
  return value === "suggested" || value === "accepted" || value === "dismissed" || value === "done"
    ? value
    : "suggested";
}

function normalizeFollowupState(value: string | undefined): FollowupState | null {
  return value === "open" || value === "waiting" || value === "blocked" ? value : null;
}

function normalizeThemeId(value: string): string {
  return value === "dark" ? "dark-compact" : value;
}
