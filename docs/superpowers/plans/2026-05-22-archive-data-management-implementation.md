# Archive Data Management Implementation Plan

> **For the implementer:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to execute this plan step by step.

**Goal:** Make the Archive sidebar item functional by adding an archive view, restore workflow, and permanent-delete workflow while preserving the invariant that raw notes are the source of truth and parser output is derived data.

**Approved design:** `docs/superpowers/specs/2026-05-22-archive-data-management-design.md`

**Architecture boundaries:**
- Frontend workflow sequencing belongs in `src/lib/stores/inbox.ts`.
- Tauri command calls belong in `src/lib/api.ts`.
- Tauri command DTOs and command validation belong in `src-tauri/src/commands.rs`.
- Workflow behavior belongs in `src-tauri/src/services/`.
- SQLite reads and writes belong in `src-tauri/src/repositories/`.
- Svelte components should stay thin and emit UI events.

**Out of scope for this slice:**
- Export, backup, and import.
- Bulk archive cleanup.
- Undo after permanent delete.
- Permanent delete from the normal inbox.
- Parser prompt, schema, retry, or Codex CLI changes.
- Raw note editing.

## Current State

- `delete_note` soft-archives a note through `NoteRepository::archive`.
- The `Archive` item in `AppShell.svelte` is visual navigation only.
- `list_inbox` can include archived notes when `includeArchived` is true.
- The frontend store has only inbox mode and clears selection after soft archive.
- No restore or permanent-delete Tauri command exists.

## Desired Behavior

- Inbox mode lists non-archived notes.
- Archive mode lists archived notes only and supports the existing search, tag, parse status, and review status filters.
- Soft delete keeps using `delete_note` and sets `is_archived = 1`.
- Restore is available only for archived notes, clears `is_archived`, switches to Inbox mode, and selects the restored note.
- Permanent delete is available only for archived notes, requires a browser confirmation that names the note title, deletes dependent SQLite rows in one transaction, stays in Archive mode, and selects the next archived note when one exists.
- Permanent delete of a non-archived note returns an invalid-input error.

## Task 1: Add Backend Restore And Permanent Delete Repository Support

**Files:**
- `src-tauri/src/repositories/notes.rs`

**Tests first:**
Add a `#[cfg(test)] mod tests` block in `src-tauri/src/repositories/notes.rs`. Use this helper at the top of the module:

```rust
fn setup_notes() -> (Database, NoteRepository) {
    let db = Database::in_memory().unwrap();
    let notes = NoteRepository::new(db.clone());
    (db, notes)
}
```

Then add these repository tests.

```rust
#[test]
fn restore_archived_note_returns_it_to_default_inbox() {
    let (db, notes) = setup_notes();
    let _keep_db_alive = db;
    let note = notes.create_raw_note("restore me").expect("create note");

    notes.archive(note.id).expect("archive note");
    notes.restore(note.id).expect("restore note");

    let restored = notes.get(note.id).expect("get note").expect("note exists");
    assert!(!restored.is_archived);

    let inbox = notes
        .list_inbox(&InboxFilters::default())
        .expect("list default inbox");
    assert!(inbox.iter().any(|item| item.id == note.id));
}

#[test]
fn permanently_delete_archived_note_removes_dependents() {
    let (db, notes) = setup_notes();
    let tags = TagRepository::new(db.clone());
    let actions = ActionItemRepository::new(db.clone());
    let parse_jobs = ParseJobRepository::new(db.clone());
    let note = notes.create_raw_note("delete me").expect("create note");

    let tag = tags.upsert("cleanup", TagKind::Topic).expect("create tag");
    actions
        .create_suggested(note.id, "Follow up", None, None, Some(0.8))
        .expect("add action");
    tags.apply_to_note(note.id, tag.id, "parser", Some(0.8))
        .expect("tag note");

    parse_jobs
        .record_run(note.id, "test", "test-v1", "raw response", "{}", None)
        .expect("record parse run");

    notes.archive(note.id).expect("archive note");
    notes.permanently_delete(note.id).expect("delete note");

    assert!(notes.get(note.id).expect("get deleted note").is_none());

    let connection = db.connection().expect("connect db");
    for table in ["note_tags", "action_items", "parse_jobs", "parse_runs", "notes_fts"] {
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE note_id = ?1"),
                params![note.id.to_string()],
                |row| row.get(0),
            )
            .expect("count dependent rows");
        assert_eq!(count, 0, "{table} rows should be removed");
    }
}
```

