# People View Design

## Source Design

This design is derived from `work-notes-handoff.zip`, specifically:

- `work-notes/project/Work Notes Redesign v3.html`, artboard `09 · People — Maria · Memphis`.
- `work-notes/project/today-people-v2.jsx`, `PeopleView`.
- `work-notes/project/app-v2-extra.css`, `.people-*` and `.person-*` styles.
- `work-notes/project/data-v2.jsx`, sample note tags and suggested action owners.

The production implementation should follow the current Svelte/Tauri architecture and semantic theme variables rather than copying React prototype internals.

## Goal

Make the sidebar `People` item real. Selecting it should show a person list and selected-person detail surface built from existing note person tags and suggested action owners.

## Scope

In scope:

- Add `people` as a primary view in the existing app shell.
- Replace the inert `People` sidebar link with an active navigation button.
- Load all non-archived inbox notes and suggested actions when entering People.
- Build a person list from note tags with `kind: "person"` and suggested action owners other than `me`.
- Show each person with note count, last interaction, and suggested-action count.
- Provide person search in the list column.
- Show selected person detail with `You owe`, `<person> owes you`, and `Recent notes` sections.
- Clicking an action or note opens its source note through the normal Inbox detail flow.
- Keep all persistence and Tauri calls in the existing store/API path.

Out of scope:

- New backend commands or database schema.
- Editing people, aliases, owners, action text, or due dates.
- Creating standalone notes from People.
- Accepted follow-up board behavior.
- Tags view implementation.
- Keyboard shortcut overlay behavior.

## Product Behavior

- `Inbox` remains the default view.
- `People` is a full content surface with its own list/detail layout.
- Person identity is case-insensitive for grouping but should display a stable human label, preferring the first tag/owner casing seen in data.
- Suggested action owner `me` is not a person row.
- `You owe <person>` includes suggested actions assigned to `me` whose source note has a matching person tag.
- `<person> owes you` includes suggested actions whose owner matches the selected person.
- `Recent notes` includes non-archived notes tagged with the selected person, newest first.
- If there are no people, show a compact empty state.
- If the selected person disappears after refresh, select the first visible person; if no people remain, clear the selection.

## UI Shape

The People view uses the existing shell sidebar and replaces the main workspace with two columns:

- Left column: `People` header, search field, person rows with avatar initial, note count, last interaction, and pending action badge.
- Right column: selected person header with large avatar, name, note count, and last interaction.
- Detail body: two compact action columns, then recent notes.

Use semantic CSS variables such as `--color-surface-1`, `--color-surface-2`, `--color-surface-input`, `--color-border-default`, `--color-text-primary`, `--color-text-muted`, `--color-accent-primary`, `--color-status-warning`, and `--color-status-error`.

## Data Flow

Frontend-only flow:

1. `AppShell` emits `navigate: "people"` from the People sidebar button.
2. `src/routes/+page.svelte` calls `workNotes.showPeople()`.
3. `src/lib/stores/inbox.ts` sets `viewMode` to `people`, clears archived filtering, refreshes non-archived inbox notes, and refreshes suggested actions.
4. `people.ts` derives people, selected person detail, and person/action groupings from `NoteListItem[]` and `ActionReviewItem[]`.
5. `PeopleView.svelte` renders the derived list/detail surface and emits `openNote`.
6. Opening a source note calls `workNotes.showInbox()` and then `workNotes.selectNote(noteId)`.

## Error Handling

- Inbox and suggested-action load failures use the existing store error banner.
- Unparseable dates show their raw value instead of throwing.
- Source note lookup failures reuse the existing stale-selection handling from `selectNote`.
- Empty data states should not throw when no selected person exists.

## Testing

Frontend tests should cover:

- People helper functions derive person rows from person tags and suggested action owners.
- People helper functions group `You owe`, `<person> owes you`, and recent notes correctly.
- AppShell marks People active and emits People navigation.
- The store enters People view and refreshes inbox plus suggested actions without stale filters.
- PeopleView filters people by search text.
- PeopleView dispatches open-source-note events from action and note rows.

Full verification:

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```
