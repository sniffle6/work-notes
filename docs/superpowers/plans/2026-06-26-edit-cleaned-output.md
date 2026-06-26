# Edit Cleaned Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user edit a note's cleaned title/summary/body, mark the note user-edited, and warn before a reparse discards those edits.

**Architecture:** A persisted `cleaned_edited` flag on the note (set on manual save, cleared on every parser write) flows DB → domain → `NoteDetailDto` → frontend. A new `update_note_cleaned` command persists edits via a repository write that sets the flag and reindexes FTS. The UI gains a `CleanedEditor` form, an "edited" indicator, and a confirm before reparsing an edited note.

**Tech Stack:** Rust (rusqlite/Tauri, inline `#[cfg(test)]` tests), Svelte 5 runes, TypeScript, Vitest + Testing Library Svelte.

## Global Constraints

- `cleaned_edited` semantics: `1`/`true` = user-owned, `0`/`false` = parser-owned. A manual save sets it to `1`; EVERY parser write (`RepositoryParserResultSink::apply_cleaned_text`) sets it to `0`.
- Editable fields are `title`, `summary`, and the cleaned markdown `cleanedText` body. The raw note is never modified.
- The reparse confirmation is enforced in the UI only (reparse is always user-initiated); the backend reparse path keeps overwriting as today.
- Title is normalized via the existing `normalize_title` (empty → default "Untitled note"); body and summary are stored as given.
- The new flag is exposed only on the single-note path (`get` → `Note` → `NoteDetailDto`), not on the inbox list path.
- Editing is offered only when cleaned output already exists (`cleanedText` present).

---

### Task 1: Migration, domain field, and single-note read path

**Files:**
- Modify: `src-tauri/src/db/migrations.rs` (add a column migration)
- Modify: `src-tauri/src/domain.rs` (add `cleaned_edited` to `Note`)
- Modify: `src-tauri/src/repositories/notes.rs` (`NoteRecord`, `from_row`, `into_note`, the `get` SELECT)
- Modify: `src-tauri/src/commands.rs` (`NoteDetailDto` + its `From<NoteDetail>`)
- Test: inline `#[cfg(test)]` tests in `src-tauri/src/repositories/notes.rs`

**Interfaces:**
- Produces: `Note.cleaned_edited: bool`; `NoteDetailDto.cleaned_edited: bool` (serialized as `cleanedEdited`).

- [ ] **Step 1: Add the migration**

In `src-tauri/src/db/migrations.rs`, after the existing `ensure_column(...)` calls (right after the `followup_lane` one), add:

```rust
    ensure_column(
        connection,
        "notes",
        "cleaned_edited",
        "cleaned_edited INTEGER NOT NULL DEFAULT 0",
    )?;
```

- [ ] **Step 2: Add the domain field**

In `src-tauri/src/domain.rs`, add `cleaned_edited` to the `Note` struct (after `is_archived`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub raw_text: String,
    pub cleaned_text: Option<String>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub capture_source: String,
    pub parse_status: ParseStatus,
    pub review_status: ReviewStatus,
    pub is_archived: bool,
    pub cleaned_edited: bool,
}
```

- [ ] **Step 3: Read the column in the repository**

In `src-tauri/src/repositories/notes.rs`:

In the `get` method, add `cleaned_edited` to the SELECT column list:

```rust
            "SELECT id, title, raw_text, cleaned_text, summary, created_at, updated_at,
                    capture_source, parse_status, review_status, is_archived, cleaned_edited
             FROM notes
             WHERE id = ?1",
```

In `NoteRecord`, add the field (after `is_archived`):

```rust
    is_archived: i64,
    cleaned_edited: i64,
```

In `NoteRecord::from_row`, read column index 11 (after `is_archived` at 10):

```rust
            is_archived: row.get(10)?,
            cleaned_edited: row.get(11)?,
```

In `NoteRecord::into_note`, map it (after `is_archived`):

```rust
            is_archived: self.is_archived != 0,
            cleaned_edited: self.cleaned_edited != 0,
```

`NoteRecord::from_row` now reads column index 11, so EVERY query that maps with `NoteRecord::from_row` must select `cleaned_edited` as its 12th column or it will fail at runtime (the build will not catch this). Search the file for `NoteRecord::from_row` and add `cleaned_edited` to the SELECT column list of each such query (the `get` method above is one; update any others the same way).

- [ ] **Step 4: Expose it on the DTO**

In `src-tauri/src/commands.rs`, add the field to `NoteDetailDto` (after `is_archived`):

```rust
    pub is_archived: bool,
    pub cleaned_edited: bool,