Import `Database`, `NoteRepository`, `ActionItemRepository`, `ParseJobRepository`, `TagRepository`, `InboxFilters`, `TagKind`, and `rusqlite::params` in the test module if they are not already in scope.

**Implementation:**
Add two methods on `NoteRepository`.

```rust
pub fn restore(&self, id: NoteId) -> RepositoryResult<()> {
    let id_text = id.to_string();
    let now = now_db_string();
    let connection = self.db.connection()?;
    let changed = connection.execute(
        "UPDATE notes SET is_archived = 0, updated_at = ?2 WHERE id = ?1",
        params![id_text, now],
    )?;

    if changed == 0 {
        return Err(RepositoryError::NotFound {
            entity: "note",
            id: id.to_string(),
        });
    }

    Ok(())
}

pub fn permanently_delete(&self, id: NoteId) -> RepositoryResult<()> {
    let id_text = id.to_string();
    let mut connection = self.db.connection()?;
    let transaction = connection.transaction()?;

    transaction.execute("DELETE FROM note_tags WHERE note_id = ?1", params![&id_text])?;
    transaction.execute("DELETE FROM action_items WHERE note_id = ?1", params![&id_text])?;
    transaction.execute("DELETE FROM parse_jobs WHERE note_id = ?1", params![&id_text])?;
    transaction.execute("DELETE FROM parse_runs WHERE note_id = ?1", params![&id_text])?;
    transaction.execute("DELETE FROM notes_fts WHERE note_id = ?1", params![&id_text])?;
    let changed = transaction.execute("DELETE FROM notes WHERE id = ?1", params![&id_text])?;

    if changed == 0 {
        return Err(RepositoryError::NotFound {
            entity: "note",
            id: id.to_string(),
        });
    }

    transaction.commit()?;
    Ok(())
}
```

**Verification:**

```powershell
scripts\cargo-test.cmd repositories::notes
```

## Task 2: Add Archive Workflow Service

**Files:**
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/services/archive.rs`

**Tests first:**
Create service tests in `src-tauri/src/services/archive.rs`.

```rust
#[test]
fn permanently_delete_rejects_non_archived_notes() {
    let repositories = repositories();
    let service = ArchiveService::new(repositories.clone());
    let note = repositories
        .notes
        .create_raw_note("active note")
        .expect("create note");

    let error = service
        .permanently_delete(note.id)
        .expect_err("active notes cannot be hard deleted");

    assert!(matches!(error, ServiceError::InvalidInput(_)));
    assert!(repositories.notes.get(note.id).expect("get note").is_some());
}

#[test]
fn restore_archived_note_clears_archive_flag() {
    let repositories = repositories();
    let service = ArchiveService::new(repositories.clone());
    let note = repositories
        .notes
        .create_raw_note("archived note")
        .expect("create note");
    repositories.notes.archive(note.id).expect("archive note");

    service.restore(note.id).expect("restore note");

    let restored = repositories
        .notes
        .get(note.id)
        .expect("get note")
        .expect("note exists");
    assert!(!restored.is_archived);
}
```

Use this service test helper unless an equivalent helper already exists in the file:

```rust
fn repositories() -> AppRepositories {
    AppRepositories::new(Database::in_memory().unwrap())
}
```

**Implementation:**
Add an archive service that centralizes lifecycle rules.

```rust
use crate::app_state::AppRepositories;
use crate::domain::NoteId;

use super::{ServiceError, ServiceResult};

#[derive(Clone)]
pub struct ArchiveService {
    repositories: AppRepositories,
}

impl ArchiveService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn restore(&self, note_id: NoteId) -> ServiceResult<()> {
        self.repositories.notes.restore(note_id)?;
        Ok(())
    }

    pub fn permanently_delete(&self, note_id: NoteId) -> ServiceResult<()> {
        let note = self
            .repositories
            .notes
            .get(note_id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "note",
                id: note_id.to_string(),
            })?;

        if !note.is_archived {
            return Err(ServiceError::InvalidInput(
                "note must be archived before permanent delete",
            ));
        }

        self.repositories.notes.permanently_delete(note_id)?;
        Ok(())
    }
}
```

Expose the module in `src-tauri/src/services/mod.rs`:

```rust
pub mod archive;
```

`ServiceError` already has `NotFound` and `InvalidInput`. Keep existing parser/action-item service behavior unchanged.

**Verification:**

```powershell
scripts\cargo-test.cmd services::archive
```

## Task 3: Expose Restore And Permanent Delete Tauri Commands

**Files:**
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

**Tests first:**
Add command error mapping tests in `src-tauri/src/commands.rs`.

```rust
#[test]
fn service_invalid_input_maps_to_command_invalid_input() {
    let error = CommandError::from(ServiceError::InvalidInput(
        "note must be archived before permanent delete",
    ));

    assert_eq!(error.code, "invalid_input");
}

