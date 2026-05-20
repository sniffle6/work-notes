# Work Notes Design

## Purpose

Work Notes is a Windows-only local desktop note app optimized for drive-by information dumps. The app must open instantly, capture raw notes with almost no friction, and then organize those notes automatically in the background using the user's local Codex subscription through the Codex CLI.

The highest priority is preserving speed and trust at capture time. Raw notes are saved immediately and never overwritten. AI-generated cleanup, tags, summaries, and action extraction improve scanability, but they remain derived data that can be reviewed, retried, or corrected.

## Product Workflow

The app runs in the Windows tray and registers a global hotkey. Pressing the hotkey opens a compact dark quick-capture window anchored to the bottom-right of the screen. The note field is focused immediately.

Quick-capture keyboard behavior:

- `Enter` saves and closes.
- `Shift+Enter` inserts a newline.
- `Esc` closes the capture window and preserves the draft.

Saving writes the raw note to SQLite immediately. The UI does not wait for parsing. After save, the note lands in the inbox and a background parse job is queued automatically.

The main app is inbox-first. Notes are ordered by recency and can be filtered by tags, parse status, project/topic, person, review state, and search text.

Primary screens:

- Quick Capture: compact bottom-right transient window.
- Inbox: dense default view for captured notes, filters, tags, search, and parse status.
- Note Detail: raw note, cleaned note, summary, tags, action items, parse status, and parse history.
- Review Queue: extracted actions, due dates, and owner assumptions that need confirmation.
- Settings: hotkey, startup/tray behavior, selected theme, parser timeout, Codex command path, and parser health.

## Technology Stack

The app uses:

- Tauri v2 for the Windows desktop shell.
- Svelte and TypeScript for the frontend.
- Rust for native commands, persistence, background worker ownership, and process execution.
- SQLite as the source of truth.
- SQLite FTS5 for full-text search over raw and cleaned note content.
- Local `codex exec` for automatic background parsing through the user's Codex subscription.

This stack is intentionally local-first. The app does not use OpenAI API keys and does not incur OpenAI API token billing.

## Data Model

### `notes`

- `id`
- `raw_text`
- `cleaned_text`
- `summary`
- `created_at`
- `updated_at`
- `capture_source`
- `parse_status`: `queued | parsing | parsed | failed`
- `review_status`: `none | needs_review | reviewed`
- `is_archived`

`raw_text` is immutable after capture except for explicit user correction flows added later. Parser output must never overwrite it.

### `tags`

- `id`
- `name`
- `kind`: `person | project | topic | urgency | category | custom`
- `created_at`

### `note_tags`

- `note_id`
- `tag_id`
- `source`: `ai | user`
- `confidence`

AI-generated tags are auto-applied but remain distinguishable from user-created tags.

### `action_items`

- `id`
- `note_id`
- `text`
- `owner`
- `due_date`
- `status`: `suggested | accepted | dismissed | done`
- `source`
- `confidence`

Action items start as `suggested`. The parser must not silently mark an action as accepted or done.

### `parse_jobs`

- `id`
- `note_id`
- `status`: `queued | parsing | parsed | failed`
- `attempt_count`
- `last_error`
- `created_at`
- `started_at`
- `finished_at`

### `parse_runs`

- `id`
- `note_id`
- `provider`
- `prompt_version`
- `raw_response`
- `parsed_json`
- `created_at`

Parse runs are retained for debugging, audit, and prompt iteration.

### `settings`

Settings include app startup behavior, tray behavior, global hotkey, selected theme, parser timeout, retry limits, and Codex command path.

## Service Boundaries

The app should keep UI, persistence, parsing, and window ownership separated.

- `CaptureService`: validates and saves raw notes quickly.
- `DraftService`: preserves unsaved quick-capture text.
- `NoteRepository`: owns note persistence, note queries, and FTS write synchronization.
- `SearchService`: wraps SQLite FTS5 search and filter queries.
- `TagService`: normalizes, creates, and applies tags.
- `ActionItemService`: stores, confirms, dismisses, and completes extracted actions.
- `ParseQueue`: owns parse job lifecycle and background worker scheduling.
- `CodexParserProvider`: the only module that knows how to invoke local `codex exec`.
- `ParserResultApplier`: validates and applies parser output according to trust rules.
- `ThemeService`: loads semantic theme definitions and exposes CSS custom properties.
- `WindowService`: owns tray integration, global hotkey registration, quick-capture positioning, and show/hide behavior.

Svelte components call narrow Tauri commands. Components must not know about SQLite, Codex CLI, filesystem paths, or background worker details.

## Parser Pipeline

Parsing is automatic and decoupled from capture.

1. `CaptureService` saves `raw_text` immediately.
2. `ParseQueue` creates a queued `parse_jobs` row.
3. A background worker claims one queued job at a time by default.
4. The worker marks the job `parsing`.
5. `CodexParserProvider` invokes local `codex exec`.
6. The response is validated against a JSON schema.
7. `parse_runs` stores the raw response and parsed JSON.
8. `ParserResultApplier` updates the note with derived data.
9. The job becomes `parsed` or `failed`.

Initial command shape:

```powershell
codex exec `
  --ephemeral `
  --skip-git-repo-check `
  --output-schema parse-note.schema.json `
  -o parse-result.json `
  -
