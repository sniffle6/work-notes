import { invoke } from "@tauri-apps/api/core";

import type {
  ActionItem,
  AppSettings,
  InboxFilters,
  NoteDetail,
  NoteListItem,
  ParseStatus,
  ReviewStatus,
  Tag,
} from "$lib/types";

type UnknownRecord = Record<string, unknown>;

const fallbackNow = "2026-05-20T13:42:00.000Z";

let fallbackNotes: NoteDetail[] = [
  {
    id: "n-1024",
    title: "Kiosk 7 telemetry IDs",
    rawText: "Maya: bring serial list into the Tuesday sync and flag missing asset tags before the vendor call.",
    cleanedText:
      "Maya needs the kiosk 7 serial list brought into the Tuesday sync, with missing asset tags flagged before the vendor call.",
    summary: "Bring kiosk 7 serials to Tuesday sync and flag missing asset tags.",
    captureSource: "quick_capture",
    createdAt: "2026-05-20T13:42:00.000Z",
    updatedAt: "2026-05-20T13:45:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [
      { id: "tag-maya", name: "Maya", kind: "person", source: "ai", confidence: 0.94 },
      { id: "tag-kiosk", name: "Kiosk", kind: "topic", source: "ai", confidence: 0.88 },
    ],
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    actionItems: [
      {
        id: "a-1024",
        noteId: "n-1024",
        text: "Bring serial list into the Tuesday sync.",
        owner: "Maya",
        dueDate: null,
        status: "suggested",
        source: "parser",
        confidence: 0.82,
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
    createdAt: "2026-05-20T13:18:00.000Z",
    updatedAt: "2026-05-20T13:18:00.000Z",
    parseStatus: "failed",
    reviewStatus: "none",
    tags: [{ id: "tag-finance", name: "Finance", kind: "topic", source: "ai", confidence: 0.91 }],
    actionItemCount: 0,
    suggestedActionItemCount: 0,
    actionItems: [],
    parseError: "Codex parser timed out.",
  },
  {
    id: "n-1022",
    title: "Visitor badge printer",
    rawText: "Rina: front desk can print test labels, but badge names are shifted one line on real entries.",
    cleanedText: "The visitor badge printer shifts names down one line on real entries, though test labels print correctly.",
    summary: "Badge printer alignment fails for real visitor entries.",
    captureSource: "quick_capture",
    createdAt: "2026-05-19T18:30:00.000Z",
    updatedAt: "2026-05-19T18:35:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "reviewed",
    tags: [{ id: "tag-front-desk", name: "Front desk", kind: "project", source: "ai", confidence: 0.86 }],
    actionItemCount: 1,
    suggestedActionItemCount: 0,
    actionItems: [
      {
        id: "a-1022",
        noteId: "n-1022",
        text: "Check badge printer template alignment for real entries.",
        owner: "Rina",
        dueDate: null,
        status: "accepted",
        source: "parser",
        confidence: 0.78,
        noteTitle: "Visitor badge printer",
      },
    ],
  },
];

let fallbackSettings: AppSettings = {
  hotkey: "Ctrl+Shift+Space",
  parserTimeoutSeconds: 45,
  parserMaxRetries: 3,
  codexCommandPath: "codex",
  selectedTheme: "dark-compact",
  launchAtStartup: true,
  minimizeToTray: true,
};

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invokeCommand<T>(command: string, args?: UnknownRecord): Promise<T> {
  if (!isTauriRuntime()) {
    return fallbackCommand<T>(command, args);
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

export async function acceptActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("accept_action_item", { actionId: actionItemId });
}

export async function dismissActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("dismiss_action_item", { actionId: actionItemId });
}

export async function getSettings(): Promise<AppSettings> {
  const settings = await invokeCommand<unknown>("get_settings");
  return normalizeSettings(settings);
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  const saved = await invokeCommand<unknown>("save_settings", { settings: toBackendSettings(settings) });
  return normalizeSettings(saved);
}

export const api = {
  saveCaptureNote,
  listInbox,
  getNote,
  retryParse,
  acceptActionItem,
  dismissActionItem,
  getSettings,
  saveSettings,
};

function toBackendFilters(filters: InboxFilters): UnknownRecord {
  return {
    parseStatus: filters.parseStatuses[0] ?? null,
    reviewStatus: filters.reviewStatuses[0] ?? null,
    tagIds: filters.tagIds,
    query: filters.search.trim() || null,
    includeArchived: Boolean(filters.includeArchived),
  };
}

function toBackendSettings(settings: AppSettings): UnknownRecord {
  return {
    hotkey: settings.hotkey,
    globalHotkey: settings.hotkey,
    parserTimeoutSeconds: settings.parserTimeoutSeconds,
    parserMaxRetries: settings.parserMaxRetries ?? 3,
    codexCommandPath: settings.codexCommandPath,
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
    suggestedActionItemCount: actionItems.filter((item) => item.status === "suggested").length,
    actionItemCount: actionItems.length || base.actionItemCount,
    parseError: getNullableString(record, "parseError", "parse_error", "lastError", "last_error"),
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
      };
      fallbackNotes = [note, ...fallbackNotes];
      return normalizeNoteListItem(note) as T;
    }
    case "list_inbox":
      return fallbackNotes.map(normalizeNoteListItem) as T;
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
    case "accept_action_item":
    case "dismiss_action_item": {
      const actionItemId = String(args?.actionId ?? "");
      const status = command === "accept_action_item" ? "accepted" : "dismissed";
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === actionItemId);
      if (action) {
        action.status = status;
      }
      return undefined as T;
    }
    case "get_settings":
      return normalizeSettings(fallbackSettings) as T;
    case "save_settings":
      fallbackSettings = normalizeSettings(args?.settings);
      return normalizeSettings(fallbackSettings) as T;
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

function normalizeThemeId(value: string): string {
  return value === "dark" ? "dark-compact" : value;
}