#[test]
fn service_not_found_maps_to_command_not_found() {
    let error = CommandError::from(ServiceError::NotFound {
        entity: "note",
        id: "note-missing".to_string(),
    });

    assert_eq!(error.code, "not_found");
}
```

**Implementation:**
Add command functions that parse the note id through the existing validation helper and call the service.

```rust
#[tauri::command]
pub async fn restore_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    let service = ArchiveService::new(state.repositories.clone());
    service.restore(note_id)?;
    Ok(())
}

#[tauri::command]
pub async fn permanently_delete_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    let service = ArchiveService::new(state.repositories.clone());
    service.permanently_delete(note_id)?;
    Ok(())
}
```

Add the required imports:

```rust
use crate::services::archive::ArchiveService;
```

Update the invoke handler in `src-tauri/src/lib.rs`:

```rust
tauri::generate_handler![
    commands::save_capture_note,
    commands::list_inbox,
    commands::get_note,
    commands::retry_parse,
    commands::retry_parse_with_feedback,
    commands::delete_note,
    commands::restore_note,
    commands::permanently_delete_note,
    commands::accept_action_item,
    commands::dismiss_action_item,
    commands::complete_action_item,
    commands::reopen_action_item,
    commands::list_suggested_actions,
    commands::get_settings,
    commands::save_settings,
    commands::hide_quick_capture,
]
```

Update `impl From<ServiceError> for CommandError` to preserve invalid-input and not-found codes:

```rust
impl From<ServiceError> for CommandError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::InvalidInput(message) => CommandError::invalid_input(message),
            ServiceError::NotFound { entity, id } => CommandError::not_found(entity, &id),
            ServiceError::Repository(error) => CommandError::from(error),
            ServiceError::Database(_) | ServiceError::Sqlite(_) => CommandError::storage_error(),
            ServiceError::Json(_) => CommandError::new("settings_error", "settings data is invalid"),
            ServiceError::Parser(_) => CommandError::new("parser_error", "parser failed"),
            ServiceError::StatePoisoned { name } => {
                CommandError::new("state_error", format!("{name} state unavailable"))
            }
        }
    }
}
```

Match the actual `ServiceError` variants in the repo. Do not widen command errors to plain storage errors for lifecycle validation failures.

**Verification:**

```powershell
scripts\cargo-test.cmd commands
```

## Task 4: Add Frontend API Methods And Fallback Behavior

**Files:**
- `src/lib/api.ts`
- `src/lib/types.ts` if an app-view type is added there

**Tests first:**
The store tests in Task 5 will cover the API surface through the existing invoke mock. If `src/lib/api.test.ts` exists, add direct tests for the fallback behavior there.

**Implementation:**
Add command wrappers near `deleteNote`.

```ts
export async function restoreNote(noteId: string): Promise<void> {
  await invokeCommand<void>("restore_note", { noteId });
}