```

And in its `From<NoteDetail>` impl, map it (after `is_archived: note.is_archived,`):

```rust
            is_archived: note.is_archived,
            cleaned_edited: note.cleaned_edited,
```

- [ ] **Step 5: Fix remaining `Note` constructors so the crate compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`

The only production reader is `into_note` (done above). For every remaining `missing field cleaned_edited in initializer of ... Note` error (these are test fixtures and any other `Note { ... }` literals), add `cleaned_edited: false,` to that initializer. Re-run until the build is clean.

- [ ] **Step 6: Write a test for the default**

Add a test to the existing `#[cfg(test)] mod tests` in `src-tauri/src/repositories/notes.rs`. Use the same in-memory DB + repository setup the other tests in this file already use (mirror an existing test that creates a note and calls `get`). Assert a freshly created note reads back `cleaned_edited == false`:

```rust
    #[test]
    fn new_note_defaults_to_not_cleaned_edited() {
        // ... set up the repository and insert a note exactly as the other
        //     tests in this file do, capturing its NoteId as `id` ...
        let stored = repository.get(id).expect("get note").expect("note exists");
        assert!(!stored.cleaned_edited);
    }
```

- [ ] **Step 7: Run the tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml notes::`
Expected: PASS, including the new test.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/db/migrations.rs src-tauri/src/domain.rs src-tauri/src/repositories/notes.rs src-tauri/src/commands.rs
git commit -m "feat: add cleaned_edited flag to notes read path"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

### Task 2: Backend write path, parser reset, and command

**Files:**
- Modify: `src-tauri/src/repositories/notes.rs` (new `update_cleaned_by_user`)
- Modify: `src-tauri/src/services/parse_queue.rs` (sink resets the flag to 0)
- Modify: `src-tauri/src/commands.rs` (new `update_note_cleaned` command)
- Modify: `src-tauri/src/lib.rs` (register the command)
- Test: inline tests in `notes.rs` and `parse_queue.rs`

**Interfaces:**
- Consumes: `Note.cleaned_edited` (Task 1); `apply_cleaned_note` / `replace_fts` / `normalize_title` patterns already in `notes.rs`; `NoteDetailDto` (Task 1); `SearchService::get_note`.
- Produces: `NoteRepository::update_cleaned_by_user(id: NoteId, title: &str, cleaned_text: &str, summary: &str) -> RepositoryResult<()>`; Tauri command `update_note_cleaned(note_id, title, cleaned_text, summary) -> Result<NoteDetailDto, CommandError>`.

- [ ] **Step 1: Write the failing repository test**

Add to the `#[cfg(test)] mod tests` in `src-tauri/src/repositories/notes.rs`, mirroring the existing setup that creates a parsed note (an existing test already exercises `apply_cleaned_note`; copy its setup):

```rust
    #[test]
    fn update_cleaned_by_user_sets_fields_and_marks_edited() {
        // ... set up the repository and insert a note as the existing
        //     apply_cleaned_note test does, capturing its NoteId as `id` ...
        repository
            .update_cleaned_by_user(id, "My Title", "## Edited body", "Edited summary")
            .expect("update cleaned by user");

        let stored = repository.get(id).expect("get note").expect("note exists");
        assert_eq!(stored.title, "My Title");
        assert_eq!(stored.cleaned_text.as_deref(), Some("## Edited body"));
        assert_eq!(stored.summary.as_deref(), Some("Edited summary"));
        assert!(stored.cleaned_edited);
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml update_cleaned_by_user_sets_fields_and_marks_edited`
Expected: FAIL to compile — `update_cleaned_by_user` does not exist yet.

- [ ] **Step 3: Implement the repository write**

In `src-tauri/src/repositories/notes.rs`, add this method to the same `impl` block that contains `apply_cleaned_note` (it mirrors `apply_cleaned_note`, but sets `cleaned_edited = 1`):

