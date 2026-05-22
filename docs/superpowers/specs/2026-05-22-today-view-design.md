# Today View Design

## Source Design

This design is derived from `work-notes-handoff.zip`, specifically:

- `work-notes/project/Work Notes Redesign v3.html`, artboard `03 · Today · Memphis`.
- `work-notes/project/today-people-v2.jsx`, `TodayView`.
- `work-notes/project/app-v2-extra.css`, `.today-*` styles.
- `work-notes/project/data-v2.jsx`, sample note and action shape.

The production implementation should follow the current Svelte/Tauri architecture and semantic theme variables rather than copying React prototype internals.

## Goal

Make the sidebar `Today` item real. Selecting it should show a compact operational dashboard for current work: suggested actions due today or overdue, notes captured today, and a simple work-week activity strip.

## Scope

In scope:

- Add `today` as a primary view in the existing app shell.
- Replace the inert `Today` sidebar link with an active navigation button.
- Load non-archived inbox notes and suggested actions when entering Today.
- Show due-today/overdue suggested actions with owner, due label, and source note title.
- Allow accepting suggested actions directly from the Today due list.
- Show notes captured today.
- Show a Monday-Friday activity strip based on captured notes and due suggested actions.
- Let action and note rows open their source note in the normal Inbox detail flow.

Out of scope:

- New backend commands or database schema.
- Editing action text, owner, or due date.
- Accepted follow-up board behavior.
- Calendar/reminder behavior.
- People or Tags view implementation.
- Keyboard shortcut overlay behavior.

## Product Behavior

- `Inbox` remains the default view.
- `Today` is a full content surface, not another list/detail split.
- Due actions are suggested actions with a parseable `dueDate` on or before the current local day.
- Captured-today notes are non-archived notes whose `createdAt` falls on the current local day.
- If there are no due actions or no captured notes, the relevant section shows a compact empty state.
- Accepting a due action removes it from Today after the existing suggested-action refresh.
- Clicking an action row or captured note opens that note in Inbox, preserving the established note detail workflow.

## UI Shape

The Today view uses the existing shell sidebar and replaces the main workspace with one scrollable dashboard:

- Header: `Today`, current weekday/month/day, and summary counts.
- Section 1: `Due today`, rows styled similarly to the Actions list but denser.
- Section 2: `Captured today`, note rows with status dot, title, capture time, tags, and suggested-action count.
- Section 3: `This week`, five compact weekday tiles with activity dots/counts.

Use semantic CSS variables such as `--color-surface-1`, `--color-surface-2`, `--color-surface-input`, `--color-border-default`, `--color-text-primary`, `--color-text-muted`, `--color-accent-primary`, `--color-status-warning`, and `--color-status-error`.

## Data Flow

Frontend-only flow:

1. `AppShell` emits `navigate: "today"` from the Today sidebar button.
2. `src/routes/+page.svelte` calls `workNotes.showToday()`.
3. `src/lib/stores/inbox.ts` sets `viewMode` to `today`, clears archived filtering, refreshes non-archived inbox notes, and refreshes suggested actions.
4. `TodayView.svelte` derives due actions, captured notes, summary counts, and week activity from the existing store data.
5. Accepting a due action calls `workNotes.acceptSuggestedAction()`.
6. Opening a source note calls `workNotes.showInbox()` and then `workNotes.selectNote(noteId)`.

## Error Handling

- Inbox and suggested-action load failures use the existing store error banner.
- Accept failures keep the action visible and show the existing store error.
- Unparseable due dates are ignored for due-today and week due counts instead of throwing.
- Source note lookup failures reuse the existing stale-selection handling from `selectNote`.

## Testing

Frontend tests should cover:

- Today helper functions identify due/overdue actions, captured-today notes, and work-week counts.
- AppShell marks Today active and emits Today navigation.
- The store enters Today view and refreshes inbox plus suggested actions.
- TodayView renders due actions, captured notes, and week activity.
- TodayView dispatches open-source-note and accept events without row-click leakage from the accept button.

Full verification:

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```