export async function permanentlyDeleteNote(noteId: string): Promise<void> {
  await invokeCommand<void>("permanently_delete_note", { noteId });
}
```

Extend the fallback invoke switch.

```ts
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
```

Seed one archived fallback note so the archive view can be exercised when Tauri invoke is not available:

```ts
{
  id: "fallback-archived-1",
  title: "Archived workspace cleanup note",
  rawText: "Archived workspace cleanup note",
  cleanedText: "Archived workspace cleanup note",
  summary: "Archived workspace cleanup note",
  captureSource: "quick_capture",
  tags: [{ id: "fallback-tag-archive", name: "archive", kind: "topic", source: "user" }],
  parseStatus: "parsed",
  reviewStatus: "reviewed",
  isArchived: true,
  createdAt: new Date(Date.now() - 180000).toISOString(),
  updatedAt: new Date(Date.now() - 180000).toISOString(),
  actionItems: [],
  actionItemCount: 0,
  suggestedActionItemCount: 0,
}
```

**Verification:**

```powershell
npm test -- api
```

If there is no API-specific test file, rely on the Task 5 store tests and run the full Vitest suite after Task 6.

## Task 5: Add Inbox Store View Modes And Archive Workflows

**Files:**
- `src/lib/stores/inbox.ts`
- Existing store test file under `src/lib/stores/`
- `src/lib/types.ts` if shared type placement is cleaner

**Tests first:**
Add tests for mode switching, archive filtering, restore behavior, and permanent delete behavior. Use the existing invoke mock pattern from the store tests.

```ts
it("loads archived notes in archive mode only", async () => {
  const api = testApi({
    listInbox: vi.fn().mockResolvedValue([
      note({ id: "active", isArchived: false }),
      note({ id: "archived", isArchived: true }),
    ]),
  });

  const store = createWorkNotesStore(api);
  await store.showArchive();

  expect(get(store.viewMode)).toBe("archive");
  expect(get(store.inbox).map((item) => item.id)).toEqual(["archived"]);
  expect(api.listInbox).toHaveBeenCalledWith(
    expect.objectContaining({ includeArchived: true }),
  );
});

it("restores the selected archived note and returns to inbox mode", async () => {
  const archived = note({ id: "archived", isArchived: true });
  const restored = note({ id: "archived", isArchived: false });
  const api = testApi({
    listInbox: vi
      .fn()
      .mockResolvedValueOnce([archived])
      .mockResolvedValueOnce([restored]),
    getNote: vi
      .fn()
      .mockResolvedValueOnce({ ...archived, actionItems: [] })
      .mockResolvedValueOnce({ ...restored, actionItems: [] }),
  });

  const store = createWorkNotesStore(api);
  await store.showArchive();
  await store.selectNote("archived");
  await store.restoreSelectedNote();

  expect(get(store.viewMode)).toBe("inbox");
  expect(get(store.selectedNote)?.id).toBe("archived");
  expect(api.restoreNote).toHaveBeenCalledWith("archived");
});

it("permanently deletes selected archived note and stays in archive mode", async () => {
  const first = note({ id: "archived-1", isArchived: true });
  const second = note({ id: "archived-2", isArchived: true });
  const api = testApi({
    listInbox: vi.fn().mockResolvedValueOnce([first, second]).mockResolvedValueOnce([second]),
    getNote: vi
      .fn()
      .mockResolvedValueOnce({ ...first, actionItems: [] })
      .mockResolvedValueOnce({ ...second, actionItems: [] }),
  });

  const store = createWorkNotesStore(api);
  await store.showArchive();
  await store.selectNote("archived-1");
  await store.permanentlyDeleteSelectedNote();

  expect(get(store.viewMode)).toBe("archive");
  expect(get(store.selectedNote)?.id).toBe("archived-2");
  expect(api.permanentlyDeleteNote).toHaveBeenCalledWith("archived-1");
});
```

Adjust helper names to the existing test file. Keep assertions focused on command names, mode, filtered list, and selected note.

**Implementation:**
Extend the store API type and defaults with the new API methods:

```ts
export type InboxViewMode = "inbox" | "archive";

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
  listSuggestedActions: typeof listSuggestedActions;
  getSettings: typeof getSettings;
  saveSettings: typeof saveSettings;
};
```

Import `restoreNote` and `permanentlyDeleteNote` from `$lib/api`, add them to `defaultApi`, and add matching mocks to `testApi()` in `src/lib/stores/inbox.test.ts`.

Add the mode store near `filters`:

```ts
const viewMode = writable<InboxViewMode>("inbox");
```

Update the filtered derived store so archive mode allows archived rows through the existing filter helper.

```ts
const filteredInbox = derived([inbox, filters, viewMode], ([$inbox, $filters, $viewMode]) => {
  const modeFilters = createInboxFilters({
    ...$filters,
    includeArchived: $viewMode === "archive",
  });

  return $inbox.filter((note) => matchesNoteFilters(note, modeFilters));
});
```

Update `loadInbox` to load the right backend shape for the active mode.

```ts
async function loadInbox(): Promise<void> {
  loadingInbox.set(true);
  error.set(null);

  try {
    const mode = get(viewMode);
    const currentFilters = get(filters);
    const backendFilters = createInboxFilters({
      ...currentFilters,
      includeArchived: mode === "archive",
    });
    const items = await api.listInbox(backendFilters);
    const visibleItems =
      mode === "archive"
        ? items.filter((note) => note.isArchived)
        : items.filter((note) => !note.isArchived);

    inbox.set(visibleItems);
    await ensureSelectionAfterLoad(visibleItems);
  } catch (cause) {
    error.set(errorMessage(cause, "Could not load inbox."));
  } finally {
    loadingInbox.set(false);
  }
}
```

Extract the existing selection logic into `ensureSelectionAfterLoad(items: NoteListItem[])` so delete, restore, and hard-delete workflows share one path.

Add public methods:

```ts
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