```rust
    pub fn update_cleaned_by_user(
        &self,
        id: NoteId,
        title: &str,
        cleaned_text: &str,
        summary: &str,
    ) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;

        let raw_text = transaction
            .query_row(
                "SELECT raw_text FROM notes WHERE id = ?1",
                [id_text.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| RepositoryError::NotFound {
                entity: "note",
                id: id.to_string(),
            })?;

        transaction.execute(
            "UPDATE notes
             SET title = ?2, cleaned_text = ?3, summary = ?4, cleaned_edited = 1, updated_at = ?5
             WHERE id = ?1",
            params![id_text, normalize_title(title), cleaned_text, summary, now],
        )?;
        replace_fts(
            &transaction,
            &id_text,
            &raw_text,
            Some(cleaned_text),
            Some(summary),
        )?;
        transaction.commit()?;
        Ok(())
    }
```

- [ ] **Step 4: Run the repository test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml update_cleaned_by_user_sets_fields_and_marks_edited`
Expected: PASS.

- [ ] **Step 5: Write the failing parser-reset test**

Add a test to the `#[cfg(test)] mod tests` in `src-tauri/src/services/parse_queue.rs`. Mirror the existing `process_next_with_provider_applies_successful_parse` test setup, but first mark the note user-edited, then run a successful parse, then assert the flag is reset:

```rust
    #[test]
    fn successful_parse_resets_cleaned_edited_flag() {
        // ... set up exactly as process_next_with_provider_applies_successful_parse
        //     does (repositories, a note with a queued parse job, a provider that
        //     returns a successful ParserOutput), capturing the note id as `note_id` ...
        repositories
            .notes
            .update_cleaned_by_user(note_id, "User Title", "user body", "user summary")
            .expect("mark edited");
        assert!(
            repositories.notes.get(note_id).unwrap().unwrap().cleaned_edited,
            "precondition: note is user-edited"
        );

        queue
            .process_next_with_provider(&provider)
            .expect("process parse");

        let stored = repositories.notes.get(note_id).unwrap().unwrap();
        assert!(!stored.cleaned_edited, "parser write reclaims ownership");
    }
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml successful_parse_resets_cleaned_edited_flag`
Expected: FAIL — the sink does not reset `cleaned_edited`, so it stays `true`.

- [ ] **Step 7: Reset the flag in the parser sink**

In `src-tauri/src/services/parse_queue.rs`, in `RepositoryParserResultSink::apply_cleaned_text`, add `cleaned_edited = 0` to the UPDATE:

```rust
        self.transaction.execute(
            "UPDATE notes
             SET title = ?2, cleaned_text = ?3, summary = ?4, cleaned_edited = 0, updated_at = ?5
             WHERE id = ?1",
            params![
                note_id_text,
                normalize_title(title),
                cleaned_text,
                summary,
                self.updated_at
            ],
        )?;
```

- [ ] **Step 8: Run the parser-reset test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml successful_parse_resets_cleaned_edited_flag`
Expected: PASS.

- [ ] **Step 9: Add the command and register it**

In `src-tauri/src/commands.rs`, add the command (mirrors `get_note`'s return shape and `retry_parse`'s state access):

```rust
#[tauri::command]
pub async fn update_note_cleaned(
    state: tauri::State<'_, AppState>,
    note_id: String,
    title: String,
    cleaned_text: String,
    summary: String,
) -> Result<NoteDetailDto, CommandError> {
    let note_id = parse_note_id(&note_id)?;
    state
        .repositories
        .notes
        .update_cleaned_by_user(note_id, &title, &cleaned_text, &summary)?;
    let detail = SearchService::new(state.repositories.clone()).get_note(note_id)?;
    Ok(detail.into())
}
```

In `src-tauri/src/lib.rs`, add the command to the `tauri::generate_handler![ ... ]` list (after `commands::retry_parse_with_feedback,`):

```rust
            commands::retry_parse_with_feedback,
            commands::update_note_cleaned,
