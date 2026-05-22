# Archive Data Management Design

## Purpose

Work Notes already soft-archives notes through `delete_note`, but the app has no real Archive surface, no restore path, and no permanent delete path. This slice makes the Archive sidebar item real and gives users a controlled way to restore or permanently remove archived notes.

The feature keeps capture and parser invariants intact:

- Raw note text remains the source of truth.
- Soft archive is reversible.
- Permanent delete is explicit, confirmed, and only available from archived notes.
- Parser jobs and derived data are removed only when the user confirms permanent deletion.

## Scope

In scope:

- Make the sidebar `Archive` entry switch the main view into archive mode.
- Show archived notes in the existing inbox/list/detail layout.
- Add `restore_note`.
- Add `permanently_delete_note`.
- Add frontend API/store methods for restore and permanent delete.
- Add archived-note detail actions: `Restore` and `Delete permanently`.
- Confirm permanent delete with the note title before executing.
- Refresh list and selection after restore or permanent delete.

Out of scope:

- Export and backup.
- Bulk archive cleanup.
- Undo after permanent delete.
- Permanent delete from normal inbox mode.
- Raw note editing.
- Parser prompt or schema changes.

## Product Behavior

The sidebar has two relevant modes:

- `Inbox`: default mode, shows non-archived notes.
- `Archive`: shows only archived notes.

Archiving from the normal inbox keeps the current behavior: `delete_note` marks the note archived and removes it from the default inbox.

Archive mode shows archived notes in the same dense list/detail pattern as the inbox. Search, tag filters, parse status filters, and review status filters should still work against archived notes. Empty archive state should say that archived notes will appear there after notes are deleted from the inbox.

When an archived note is selected, note detail shows:

- `Restore`: clears `is_archived`, refreshes the archive list, and moves the note back to the inbox.
- `Delete permanently`: opens a confirmation flow that names the note. Confirming removes the note and dependent rows.

After restore, the app should leave archive mode or clear the archived selection so the restored note does not look like it still belongs in Archive. The preferred behavior is to switch to Inbox and select the restored note.

After permanent delete, the app stays in Archive mode, clears the selected note, and selects the next archived note if one exists.

## Backend Architecture

### Repository

Extend `NoteRepository` with:

```text
restore(note_id)
permanently_delete(note_id)
```

`restore` sets `is_archived = 0` and updates `updated_at`.

`permanently_delete` runs in one SQLite transaction and deletes dependent rows before deleting the note:

```text
note_tags
action_items
parse_jobs
parse_runs
notes_fts
notes
```

The method should return `not_found` when the note does not exist. Permanent delete should not depend on parser state.

### Commands

Keep existing command behavior:

```text
delete_note
```

`delete_note` remains soft archive for compatibility with the current UI and user expectations.

Add commands:

```text
restore_note
permanently_delete_note
```

Both commands accept a `noteId` string, parse it to `NoteId`, call repository methods, and return `CommandError` consistently with existing command error handling.

### Data Safety

Permanent delete is intentionally not named `delete_note`; the naming difference keeps soft and hard deletion separate in frontend code and command logs.

No backend command should permanently delete a non-archived note in this slice. If `permanently_delete_note` is called for a non-archived note, return `invalid_input`. This forces the user workflow to pass through Archive before hard deletion.

## Frontend Architecture

### Navigation State

Add a main view mode in `src/routes/+page.svelte` or `src/lib/stores/inbox.ts`:

```text
inbox | archive
```

`AppShell.svelte` should stop rendering Archive as a dead link. It should emit a navigation event or accept active view props so the route can switch modes without a full page reload.

The first implementation should keep a single route and not introduce SvelteKit pages. This matches the current Tauri SPA structure.

### Store

Workflow sequencing stays in `src/lib/stores/inbox.ts`.

Add store methods:

```text
loadArchive()
showArchive()
restoreSelectedNote()
permanentlyDeleteSelectedNote()
```

Implementation options:

- Use `listInbox({ includeArchived: true })`, then filter `isArchived` client-side for archive mode.
- Keep default inbox loading with `includeArchived: false`.

Client-side archive filtering is acceptable for this slice because the existing backend filter model only has `includeArchived`; a dedicated `archivedOnly` backend filter can be added later if archive counts grow.

After restore:

1. Call `restore_note`.
2. Switch filters/view back to Inbox.
3. Reload inbox.
4. Select the restored note.

After permanent delete:

1. Call `permanently_delete_note`.
2. Clear selected note.
3. Reload archive.
4. Select the next archived note if available.

### API

Add wrappers in `src/lib/api.ts`:

```text
restoreNote(noteId)
permanentlyDeleteNote(noteId)
```

Update browser fallback data:

- `restore_note` clears `isArchived`.
- `permanently_delete_note` removes the note from the fallback array only if it is archived.
- Non-archived permanent delete in fallback should throw an error to match backend behavior.

### Components

`AppShell.svelte`:

- Accept `activeView`.
- Emit a `navigate` event for `inbox` and `archive`.
- Mark the active nav item visually.
- Keep Today, Tags, and People visually inactive until they get real modes later.

`InboxList.svelte`:

- Accept a label or mode prop so empty states can say `No archived notes` in archive mode.
- Continue to handle search and tag filters.

`NoteDetail.svelte`:

- For non-archived notes, keep current archive button behavior.
- For archived notes, show `Restore` and `Delete permanently`.
- Permanent delete event should be separate from soft archive event.

`src/routes/+page.svelte`:

- Own the permanent delete confirmation with `window.confirm` for the first slice.
- Confirmation text should include the note title.

## Error Handling

Restore errors should keep the user in Archive mode and show the existing app error banner.

Permanent delete errors should keep the archived note selected and show the existing app error banner.

Confirmation cancellation should do nothing and must not clear selection.

If a note disappears because it was permanently deleted from another app instance, reload archive and clear stale selection when `get_note` returns not found.

## Testing

Rust tests:

- `NoteRepository::restore` clears `is_archived`.
- `NoteRepository::permanently_delete` removes the note and dependent rows.
- Permanent delete of a non-archived note returns invalid input through the service or command layer.
- Command DTO/command tests cover `restore_note` and `permanently_delete_note` error mapping.

Frontend tests:

- Store can switch between inbox and archive mode.
- Store loads only archived notes in archive mode.
- Restore switches back to inbox and selects the restored note.
- Permanent delete clears selection and refreshes archive.
- `AppShell.svelte` emits navigation events and marks Archive active.
- `NoteDetail.svelte` emits restore and permanent delete events only for archived notes.

Manual checks:

- Archive a note from Inbox.
- Open Archive and see the archived note.
- Restore the note and confirm it returns to Inbox.
- Archive it again.
- Cancel permanent delete and confirm the note remains.
- Confirm permanent delete and verify the note disappears from Archive.
- Verify raw note text and parser data for other notes are unaffected.

## Implementation Order

1. Add backend restore and permanent-delete repository tests.
2. Implement repository methods and command handlers.
3. Add frontend API wrappers and fallback behavior.
4. Add store view-mode and archive workflows.
5. Wire AppShell navigation.
6. Update InboxList and NoteDetail archived states.
7. Add route-level confirmation.
8. Run frontend, Rust, and browser smoke verification.