async function restoreSelectedNote(): Promise<void> {
  const note = get(selectedNote);

  if (!note) {
    return;
  }

  await api.restoreNote(note.id);
  viewMode.set("inbox");
  filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
  await loadInbox();
  await selectNote(note.id);
}

async function permanentlyDeleteSelectedNote(): Promise<void> {
  const note = get(selectedNote);

  if (!note) {
    return;
  }

  await api.permanentlyDeleteNote(note.id);
  selectedNote.set(null);
  await loadInbox();
}
```

Make `permanentlyDeleteSelectedNote` rely on `loadInbox` selection handling for selecting the next archived note. If current selection handling only selects the first note, preserve that behavior; the approved design requires "next if any", and first remaining archived note is acceptable for this slice when the deleted note is gone.

Return the new methods and `viewMode` from the store object so routes can subscribe to them.

Update `showCapturedNote` so externally captured notes always return the UI to Inbox mode before selecting the note:

```ts
async function showCapturedNote(noteId: string): Promise<void> {
  viewMode.set("inbox");
  filters.set(createInboxFilters({ includeArchived: false }));
  await loadInbox();
  await selectNote(noteId);
}
```

**Verification:**

```powershell
npm test -- inbox
```

## Task 6: Wire Archive Navigation And Note Detail Actions

**Files:**
- `src/lib/components/AppShell.svelte`
- `src/lib/components/InboxList.svelte`
- `src/lib/components/NoteDetail.svelte`
- Existing component test files under `src/lib/components/`

**Tests first:**
Add or update component tests to cover UI events and archive-specific rendering.

`AppShell.svelte`:

```ts
it("marks archive active and emits archive navigation", async () => {
  const navigate = vi.fn();
  const { getByRole } = render(AppShell, {
    props: {
      title: "Work Notes",
      subtitle: "Fast capture",
      workspace: "Local workspace",
      metrics: [{ label: "Inbox", value: "1" }],
      activeView: "archive",
    },
    events: { navigate },
  });

  await fireEvent.click(getByRole("button", { name: "Archive" }));

  expect(getByRole("button", { name: "Archive" })).toHaveAttribute("aria-current", "page");
  expect(navigate).toHaveBeenCalledWith(expect.objectContaining({ detail: "archive" }));
});
```

`NoteDetail.svelte`:

```ts
it("shows restore and permanent delete actions for archived notes", async () => {
  const restoreNote = vi.fn();
  const permanentlyDeleteNote = vi.fn();
  const { getByRole, queryByRole } = render(NoteDetail, {
    props: { note: { ...noteDetail(), isArchived: true } },
    events: { restoreNote, permanentlyDeleteNote },
  });

  expect(queryByRole("button", { name: "Archive" })).not.toBeInTheDocument();

  await fireEvent.click(getByRole("button", { name: "Restore" }));
  await fireEvent.click(getByRole("button", { name: "Delete permanently" }));

  expect(restoreNote).toHaveBeenCalledTimes(1);
  expect(permanentlyDeleteNote).toHaveBeenCalledTimes(1);
});
```

`InboxList.svelte`:

```ts
it("uses archive empty copy in archive mode", () => {
  const { getByText } = render(InboxList, {
    props: {
      items: [],
      filters: createInboxFilters({ includeArchived: true }),
      selectedId: undefined,
      viewMode: "archive",
    },
  });

  expect(getByText("No archived notes")).toBeInTheDocument();
});
```

**Implementation:**
Update `AppShell.svelte` to accept active view and emit navigation.

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { InboxViewMode } from "$lib/stores/inbox";

  type Props = {
    title: string;
    subtitle: string;
    workspace: string;
    metrics: ShellMetric[];
    activeView?: InboxViewMode;
    tags?: string[];
    parserCommand?: string;
    parserQueueCount?: number;
    themeId?: string;
    themeStyle?: string;
    children?: Snippet;
    quickCapture?: Snippet;
  };

  let {
    title,
    subtitle,
    workspace,
    metrics,
    activeView = "inbox",
    tags = [],
    parserCommand = "codex.cmd",
    parserQueueCount = 0,
    themeId = "dark-compact",
    themeStyle = "",
    children,
    quickCapture,
  }: Props = $props();

  const dispatch = createEventDispatcher<{
    newNote: void;
    settings: void;
    navigate: InboxViewMode;
  }>();

  function navigate(view: InboxViewMode) {
    dispatch("navigate", view);
  }
</script>
```