```

- [ ] **Step 10: Run the backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — full backend suite green, including the two new tests.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/repositories/notes.rs src-tauri/src/services/parse_queue.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: persist user edits to cleaned output and reset on reparse"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

### Task 3: Frontend type, API, and store action

**Files:**
- Modify: `src/lib/types.ts` (`NoteDetail` gains `cleanedEdited`)
- Modify: `src/lib/api.ts` (normalizer mapping, `updateNoteCleaned`, fallback, `api` object)
- Modify: `src/lib/stores/inbox.ts` (`saveCleanedEdits` action + return object)
- Test: `src/lib/stores/inbox.test.ts`

**Interfaces:**
- Produces: `api.updateNoteCleaned(noteId, { title, summary, cleanedText }): Promise<NoteDetail>`; store `saveCleanedEdits(fields: { title: string; summary: string; cleanedText: string }): Promise<void>` (rejects on error after setting the error store); `NoteDetail.cleanedEdited?: boolean`.

- [ ] **Step 1: Add the type field**

In `src/lib/types.ts`, extend the `NoteDetail` type with the optional flag:

```typescript
export type NoteDetail = NoteListItem & {
  actionItems: ActionItem[];
  parseError?: string | null;
  cleanedEdited?: boolean;
};
```

- [ ] **Step 2: Map the field in the normalizer**

In `src/lib/api.ts`, in `normalizeNoteDetail`, add `cleanedEdited` to the returned object (after `parseError`). Use the existing `getBoolean` helper (already used for `isArchived`):

```typescript
  return {
    ...base,
    actionItems,
    suggestedActionItemCount: actionItems.filter((item) => item.status === "suggested").length,
    actionItemCount: actionItems.length || base.actionItemCount,
    parseError: getNullableString(record, "parseError", "parse_error", "lastError", "last_error"),
    cleanedEdited: getBoolean(record, "cleanedEdited", "cleaned_edited") ?? false,
  };
```

- [ ] **Step 3: Add the API function and dev fallback**

In `src/lib/api.ts`, add this function next to `retryParseWithFeedback`:

```typescript
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
```

Add a `fallbackCommand` case (next to the `retry_parse_with_feedback` case) so dev mode works:

```typescript
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
```

Add `updateNoteCleaned` to the `export const api = { ... }` object (after `retryParseWithFeedback`):

```typescript
  retryParseWithFeedback,
  updateNoteCleaned,
```

- [ ] **Step 4: Write the failing store test**

In `src/lib/stores/inbox.test.ts`, add `updateNoteCleaned` to the `TestApi` type and the `testApi` factory (after `retryParseWithFeedback`):

```typescript
    updateNoteCleaned: vi
      .fn<(noteId: string, fields: { title: string; summary: string; cleanedText: string }) => Promise<NoteDetail>>()
      .mockResolvedValue(detail),
```

Then add the test:

```typescript
  it("saves cleaned edits and reloads the selected note", async () => {
    const api = testApi();
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.saveCleanedEdits({ title: "New title", summary: "New summary", cleanedText: "New body" });

    expect(api.updateNoteCleaned).toHaveBeenCalledWith("note-1", {
      title: "New title",
      summary: "New summary",
      cleanedText: "New body",
    });
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
  });
```

- [ ] **Step 5: Run it to verify it fails**

Run: `npm run test -- inbox`
Expected: FAIL — `store.saveCleanedEdits` is not a function / not defined.

- [ ] **Step 6: Implement the store action**

In `src/lib/stores/inbox.ts`, add this action next to `retrySelectedParseWithFeedback` (mirrors `createFollowupFromSelectedNote`: rethrows on error so the caller can keep the editor open):

```typescript
  async function saveCleanedEdits(fields: {
    title: string;
    summary: string;
    cleanedText: string;
  }): Promise<void> {
    const note = get(selectedNote);
    if (!note) {
      return;
    }

    loadingNote.set(true);
    error.set(null);

    try {
      await api.updateNoteCleaned(note.id, fields);
      await loadInbox();
      selectedNote.set(await api.getNote(note.id));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not save edits."));
      throw unknownError;
    } finally {
      loadingNote.set(false);
    }
  }
```

Add `saveCleanedEdits` to the store's return object (after `retrySelectedParseWithFeedback`):

```typescript
    retrySelectedParseWithFeedback,
    saveCleanedEdits,
```

- [ ] **Step 7: Run the store test to verify it passes**

Run: `npm run test -- inbox`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/lib/stores/inbox.ts src/lib/stores/inbox.test.ts
git commit -m "feat: add updateNoteCleaned api and saveCleanedEdits store action"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

### Task 4: `CleanedEditor` component

**Files:**
- Create: `src/lib/components/CleanedEditor.svelte`
- Test: `src/lib/components/CleanedEditor.test.ts`

**Interfaces:**
- Produces: a Svelte component with props `{ title: string; summary: string; cleanedText: string }` that emits `save` with `{ title: string; summary: string; cleanedText: string }` and `cancel` (void).

- [ ] **Step 1: Write the failing test**

Create `src/lib/components/CleanedEditor.test.ts`:

```typescript
// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import CleanedEditor from "./CleanedEditor.svelte";

