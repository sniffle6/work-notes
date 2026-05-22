# Actions View Design

## Source Design

This design is derived from `work-notes-handoff.zip`, specifically:

- `work-notes/README.md`, which identifies `project/Work Notes Redesign v3.html` as the primary design.
- `project/Work Notes Redesign v3.html`, artboard `02 · Actions mode · Memphis`.
- `project/inbox-screen-v2.jsx`, `ActionsList`.
- `project/app-v2.css`, the `Actions mode` styles.
- `project/data-v2.jsx`, which shows suggested actions with owners and due dates.

The production implementation should follow the existing Svelte/Tauri architecture and theme variables rather than copying the React prototype internals.

## Goal

Make the sidebar `Actions` item real. Selecting it should switch the main list column from notes to action rows, grouped by due bucket, while keeping the note detail panel focused on the source note.

## Scope

In scope:

- Add an Actions primary view in the existing app shell.
- Show suggested actions as primary list rows.
- Group suggested actions by due bucket: `Overdue`, `Today`, `This week`, `Later`, `No date`.
- Provide search over action text, owner, due date, and source note title.
- Clicking an action row selects its source note in the existing detail panel.
- Provide quick Accept and Dismiss controls on action rows.
- Keep the current ReviewQueue behavior outside Actions view.
- Preserve the current NoteDetail action lifecycle controls.

Out of scope:

- Today view.
- People view.
- Tags view.
- Keyboard shortcut overlay.
- Reordering actions.
- Editing action text, owner, or due date.
- Listing accepted or completed actions in the Actions primary list.
- Parser prompt, parser schema, or Codex CLI changes.

## Product Behavior

- `Inbox` remains the default primary view.
- `Archive` keeps showing archived notes only.
- `Actions` shows non-archived suggested actions only. This matches the existing `list_suggested_actions` backend query, which excludes archived notes.
- The Actions sidebar count should show the number of suggested actions.
- When entering Actions view, the store should refresh suggested actions. If no note is selected, or if the selected note has no visible suggested action, select the first visible action's source note.
- Accepting or dismissing an action removes it from the Actions list after refresh and keeps the source note detail up to date.
- If there are no suggested actions, the list column should show a compact empty state.

## UI Shape

The Actions view reuses the current three-column app composition:

- Sidebar: Actions is active.
- Left column: action rows grouped by due bucket, with a search row at the top.
- Right column: the existing note detail panel.

Each action row should contain:

- A small completion-style action button.
- Action text.
- Metadata line: owner, due label, and `from "note title"`.
- A dismiss affordance with an accessible label.

The design should stay compact and operational. Use existing semantic CSS variables such as `--color-surface-1`, `--color-surface-2`, `--color-border-default`, `--color-text-primary`, `--color-text-muted`, `--color-accent-primary`, `--color-status-warning`, and `--color-status-error`.

## Data Flow

The implementation should use existing backend behavior:

- `list_suggested_actions` returns `ActionReviewItem[]`.
- `accept_action_item` accepts a suggested action.
- `dismiss_action_item` dismisses a suggested action.
- `get_note` loads the source note.

Frontend flow:

1. `AppShell` emits `navigate: "actions"` from the Actions sidebar button.
2. `src/routes/+page.svelte` calls `workNotes.showActions()`.
3. `src/lib/stores/inbox.ts` sets `viewMode` to `actions`, refreshes `suggestedActions`, and selects a source note when needed.
4. `ActionsList.svelte` groups and filters `suggestedActions`.
5. Row click dispatches `select` with `noteId`.
6. Accept and dismiss buttons dispatch action lifecycle events to the existing store methods.

## Error Handling

- Action list load failures should use the existing store error surface.
- Accept and dismiss failures should preserve the current row state and show the existing store error.
- If a source note lookup returns not found, reuse the stale-selection handling already added for Archive.

## Testing

Frontend tests should cover:

- AppShell marks Actions active and emits Actions navigation.
- The store enters Actions view, refreshes suggested actions, and selects the first source note when appropriate.
- ActionsList groups actions by due bucket.
- ActionsList filters by text, owner, due date, and source note title.
- ActionsList dispatches select, accept, and dismiss events without row-click leakage from the buttons.
- The route hides ReviewQueue while Actions view is active, if route-level testing exists.

Full verification:

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```