```

The app pipes parser instructions and note content to stdin. The command writes the final response to a temporary output file. The app reads that file, validates it, stores a parse run, and deletes temporary files.

Parser output schema:

```json
{
  "cleanedText": "Mike said the deploy moved to Friday. Check the config flag before QA gets the build.",
  "summary": "Deploy moved to Friday; verify config flag before QA build.",
  "tags": [
    { "name": "Mike", "kind": "person", "confidence": 0.92 },
    { "name": "deploy", "kind": "topic", "confidence": 0.88 },
    { "name": "QA", "kind": "topic", "confidence": 0.84 }
  ],
  "actionItems": [
    {
      "text": "Check the config flag before QA gets the build.",
      "owner": null,
      "dueDate": null,
      "confidence": 0.83,
      "requiresReview": true
    }
  ]
}
```

Parser trust rules:

- Raw note text is never overwritten.
- Cleaned note text is auto-applied.
- Summary is auto-applied.
- Tags are auto-applied.
- Suggested action items are created as `suggested`.
- Dates, owners, and obligation assumptions require review.
- Parser failure does not affect saved notes.
- Invalid JSON marks the parse job failed.
- Failed jobs keep `last_error`.
- Users can retry failed parses.

## Theme System

The default theme is Dark Compact. It should feel like a quiet, dense Windows utility rather than a marketing page or decorative notebook.

Themes are developer-defined files. Components consume semantic CSS variables only. Adding a new theme should require adding a theme definition and registering it, not editing component styles across the app.

Required semantic theme tokens:

- `app.bg`
- `surface.1`
- `surface.2`
- `surface.input`
- `border.default`
- `border.strong`
- `text.primary`
- `text.muted`
- `accent.primary`
- `accent.hot`
- `status.success`
- `status.warning`
- `status.error`

Initial Dark Compact palette:

- `app.bg`: `#11151c`
- `surface.1`: `#1b212b`
- `surface.2`: `#202833`
- `surface.input`: `#111720`
- `border.default`: `#334052`
- `text.primary`: `#edf4fb`
- `text.muted`: `#93a3b7`
- `accent.primary`: `#2f6f7a`
- `accent.hot`: `#6fc7bd`

No Svelte component should hardcode palette colors directly. Component styles should use theme variables such as `--color-surface-2`, `--color-text-primary`, and `--color-accent-primary`.

## Error Handling

Capture errors must be visible and immediate because they threaten data safety. If SQLite write fails, the quick-capture window should keep the draft and show a concise failure state.

Parser errors are non-blocking. They appear in the inbox and note detail as `Parse failed` with retry access. Successful parsing should not create intrusive notifications.

Codex parser failure modes to handle:

- `codex` command missing.
- Codex CLI not logged in.
- Command timeout.
- Non-zero exit.
- Output file missing.
- Invalid JSON.
- JSON schema validation failure.

## SOLID, KISS, DRY, And Hollywood Principle

Single responsibility:

- Each service owns one behavior area: capture, notes, search, tags, actions, parsing, theme, or windows.

Open/closed:

- New parser providers can be added behind the parser-provider boundary later, but v1 implements only local Codex CLI.
- New themes can be added through theme definitions without component rewrites.

Liskov/interface segregation:

- Parser providers should expose a small parse method that accepts raw note input and returns a validated parser result.
- UI commands should expose workflow-level actions, not raw database primitives.

Dependency inversion:

- Worker logic depends on a parser-provider interface, not direct process execution.
- UI depends on Tauri commands, not repositories or CLI details.

KISS:

- Inbox-first organization for v1.
- One parser worker at a time by default.
- Developer-defined themes for v1, no in-app theme editor.
- SQLite is the only source of truth.

DRY:

- All theme colors come from one token system.
- Parser result application rules live in one service.
- Search query construction lives behind `SearchService`.

Hollywood principle:

- Capture does not call parsing directly.
- Capture publishes saved notes and queued work.
- The parser worker claims jobs later.
- UI observes state and invokes workflow commands, rather than orchestrating internals.

## Testing And Verification

Rust unit tests:

- `CaptureService` saves raw notes and preserves drafts on write failure.
- `NoteRepository` creates, reads, updates, and searches notes.
- FTS index updates include raw and cleaned text.
- `ParseQueue` claims one job at a time and records failures.
- `ParserResultApplier` auto-applies cleaned text, summary, and tags.
- `ParserResultApplier` creates suggested action items without accepting them.
- `CodexParserProvider` handles missing command, timeout, non-zero exit, missing output, invalid JSON, and schema errors.

Frontend tests:

- Quick capture keyboard behavior.
- Inbox filtering by tag, parse status, review status, and search.
- Note detail shows raw and cleaned text.
- Review queue confirms and dismisses suggested actions.
- Theme variables apply to key surfaces without raw palette colors in components.

Integration tests:

- Use a fake parser provider before invoking real Codex CLI.
- Verify automatic parse job creation after note save.
- Verify parser failure leaves raw note intact.
- Verify successful parse updates cleaned text, summary, tags, and suggested actions.

Manual Windows verification:

- Tray app starts.
- Global hotkey opens quick capture.
- Quick capture appears bottom-right and focused.
- `Enter`, `Shift+Enter`, and `Esc` behave correctly.
- Note save is immediate.
- Parser runs automatically.
- Failed parser does not damage the note.
- Inbox search finds raw and cleaned text.
- Theme defaults to Dark Compact.

## Out Of Scope For V1

- OpenAI API integration.
- API key management.
- Cloud sync.
- Team/multi-user accounts.
- Mobile app.
- In-app theme editor.
- Calendar/task-manager writeback.
- Automatic acceptance of action items or due dates.
- Project/person boards as the primary organization model.