afterEach(() => cleanup());

describe("CleanedEditor", () => {
  it("emits save with the edited title, summary, and body", async () => {
    const save = vi.fn();
    render(CleanedEditor, {
      props: { title: "Old title", summary: "Old summary", cleanedText: "Old body" },
      events: { save },
    });

    await fireEvent.input(screen.getByLabelText("Edit title"), { target: { value: "New title" } });
    await fireEvent.input(screen.getByLabelText("Edit summary"), { target: { value: "New summary" } });
    await fireEvent.input(screen.getByLabelText("Edit cleaned note"), { target: { value: "New body" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(save).toHaveBeenCalledTimes(1);
    expect(save.mock.calls[0][0].detail).toEqual({
      title: "New title",
      summary: "New summary",
      cleanedText: "New body",
    });
  });

  it("emits cancel without saving", async () => {
    const cancel = vi.fn();
    const save = vi.fn();
    render(CleanedEditor, {
      props: { title: "T", summary: "S", cleanedText: "B" },
      events: { cancel, save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(cancel).toHaveBeenCalledTimes(1);
    expect(save).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `npm run test -- CleanedEditor`
Expected: FAIL — `CleanedEditor.svelte` does not exist.

- [ ] **Step 3: Implement the component**

Create `src/lib/components/CleanedEditor.svelte`:

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";

  type Props = {
    title: string;
    summary: string;
    cleanedText: string;
  };

  let { title, summary, cleanedText }: Props = $props();

  let titleValue = $state(title);
  let summaryValue = $state(summary);
  let bodyValue = $state(cleanedText);

  const dispatch = createEventDispatcher<{
    save: { title: string; summary: string; cleanedText: string };
    cancel: void;
  }>();

  function save() {
    dispatch("save", {
      title: titleValue,
      summary: summaryValue,
      cleanedText: bodyValue,
    });
  }
</script>

<div class="cleaned-editor">
  <label>
    <span>Title</span>
    <input aria-label="Edit title" bind:value={titleValue} />
  </label>
  <label>
    <span>Summary</span>
    <textarea aria-label="Edit summary" bind:value={summaryValue} rows="2"></textarea>
  </label>
  <label>
    <span>Cleaned note</span>
    <textarea aria-label="Edit cleaned note" bind:value={bodyValue} rows="12"></textarea>
  </label>
  <footer>
    <button class="secondary-action" type="button" onclick={() => dispatch("cancel")}>Cancel</button>
    <button class="primary-action" type="button" onclick={save}>Save</button>
  </footer>
</div>

<style>
  .cleaned-editor {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .cleaned-editor label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .cleaned-editor label span {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .cleaned-editor input,
  .cleaned-editor textarea {
    background: var(--surface-input);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    color: var(--text-primary);
    padding: 0.5rem;
    font: inherit;
  }

  .cleaned-editor textarea {
    resize: vertical;
  }

  .cleaned-editor footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npm run test -- CleanedEditor`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/CleanedEditor.svelte src/lib/components/CleanedEditor.test.ts
git commit -m "feat: add CleanedEditor form component"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

### Task 5: NoteDetail integration and page wiring

**Files:**
- Modify: `src/lib/components/NoteDetail.svelte`
- Modify: `src/routes/+page.svelte`
- Test: `src/lib/components/NoteDetail.test.ts`

**Interfaces:**
- Consumes: `CleanedEditor` (Task 4); `note.cleanedEdited` (Task 3); `workNotes.saveCleanedEdits` (Task 3).
- Produces: a `saveCleaned` dispatched event `{ title: string; summary: string; cleanedText: string; done: () => void }`; an "Edit" affordance and an edited indicator; a reparse confirmation for edited notes.

- [ ] **Step 1: Write the failing tests**

In `src/lib/components/NoteDetail.test.ts`, add these tests (the `noteDetail()` helper builds a `failed` note with `cleanedText: null`; pass overrides for a parsed, optionally edited note):

```typescript
  it("shows an Edit button only when cleaned output exists", () => {
    const { unmount } = render(NoteDetail, {
      props: { note: { ...noteDetail(), parseStatus: "parsed", cleanedText: "## Body", parseError: null } },
    });
    expect(screen.getByRole("button", { name: "Edit" })).toBeTruthy();
    unmount();

    render(NoteDetail, { props: { note: { ...noteDetail(), parseStatus: "parsed", cleanedText: null } } });
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
  });

  it("dispatches saveCleaned with edited values and closes on done", async () => {
    const saveCleaned = vi.fn();
    render(NoteDetail, {
      props: { note: { ...noteDetail(), parseStatus: "parsed", cleanedText: "## Body", parseError: null } },
      events: { saveCleaned },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await fireEvent.input(screen.getByLabelText("Edit title"), { target: { value: "Edited title" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(saveCleaned).toHaveBeenCalledTimes(1);
    expect(saveCleaned.mock.calls[0][0].detail.title).toBe("Edited title");

    saveCleaned.mock.calls[0][0].detail.done();
    await tick();
    expect(screen.queryByLabelText("Edit title")).toBeNull();
  });

  it("confirms before reparsing a user-edited note", async () => {
    const retryParse = vi.fn();
    render(NoteDetail, {
      props: {
        note: {
          ...noteDetail(),
          parseStatus: "parsed",
          cleanedText: "## Body",
          parseError: null,
          cleanedEdited: true,
        },
      },
      events: { retryParse },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Reparse" }));
    expect(retryParse).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog", { name: "Discard manual edits?" })).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Discard & reparse" }));
    expect(retryParse).toHaveBeenCalledTimes(1);
  });

  it("reparses an unedited note without confirmation", async () => {
    const retryParse = vi.fn();
    render(NoteDetail, {
      props: {
        note: { ...noteDetail(), parseStatus: "parsed", cleanedText: "## Body", parseError: null, cleanedEdited: false },
      },
      events: { retryParse },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Reparse" }));
    expect(retryParse).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog", { name: "Discard manual edits?" })).toBeNull();
  });
```

- [ ] **Step 2: Run them to verify they fail**

Run: `npm run test -- NoteDetail`
Expected: FAIL — there is no Edit button, no `saveCleaned` event, no reparse confirm, and no plain "Reparse" button yet.

- [ ] **Step 3: Wire the editor, indicator, and confirm into NoteDetail**

In `src/lib/components/NoteDetail.svelte`:

Import the editor (with the other imports):

```svelte
  import CleanedEditor from "./CleanedEditor.svelte";
```

Add state and a pending-reparse holder (with the other `$state` declarations):

```svelte
  let editing = $state(false);
  let reparseConfirmOpen = $state(false);
  let pendingReparse = $state<{ kind: "retry" } | { kind: "feedback"; feedback: string } | null>(null);
```

Extend the dispatcher type with `saveCleaned` (add this entry to the existing `createEventDispatcher<{ ... }>()` map):

```svelte
    saveCleaned: { title: string; summary: string; cleanedText: string; done: () => void };
```

Add these functions (next to `dispatchReparseWithFeedback`):

```svelte
  function startEdit() {
    editing = true;
  }

  function onEditorSave(event: CustomEvent<{ title: string; summary: string; cleanedText: string }>) {
    dispatch("saveCleaned", {
      title: event.detail.title,
      summary: event.detail.summary,
      cleanedText: event.detail.cleanedText,
      done: () => {
        editing = false;
      },
    });
  }

  function requestRetry() {
    if (note?.cleanedEdited) {
      pendingReparse = { kind: "retry" };
      reparseConfirmOpen = true;
    } else {
      dispatch("retryParse");
    }
  }

  function confirmReparse() {
    if (pendingReparse?.kind === "retry") {
      dispatch("retryParse");
    } else if (pendingReparse?.kind === "feedback") {
      dispatch("reparseWithFeedback", pendingReparse.feedback);
      reparseFeedback = "";
    }
    pendingReparse = null;
    reparseConfirmOpen = false;
  }

  function cancelReparseConfirm() {
    pendingReparse = null;
    reparseConfirmOpen = false;
  }
```

Change `dispatchReparseWithFeedback` so an edited note routes through the confirm instead of dispatching immediately:

```svelte
  function dispatchReparseWithFeedback() {
    const feedback = reparseFeedback.trim();
    if (!feedback) {
      return;
    }
    reparseOpen = false;
    if (note?.cleanedEdited) {
      pendingReparse = { kind: "feedback", feedback };
      reparseConfirmOpen = true;
    } else {
      dispatch("reparseWithFeedback", feedback);
      reparseFeedback = "";
    }
  }
```

Replace the two existing `onclick={() => dispatch("retryParse")}` handlers (the header Retry button and the failed-banner Retry button) with `onclick={requestRetry}`. Also add a always-available **Reparse** button next to them in the header so an edited, successfully-parsed note can be reparsed (the existing header only shows Retry when `parseStatus === "failed"`); place this right after the existing `{#if note.parseStatus === "failed"} ... {/if}` Retry block in the header button row:

```svelte
        <button class="ghost-button" type="button" onclick={requestRetry} disabled={loading}>Reparse</button>
```

In the note-body section, render the editor when editing, add the Edit button and the edited indicator on the cleaned view. Replace the existing cleaned-view block with:

```svelte
      <section class="note-body" aria-label={showRaw ? "Raw note" : "Cleaned note"}>
        {#if showRaw}
          <pre>{note.rawText}</pre>
        {:else if editing}
          <CleanedEditor
            title={note.title}
            summary={note.summary ?? ""}
            cleanedText={note.cleanedText ?? ""}
            on:save={onEditorSave}
            on:cancel={() => (editing = false)}
          />
        {:else if note.cleanedText}
          <div class="cleaned-toolbar">
            {#if note.cleanedEdited}
              <span class="edited-indicator">edited</span>
            {/if}
            <button class="ghost-button" type="button" onclick={startEdit} disabled={loading}>Edit</button>
          </div>
          <MarkdownView markdown={note.cleanedText} />
        {:else}
          <p>Waiting for parser output.</p>
        {/if}
      </section>
```

Add the confirm dialog near the reparse dialog block (after the `{#if reparseOpen} ... {/if}` block):

```svelte
    {#if reparseConfirmOpen}
      <div class="reparse-backdrop" aria-hidden="true" onclick={cancelReparseConfirm}></div>
      <div class="reparse-dialog" role="dialog" aria-modal="true" aria-labelledby="reparse-confirm-title">
        <header>
          <h2 id="reparse-confirm-title">Discard manual edits?</h2>
        </header>
        <p>This note has manual edits. Reparsing will replace them with new parser output.</p>
        <footer>
          <button class="secondary-action" type="button" onclick={cancelReparseConfirm}>Keep my edits</button>
          <button class="primary-action" type="button" onclick={confirmReparse}>Discard & reparse</button>
        </footer>
      </div>
    {/if}
```

Add styles (with the other component styles):

```svelte
  .cleaned-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .edited-indicator {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    border: 1px solid var(--border-default);
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
  }
```

- [ ] **Step 4: Run the NoteDetail tests to verify they pass**

Run: `npm run test -- NoteDetail`
Expected: PASS — the four new tests and the existing ones.

- [ ] **Step 5: Wire the page handler**

In `src/routes/+page.svelte`, add the `on:saveCleaned` handler to the `<NoteDetail ... />` element (after `on:createFollowup`):

```svelte
            on:createFollowup={(event) => void createFollowupFromNote(event)}
            on:saveCleaned={(event) => void saveCleanedFromNote(event)}
```

Add the handler function in the `<script>` (next to `createFollowupFromNote`, mirroring its done-on-success pattern):

```typescript
  async function saveCleanedFromNote(
    event: CustomEvent<{ title: string; summary: string; cleanedText: string; done: () => void }>,
  ) {
    try {
      await workNotes.saveCleanedEdits({
        title: event.detail.title,
        summary: event.detail.summary,
        cleanedText: event.detail.cleanedText,
      });
      event.detail.done();
    } catch {
      // The store exposes the error; keep the editor open so the user can retry.
    }
  }
```

- [ ] **Step 6: Verify the full frontend suite and type-check**

Run: `npm run test`
Expected: PASS — all frontend tests green.

Run: `npm run check`
Expected: 0 errors, 0 warnings (svelte-check).

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/NoteDetail.svelte src/lib/components/NoteDetail.test.ts src/routes/+page.svelte
git commit -m "feat: edit cleaned output with reparse confirmation in note detail"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`
