# Architecture Context

Work Notes is an inbox-first Windows desktop app for fast note capture and background cleanup. Tauri owns the native app shell, Rust owns persistence and parsing behavior, and Svelte owns the interface.

## Runtime Flow

1. The main window and hidden `quick-capture` window are declared in `src-tauri/tauri.conf.json`.
2. Tauri starts in `src-tauri/src/lib.rs`, opens SQLite, builds `AppState`, starts the parser worker, initializes windowing, and registers commands.
3. `src/routes/+page.svelte` detects the current Tauri window label and renders either the main app or the quick-capture surface.
4. Components emit UI events into `src/lib/stores/inbox.ts`.
5. The store calls `src/lib/api.ts`, which invokes Tauri commands or browser fallback data during non-Tauri frontend smoke checks.
6. Rust commands call services, services call repositories, repositories write SQLite.
7. Capture creates a raw note and queued parse job in one persistence path.
8. The native quick-capture window saves through `captureRawNote`, emits `work-notes:note-captured`, and lets the main window refresh and select the saved note separately.
9. The parser worker claims one queued job at a time and applies validated parser output as derived data.

## Frontend Ownership

- `src/routes/+layout.ts`: disables SSR for the Tauri SPA.
- `src/routes/+page.svelte`: top-level route, window-label branching, quick-capture focus event subscription, note-captured event emission.
- `src/lib/api.ts`: the only frontend module that invokes Tauri commands. It also normalizes DTOs and provides browser fallback data.
- `src/lib/types.ts`: frontend note, tag, action item, filter, and settings types.
- `src/lib/stores/inbox.ts`: central workflow store for inbox loading, note selection, capture save, card notes, parse retry, note completion and resolution, delete/archive, action status changes, filters, and settings.
- `src/lib/stores/filters.ts`: filter creation and client-side matching helpers used by tests and fallback paths.
- `src/lib/events.ts`: named frontend/native event contract.
- `src/lib/markdown.ts`: small Markdown renderer for parser-cleaned notes.
- `src/lib/theme/`: semantic theme token system. Component CSS should consume variables generated here.
- `src/lib/components/`: visual surfaces. Keep persistence and parser decisions out of components.

## Main Components

- `AppShell.svelte`: app frame, sidebar navigation for Inbox, Done, Archive, and workflow views, status metrics, and quick capture slot.
- `InboxList.svelte`: inbox rows, Notes/Actions modes, search, status/tag filters, selection, and suggested-action counts.
- `today.ts`: calendar projection module that turns each action item into linked captured, due, and completed occurrences, including same-day merging and month counts.
- `TodayView.svelte`: month calendar that renders projected action-item lifecycle occurrences; defaults to the current day and exposes accept, complete, and reopen controls.
- `NoteDetail.svelte`: title, raw/Markdown cleaned text, summary, user-added card notes, completion resolution, tags, suggested actions, parse retry, reparse feedback, delete event.
- `MarkdownView.svelte`: renders parser-provided Markdown safely for note detail.
- `QuickCapturePanel.svelte`: compact note entry; preserves `Enter`, `Shift+Enter`, and `Esc` behavior.
- `ReviewQueue.svelte`: standalone accept/dismiss surface for suggested action items; current main flow reviews actions through the inbox Actions mode and note detail.
- `SettingsView.svelte`: hotkey, parser timeout, Codex command path, startup/tray, theme settings, and settings-save errors.
- `StatusBadge.svelte`: shared visual status indicator.

## Backend Ownership

- `src-tauri/src/lib.rs`: Tauri entrypoint, plugin setup, database path setup, `AppState`, parse worker startup, windowing startup, command registration.
- `src-tauri/src/app_state.rs`: shared database, repositories, settings service, draft service, parse queue, parser provider config.
- `src-tauri/src/domain.rs`: ID wrappers, notes, tags, action items, parse jobs, parse runs, and string-backed enums.
- `src-tauri/src/commands.rs`: Tauri command functions, DTOs, input validation, and command error conversion.
- `src-tauri/src/db/`: SQLite connection wrapper and migrations.
- `src-tauri/src/repositories/`: direct SQLite ownership.
- `src-tauri/src/services/`: workflow behavior above repositories.
- `src-tauri/src/parser/`: parser contracts, prompts, schema validation, Codex provider, result applier.
- `src-tauri/src/windowing/`: main window, quick capture, tray, hotkey, and positioning behavior.

## SQLite Model

SQLite is opened from the app data directory described in `docs/development.md`:

```text
%APPDATA%\aweber\Work Notes\data\work-notes.sqlite3
```

Core tables:

- `notes`: title, raw and derived note fields, parse status, review status, archive flag, and note completion timestamp.
- `card_notes`: timestamped user-added context attached to a note.
- `notes_fts`: FTS5 index over raw text, cleaned text, and summary.
- `tags` and `note_tags`: normalized tags and note assignments.
- `action_items`: parser-suggested or user-managed actions, including open/done lifecycle state and the timestamp recorded when work is completed.
- `parse_jobs`: queued, parsing, parsed, and failed parse work.
- `parse_runs`: raw provider response and normalized parsed JSON for audit/debugging.
- `settings`: persisted app and parser settings.

## Command Contract

Frontend command names currently used from `src/lib/api.ts`:

```text
save_capture_note
list_inbox
get_note
retry_parse
retry_parse_with_feedback
delete_note
complete_note
add_card_note
reopen_note
accept_action_item
dismiss_action_item
get_settings
save_settings
hide_quick_capture
```

Rust DTOs serialize with camelCase for frontend ergonomics. Inbox rows include `actionItemCount` and `suggestedActionItemCount` so the frontend can render action counts and the Actions mode without fetching every note detail. Parser JSON also uses camelCase. Backend domain enums serialize as snake_case status strings.

`delete_note` is a soft-delete/archive command: it marks the note archived through `is_archived` rather than removing the SQLite row.
`complete_note` records `completed_at` plus an optional completion resolution and removes the note from the active inbox without archiving or deleting it.
`add_card_note` appends timestamped user context to an existing note without changing its parser-derived fields.
`reopen_note` clears `completed_at` and returns a completed note to the active inbox.

## Native Runtime

- Main window label: `main`.
- Quick-capture window label: `quick-capture`.
- Quick capture defaults: `560x260`, hidden at startup, undecorated, skipped from taskbar.
- Default global shortcut: `Ctrl+Shift+Space`.
- Quick-capture focus event: `quick-capture:focus-note-textarea`.
- Frontend note-captured event: `work-notes:note-captured`.
- Tray menu entries: `Open`, `Quick Note`, `Quit`.