Use buttons for Inbox and Archive navigation:

```svelte
<button
  type="button"
  class:active={activeView === "inbox"}
  aria-current={activeView === "inbox" ? "page" : undefined}
  onclick={() => navigate("inbox")}
>
  Inbox
</button>

<button
  type="button"
  class:active={activeView === "archive"}
  aria-current={activeView === "archive" ? "page" : undefined}
  onclick={() => navigate("archive")}
>
  Archive
</button>
```

Keep existing styles and adjust selectors from anchors to buttons. Use semantic theme variables already present in the component.

Update `InboxList.svelte`:

```svelte
<script lang="ts">
  import type { InboxViewMode } from "$lib/stores/inbox";

  type Props = {
    items: NoteListItem[];
    filters: InboxFilters;
    selectedId?: string;
    loading?: boolean;
    viewMode?: InboxViewMode;
  };

  let { items, filters, selectedId, loading = false, viewMode = "inbox" }: Props = $props();
</script>

{#if visibleItems.length === 0}
  <div class="empty-state">
    <div class="empty-mark">WN</div>
    <h2>{viewMode === "archive" && mode === "notes" ? "No archived notes" : mode === "actions" ? "No suggested actions" : "No notes"}</h2>
    <p>{viewMode === "archive" && mode === "notes" ? "Restored notes return to Inbox." : mode === "actions" ? "Parser suggestions will appear here after capture." : "Press the hotkey to capture your first one."}</p>
  </div>
{/if}
```

Update `NoteDetail.svelte` to emit archived-note actions.

```svelte
<script lang="ts">
  const dispatch = createEventDispatcher<{
    deleteNote: void;
    restoreNote: void;
    permanentlyDeleteNote: void;
    retryParse: void;
    reparseWithFeedback: string;
    acceptAction: string;
    dismissAction: string;
    completeAction: string;
    reopenAction: string;
  }>();
</script>
```

Render lifecycle controls based on `note.isArchived`.

```svelte
{#if note.isArchived}
  <button type="button" class="ghost-button" onclick={() => dispatch("restoreNote")} disabled={loading}>
    Restore
  </button>
  <button type="button" class="delete-button" onclick={() => dispatch("permanentlyDeleteNote")} disabled={loading}>
    Delete permanently
  </button>
{:else}
  <button type="button" class="delete-button" onclick={() => dispatch("deleteNote")} disabled={loading}>
    Archive
  </button>
{/if}
```

Do not expose permanent delete for non-archived notes.

**Verification:**

```powershell
npm test -- AppShell NoteDetail InboxList
```

## Task 7: Wire Route-Level Confirmation And Event Handling

**Files:**
- `src/routes/+page.svelte`

**Tests first:**
If route-level tests exist, add coverage for:
- `navigate` to archive calls `showArchive`.
- Restore event calls `restoreSelectedNote`.
- Permanent delete event calls `window.confirm` with the selected note title and calls `permanentlyDeleteSelectedNote` only on confirmation.

If route-level tests do not exist, keep this covered by component/store tests and manual browser verification in Task 9.

**Implementation:**
Expose `viewMode` from the page's existing `workNotes` destructuring.

```ts
import { createWorkNotesStore, type InboxViewMode } from "$lib/stores/inbox";
```

```svelte
const {
  inbox,
  filteredInbox,
  filters,
  viewMode,
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
} = workNotes;
```

Add route handlers:

```ts
async function navigatePrimary(event: CustomEvent<InboxViewMode>) {
  if (event.detail === "archive") {
    await workNotes.showArchive();
    return;
  }

  await workNotes.showInbox();
}

async function restoreSelectedNote() {
  await workNotes.restoreSelectedNote();
}

async function permanentlyDeleteSelectedNote() {
  const note = $selectedNote;

  if (!note) {
    return;
  }

  const confirmed = window.confirm(
    `Permanently delete "${note.title || note.summary || note.rawText.slice(0, 40)}"? This cannot be undone.`,
  );

  if (!confirmed) {
    return;
  }

  await workNotes.permanentlyDeleteSelectedNote();
}
```

Pass props and events:

```svelte
<AppShell
  title="Work Notes"
  subtitle="Fast capture for coworker drive-bys"
  workspace="Local workspace"
  {metrics}
  tags={topTags}
  {parserCommand}
  {parserQueueCount}
  {themeId}
  {themeStyle}
  activeView={$viewMode}
  on:newNote={() => void openQuickCapture()}
  on:settings={() => (settingsOpen = true)}
  on:navigate={(event) => void navigatePrimary(event)}
>
  <InboxList
    items={$filteredInbox}
    filters={$filters}
    {selectedId}
    loading={$loadingInbox}
    viewMode={$viewMode}
    on:select={(event) => void workNotes.selectNote(event.detail)}
    on:filter={(event) => void workNotes.updateFilters(event.detail)}
  />

  <NoteDetail
    note={$selectedNote}
    loading={$loadingNote}
    busyActionId={$busyActionId}
    on:retryParse={() => void workNotes.retrySelectedParse()}
    on:reparseWithFeedback={(event) => void workNotes.retrySelectedParseWithFeedback(event.detail)}
    on:deleteNote={() => void deleteSelectedNote()}
    on:restoreNote={() => void restoreSelectedNote()}
    on:permanentlyDeleteNote={() => void permanentlyDeleteSelectedNote()}
    on:acceptAction={(event) => void workNotes.acceptSuggestedAction(event.detail)}
    on:dismissAction={(event) => void workNotes.dismissSuggestedAction(event.detail)}
    on:completeAction={(event) => void workNotes.completeAction(event.detail)}
    on:reopenAction={(event) => void workNotes.reopenAction(event.detail)}
  />
</AppShell>
```

Use the actual component nesting and prop names in `+page.svelte`; the snippet shows the required data flow.

**Verification:**

```powershell
npm run check
```

## Task 8: Run Full Automated Verification

Run these commands from `C:\code\Crazy Projects Go Brrr\work-notes`:

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```

Fix any failing test or type error before continuing.

## Task 9: Manual Browser Verification

Start the app:

```powershell
npm run tauri dev
```

Verify these flows in the running desktop app:

1. Create a note from quick capture.
2. Select the note in Inbox and use `Archive`.
3. Open the `Archive` sidebar item.
4. Confirm the archived note appears.
5. Use `Restore`; confirm the app returns to Inbox and selects the restored note.
6. Archive the same note again.
7. Open Archive again.
8. Click `Delete permanently`, cancel the confirmation, and confirm the note remains.
9. Click `Delete permanently`, confirm, and confirm the note is removed from Archive.
10. Confirm Inbox still loads active notes.

Use the Browser plugin or Playwright if the in-app browser needs direct inspection. The desktop app itself is the source of truth for this manual workflow.

## Task 10: Commit Scope

Check the worktree:

```powershell
git status --short
git diff --stat
```

Stage only the archive/data-management implementation and tests:

```powershell
git add src-tauri/src/repositories/notes.rs src-tauri/src/services/mod.rs src-tauri/src/services/archive.rs src-tauri/src/commands.rs src-tauri/src/lib.rs src/lib/api.ts src/lib/stores/inbox.ts src/lib/components/AppShell.svelte src/lib/components/InboxList.svelte src/lib/components/NoteDetail.svelte src/routes/+page.svelte
git add src/lib/stores/inbox.test.ts src/lib/components/AppShell.test.ts src/lib/components/InboxList.test.ts src/lib/components/NoteDetail.test.ts
```

Commit:

```powershell
git commit -m "feat: add archive data management workflows"
```

If `git status --short` shows unrelated user changes, leave them unstaged and mention them in the handoff.

## Handoff Summary Format

After implementation, report:
- Commands run and whether they passed.
- Manual workflow result.
- Commit hash.
- Any intentionally unaddressed scope from the approved design.
