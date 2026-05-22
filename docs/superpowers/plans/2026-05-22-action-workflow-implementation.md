# Action Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the action item lifecycle from parser suggestion review through acceptance, completion, reopening, and cross-note review queue triage.

**Architecture:** Keep lifecycle rules in a new Rust `ActionItemService`, with repositories owning SQLite details and commands exposing narrow workflow operations. Frontend workflow sequencing stays in `src/lib/stores/inbox.ts`; Svelte components emit UI events and stay persistence-free.

**Tech Stack:** Tauri v2, Rust, SQLite/rusqlite, Svelte 5, TypeScript, Vitest, Testing Library Svelte.

---

## File Structure

- Create `src-tauri/src/services/actions.rs`: backend action lifecycle service and service-level tests.
- Modify `src-tauri/src/services/mod.rs`: export the new service module.
- Modify `src-tauri/src/domain.rs`: add `ActionReviewItem`, the typed review queue read model.
- Modify `src-tauri/src/repositories/actions.rs`: add action lookup, suggested-action existence check, and suggested review queue query.
- Modify `src-tauri/src/commands.rs`: route action commands through the service, add complete/reopen/list commands, and add review queue DTO tests.
- Modify `src-tauri/src/lib.rs`: register the new Tauri commands.
- Modify `src/lib/types.ts`: add `ActionReviewItem`.
- Modify `src/lib/api.ts`: add action lifecycle API wrappers, fallback command handling, and review queue normalization.
- Modify `src/lib/stores/inbox.ts`: add suggested action queue state and complete/reopen flows.
- Modify `src/lib/stores/inbox.test.ts`: cover queue loading and action status refresh behavior.
- Modify `src/lib/components/NoteDetail.svelte`: emit complete/reopen events and render accepted/done action controls.
- Modify `src/lib/components/NoteDetail.test.ts`: cover complete/reopen events.
- Modify `src/lib/components/ReviewQueue.svelte`: consume `ActionReviewItem`, emit select/accept/dismiss, and render note context.
- Create `src/lib/components/ReviewQueue.test.ts`: cover queue rendering and emitted events.
- Modify `src/routes/+page.svelte`: load and mount the review queue and wire action lifecycle events.

Implementation must not stage the existing unstaged docs-pass edits unless the user explicitly asks. At plan creation time those files were `README.md`, `docs/architecture.md`, `docs/development.md`, `docs/superpowers/specs/2026-05-20-work-notes-design.md`, `docs/testing.md`, and `docs/verification/2026-05-20-v1-verification.md`.

---

### Task 1: Backend Review Read Model And Repository Queries

**Files:**
- Modify: `src-tauri/src/domain.rs`
- Modify: `src-tauri/src/repositories/actions.rs`
- Test: `src-tauri/src/repositories/mod.rs`

- [ ] **Step 1: Add the review queue read model**

In `src-tauri/src/domain.rs`, add this struct after `ActionItem`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReviewItem {
    pub id: ActionItemId,
    pub note_id: NoteId,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Write repository tests for lookup and review queue ordering**

In `src-tauri/src/repositories/mod.rs`, add this test inside the existing `#[cfg(test)] mod tests` module:

```rust
#[test]
fn action_repository_gets_actions_and_lists_suggested_with_note_context() {
    let db = test_db();
    let notes = NoteRepository::new(db.clone());
    let actions = ActionItemRepository::new(db);

    let older = notes.create_raw_note("Older action note").unwrap();
    let newer = notes.create_raw_note("Newer action note").unwrap();
    let older_action = actions
        .create_suggested(
            older.id,
            "Follow up on older note",
            Some("Alice"),
            None,
            Some(0.72),
        )
        .unwrap();
    let newer_action = actions
        .create_suggested(
            newer.id,
            "Follow up on newer note",
            Some("Maya"),
            Some("2026-05-23"),
            Some(0.91),
        )
        .unwrap();
    let accepted = actions
        .create_suggested(newer.id, "Already accepted", None, None, Some(0.55))
        .unwrap();
    actions
        .set_status(accepted.id, ActionStatus::Accepted)
        .unwrap();

    let stored = actions.get(newer_action.id).unwrap().unwrap();
    assert_eq!(stored.text, "Follow up on newer note");
    assert_eq!(stored.owner.as_deref(), Some("Maya"));
    assert_eq!(stored.status, ActionStatus::Suggested);
    assert!(actions.has_suggested_for_note(newer.id).unwrap());

    let review_items = actions.list_suggested_with_note_context(10).unwrap();
    assert_eq!(review_items.len(), 2);
    assert_eq!(review_items[0].id, newer_action.id);
    assert_eq!(review_items[0].note_id, newer.id);
    assert_eq!(review_items[0].note_title, newer.title);
    assert_eq!(review_items[0].due_date.as_deref(), Some("2026-05-23"));
    assert_eq!(review_items[1].id, older_action.id);
    assert_eq!(review_items[1].note_title, older.title);

    actions
        .set_status(older_action.id, ActionStatus::Dismissed)
        .unwrap();
    assert!(!actions.has_suggested_for_note(older.id).unwrap());
}
```

- [ ] **Step 3: Run the failing repository test**

Run:

```powershell
scripts\cargo-test.cmd action_repository_gets_actions_and_lists_suggested_with_note_context
```

Expected: FAIL with missing `get`, `has_suggested_for_note`, and `list_suggested_with_note_context` methods or missing `ActionReviewItem`.

- [ ] **Step 4: Implement repository methods**

In `src-tauri/src/repositories/actions.rs`, update the imports:

```rust
use rusqlite::{params, OptionalExtension, Row};

use crate::db::Database;
use crate::domain::{
    ActionItem, ActionItemId, ActionReviewItem, ActionStatus, NoteId,
};

use super::{parse_db_datetime, RepositoryError, RepositoryResult};
```

Add these methods inside `impl ActionItemRepository` after `create` and before `list_for_note`:

```rust
    pub fn get(&self, id: ActionItemId) -> RepositoryResult<Option<ActionItem>> {
        let connection = self.db.connection()?;
        let record = connection
            .query_row(
                "SELECT id, note_id, text, owner, due_date, status, source, confidence
                 FROM action_items
                 WHERE id = ?1",
                [id.to_string()],
                ActionItemRecord::from_row,
            )
            .optional()?;

        record.map(ActionItemRecord::into_action_item).transpose()
    }

    pub fn has_suggested_for_note(&self, note_id: NoteId) -> RepositoryResult<bool> {
        let connection = self.db.connection()?;
        let count = connection.query_row(
            "SELECT COUNT(*)
             FROM action_items
             WHERE note_id = ?1 AND status = 'suggested'",
            [note_id.to_string()],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(count > 0)
    }

    pub fn list_suggested_with_note_context(
        &self,
        limit: u32,
    ) -> RepositoryResult<Vec<ActionReviewItem>> {
        let connection = self.db.connection()?;
        let limit = i64::from(limit.clamp(1, 500));
        let mut statement = connection.prepare(
            "SELECT
                ai.id,
                ai.note_id,
                n.title,
                ai.text,
                ai.owner,
                ai.due_date,
                ai.confidence,
                n.created_at
             FROM action_items ai
             JOIN notes n ON n.id = ai.note_id
             WHERE ai.status = 'suggested' AND n.is_archived = 0
             ORDER BY n.created_at DESC, ai.rowid ASC
             LIMIT ?1",
        )?;

        let records = statement
            .query_map([limit], ActionReviewItemRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        records
            .into_iter()
            .map(ActionReviewItemRecord::into_review_item)
            .collect()
    }
```

Add this record type after `ActionItemRecord`:

```rust
struct ActionReviewItemRecord {
    id: String,
    note_id: String,
    note_title: String,
    text: String,
    owner: Option<String>,
    due_date: Option<String>,
    confidence: Option<f64>,
    created_at: String,
}

impl ActionReviewItemRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            note_id: row.get(1)?,
            note_title: row.get(2)?,
            text: row.get(3)?,
            owner: row.get(4)?,
            due_date: row.get(5)?,
            confidence: row.get(6)?,
            created_at: row.get(7)?,
        })
    }

    fn into_review_item(self) -> RepositoryResult<ActionReviewItem> {
        Ok(ActionReviewItem {
            id: ActionItemId::parse(&self.id)?,
            note_id: NoteId::parse(&self.note_id)?,
            note_title: self.note_title,
            text: self.text,
            owner: self.owner,
            due_date: self.due_date,
            confidence: self.confidence,
            created_at: parse_db_datetime("created_at", self.created_at)?,
        })
    }
}
```

- [ ] **Step 5: Run the repository test**

Run:

```powershell
scripts\cargo-test.cmd action_repository_gets_actions_and_lists_suggested_with_note_context
```

Expected: PASS.

- [ ] **Step 6: Commit repository read model**

Run:

```powershell
git add src-tauri/src/domain.rs src-tauri/src/repositories/actions.rs src-tauri/src/repositories/mod.rs
git commit -m "feat: add suggested action review query"
```

---

### Task 2: Backend Action Lifecycle Service

**Files:**
- Create: `src-tauri/src/services/actions.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Create failing service tests**

Create `src-tauri/src/services/actions.rs` with this initial test-focused content:

```rust
use crate::app_state::AppRepositories;
use crate::domain::{ActionItemId, ActionReviewItem, ActionStatus, ReviewStatus};

use super::{ServiceError, ServiceResult};

#[derive(Clone)]
pub struct ActionItemService {
    repositories: AppRepositories,
}

impl ActionItemService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn accept(&self, _id: ActionItemId) -> ServiceResult<()> {
        Err(ServiceError::InvalidInput("action lifecycle stub"))
    }

    pub fn dismiss(&self, _id: ActionItemId) -> ServiceResult<()> {
        Err(ServiceError::InvalidInput("action lifecycle stub"))
    }

    pub fn complete(&self, _id: ActionItemId) -> ServiceResult<()> {
        Err(ServiceError::InvalidInput("action lifecycle stub"))
    }

    pub fn reopen(&self, _id: ActionItemId) -> ServiceResult<()> {
        Err(ServiceError::InvalidInput("action lifecycle stub"))
    }

    pub fn list_suggested(&self, _limit: u32) -> ServiceResult<Vec<ActionReviewItem>> {
        Err(ServiceError::InvalidInput("action lifecycle stub"))
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::domain::{ActionStatus, ReviewStatus};

    use super::{ActionItemService, ServiceError};

    fn repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn accepting_last_suggested_action_marks_note_reviewed() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Ask Maya to review the dashboard")
            .unwrap();
        repositories
            .notes
            .set_review_status(note.id, ReviewStatus::NeedsReview)
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Review the dashboard", Some("Maya"), None, Some(0.9))
            .unwrap();

        ActionItemService::new(repositories.clone())
            .accept(action.id)
            .unwrap();

        let stored_action = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap();
        let stored_note = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored_action.status, ActionStatus::Accepted);
        assert_eq!(stored_note.review_status, ReviewStatus::Reviewed);
    }

    #[test]
    fn dismissing_one_of_multiple_suggested_actions_keeps_note_needing_review() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Two follow ups for Jordan")
            .unwrap();
        repositories
            .notes
            .set_review_status(note.id, ReviewStatus::NeedsReview)
            .unwrap();
        let first = repositories
            .action_items
            .create_suggested(note.id, "Check the export", Some("Jordan"), None, Some(0.8))
            .unwrap();
        repositories
            .action_items
            .create_suggested(note.id, "Send the filtered query", None, None, Some(0.7))
            .unwrap();

        ActionItemService::new(repositories.clone())
            .dismiss(first.id)
            .unwrap();

        let stored_note = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored_note.review_status, ReviewStatus::NeedsReview);
    }

    #[test]
    fn completing_and_reopening_actions_preserves_review_status() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Follow up after accepting action")
            .unwrap();
        repositories
            .notes
            .set_review_status(note.id, ReviewStatus::Reviewed)
            .unwrap();
        let action = repositories
            .action_items
            .create(
                note.id,
                "Follow up with Alice",
                Some("Alice"),
                None,
                ActionStatus::Accepted,
                "parser",
                Some(0.8),
            )
            .unwrap();
        let service = ActionItemService::new(repositories.clone());

        service.complete(action.id).unwrap();
        let completed = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, ActionStatus::Done);
        assert_eq!(
            repositories.notes.get(note.id).unwrap().unwrap().review_status,
            ReviewStatus::Reviewed
        );

        service.reopen(action.id).unwrap();
        let reopened = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap();
        assert_eq!(reopened.status, ActionStatus::Accepted);
    }

    #[test]
    fn invalid_lifecycle_transition_returns_invalid_input() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Cannot complete suggested directly")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Review before complete", None, None, Some(0.8))
            .unwrap();

        let error = ActionItemService::new(repositories)
            .complete(action.id)
            .unwrap_err();

        assert!(matches!(error, ServiceError::InvalidInput("invalid action status transition")));
    }

    #[test]
    fn list_suggested_uses_repository_review_rows() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Maya owns badge printer follow up")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Check badge printer", Some("Maya"), None, Some(0.77))
            .unwrap();

        let items = ActionItemService::new(repositories)
            .list_suggested(100)
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, action.id);
        assert_eq!(items[0].note_title, note.title);
        assert_eq!(items[0].owner.as_deref(), Some("Maya"));
    }
}
```

In `src-tauri/src/services/mod.rs`, add the module export:

```rust
pub mod actions;
```

- [ ] **Step 2: Run the failing service tests**

Run:

```powershell
scripts\cargo-test.cmd services::actions
```

Expected: FAIL because the service methods return `InvalidInput("action lifecycle stub")`.

- [ ] **Step 3: Implement lifecycle transitions**

Replace the placeholder method implementations in `src-tauri/src/services/actions.rs` with this implementation:

```rust
    pub fn accept(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Suggested, ActionStatus::Accepted, true)
    }

    pub fn dismiss(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Suggested, ActionStatus::Dismissed, true)
    }

    pub fn complete(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Accepted, ActionStatus::Done, false)
    }

    pub fn reopen(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Done, ActionStatus::Accepted, false)
    }

    pub fn list_suggested(&self, limit: u32) -> ServiceResult<Vec<ActionReviewItem>> {
        self.repositories
            .action_items
            .list_suggested_with_note_context(limit)
            .map_err(Into::into)
    }

    fn transition(
        &self,
        id: ActionItemId,
        expected: ActionStatus,
        next: ActionStatus,
        resolves_suggestion: bool,
    ) -> ServiceResult<()> {
        let action = self
            .repositories
            .action_items
            .get(id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            })?;

        if action.status != expected {
            return Err(ServiceError::InvalidInput("invalid action status transition"));
        }

        self.repositories.action_items.set_status(id, next)?;

        if resolves_suggestion {
            self.refresh_note_review_status(action.note_id)?;
        }

        Ok(())
    }

    fn refresh_note_review_status(&self, note_id: crate::domain::NoteId) -> ServiceResult<()> {
        if !self.repositories.action_items.has_suggested_for_note(note_id)? {
            self.repositories
                .notes
                .set_review_status(note_id, ReviewStatus::Reviewed)?;
        }

        Ok(())
    }
```

- [ ] **Step 4: Run the service tests**

Run:

```powershell
scripts\cargo-test.cmd services::actions
```

Expected: PASS.

- [ ] **Step 5: Commit the service**

Run:

```powershell
git add src-tauri/src/services/actions.rs src-tauri/src/services/mod.rs
git commit -m "feat: add action lifecycle service"
```

---

### Task 3: Tauri Commands For Action Lifecycle

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add command DTO test coverage**

In `src-tauri/src/commands.rs`, update the test imports inside `#[cfg(test)] mod tests`:

```rust
use crate::commands::{ActionReviewItemDto, AppSettingsDto, NoteListItemDto};
use crate::domain::{ActionItemId, ActionReviewItem, NoteId, ParseStatus, ReviewStatus};
```

Add this test:

```rust
    #[test]
    fn action_review_item_dto_serializes_camel_case_fields() {
        let item = ActionReviewItem {
            id: ActionItemId::new(),
            note_id: NoteId::new(),
            note_title: "Kiosk 7 telemetry IDs".to_string(),
            text: "Bring serial list into the Tuesday sync.".to_string(),
            owner: Some("Maya".to_string()),
            due_date: None,
            confidence: Some(0.82),
            created_at: Utc::now(),
        };

        let serialized = serde_json::to_value(ActionReviewItemDto::from(item)).unwrap();

        assert!(serialized.get("id").is_some());
        assert!(serialized.get("noteId").is_some());
        assert!(serialized.get("noteTitle").is_some());
        assert!(serialized.get("dueDate").is_some());
        assert!(serialized.get("createdAt").is_some());
    }
```

- [ ] **Step 2: Run the failing command DTO test**

Run:

```powershell
scripts\cargo-test.cmd action_review_item_dto_serializes_camel_case_fields
```

Expected: FAIL because `ActionReviewItemDto` does not exist.

- [ ] **Step 3: Add DTO and commands**

In `src-tauri/src/commands.rs`, add `ActionReviewItem` to the domain import list and import the service:

```rust
use crate::domain::{
    ActionItem, ActionItemId, ActionReviewItem, ActionStatus, InboxFilters, Note, NoteId,
    NoteListItem, ParseStatus, ReviewStatus, Tag, TagAssignment, TagId, TagKind,
};
use crate::services::actions::ActionItemService;
```

Add this DTO after `ActionItemDto`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionReviewItemDto {
    pub id: String,
    pub note_id: String,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
}

impl From<ActionReviewItem> for ActionReviewItemDto {
    fn from(item: ActionReviewItem) -> Self {
        Self {
            id: item.id.to_string(),
            note_id: item.note_id.to_string(),
            note_title: item.note_title,
            text: item.text,
            owner: item.owner,
            due_date: item.due_date,
            confidence: item.confidence,
            created_at: item.created_at,
        }
    }
}
```

Replace the bodies of `accept_action_item` and `dismiss_action_item` with service calls:

```rust
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).accept(action_id)?;
    Ok(())
```

```rust
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).dismiss(action_id)?;
    Ok(())
```

Add these commands after `dismiss_action_item`:

```rust
#[tauri::command]
pub async fn complete_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).complete(action_id)?;
    Ok(())
}

#[tauri::command]
pub async fn reopen_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).reopen(action_id)?;
    Ok(())
}

#[tauri::command]
pub async fn list_suggested_actions(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<ActionReviewItemDto>, CommandError> {
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let items = ActionItemService::new(state.repositories.clone()).list_suggested(limit)?;
    Ok(items.into_iter().map(Into::into).collect())
}
```

In `src-tauri/src/lib.rs`, register the new commands after `dismiss_action_item`:

```rust
            commands::complete_action_item,
            commands::reopen_action_item,
            commands::list_suggested_actions,
```

- [ ] **Step 4: Run command tests**

Run:

```powershell
scripts\cargo-test.cmd commands
```

Expected: PASS.

- [ ] **Step 5: Commit command contract**

Run:

```powershell
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: expose action lifecycle commands"
```

---

### Task 4: Frontend Types, API Wrappers, And Browser Fallback

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`
- Test: `src/lib/stores/inbox.test.ts`

- [ ] **Step 1: Add frontend API coverage through store fake shape**

In `src/lib/stores/inbox.test.ts`, update the import type list:

```ts
import type { ActionReviewItem, AppSettings, InboxFilters, NoteDetail, NoteListItem } from "$lib/types";
```

Add this helper near `note()`:

```ts
function reviewItem(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Pricing export mismatch",
    text: "Check the filtered query.",
    owner: "Jordan",
    dueDate: null,
    confidence: 0.82,
    createdAt: "2026-05-20T13:00:00.000Z",
    ...overrides,
  };
}
```

Update `testApi()` to include these fake methods:

```ts
    completeActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    reopenActionItem: vi.fn<(actionItemId: string) => Promise<void>>().mockResolvedValue(undefined),
    listSuggestedActions: vi.fn<() => Promise<ActionReviewItem[]>>().mockResolvedValue([reviewItem()]),
```

- [ ] **Step 2: Run the failing TypeScript test check**

Run:

```powershell
npm test -- src/lib/stores/inbox.test.ts
```

Expected: FAIL because `ActionReviewItem` and the API shape methods do not exist yet.

- [ ] **Step 3: Add frontend type and API methods**

In `src/lib/types.ts`, add this type after `ActionItem`:

```ts
export type ActionReviewItem = {
  id: string;
  noteId: string;
  noteTitle: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  confidence?: number | null;
  createdAt: string;
};
```

In `src/lib/api.ts`, add `ActionReviewItem` to the type import list:

```ts
  ActionReviewItem,
```

Add wrappers after `dismissActionItem`:

```ts
export async function completeActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("complete_action_item", { actionId: actionItemId });
}

export async function reopenActionItem(actionItemId: string): Promise<void> {
  await invokeCommand<void>("reopen_action_item", { actionId: actionItemId });
}

export async function listSuggestedActions(): Promise<ActionReviewItem[]> {
  const actions = await invokeCommand<unknown[]>("list_suggested_actions", { limit: 100 });
  return actions.map(normalizeActionReviewItem);
}
```

Add these methods to the exported `api` object:

```ts
  completeActionItem,
  reopenActionItem,
  listSuggestedActions,
```

Add this normalizer after `normalizeActionItem`:

```ts
function normalizeActionReviewItem(value: unknown): ActionReviewItem {
  const record = asRecord(value);

  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    noteId: getString(record, "noteId", "note_id") ?? "",
    noteTitle: getString(record, "noteTitle", "note_title") ?? "Untitled note",
    text: getString(record, "text") ?? "",
    owner: getNullableString(record, "owner"),
    dueDate: getNullableString(record, "dueDate", "due_date"),
    confidence: getNumber(record, "confidence"),
    createdAt: getString(record, "createdAt", "created_at") ?? fallbackNow,
  };
}
```

In `fallbackCommand`, add these cases:

```ts
    case "complete_action_item":
    case "reopen_action_item": {
      const actionItemId = String(args?.actionId ?? "");
      const status = command === "complete_action_item" ? "done" : "accepted";
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === actionItemId);
      if (action) {
        action.status = status;
      }
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
        ) as T;
```

- [ ] **Step 4: Run API-related frontend tests**

Run:

```powershell
npm test -- src/lib/stores/inbox.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit frontend API contract**

Run:

```powershell
git add src/lib/types.ts src/lib/api.ts src/lib/stores/inbox.test.ts
git commit -m "feat: add frontend action workflow api"
```

---

### Task 5: Store Queue State And Action Refresh Flows

**Files:**
- Modify: `src/lib/stores/inbox.ts`
- Modify: `src/lib/stores/inbox.test.ts`

- [ ] **Step 1: Add failing store behavior tests**

In `src/lib/stores/inbox.test.ts`, add these tests inside the existing `describe("createWorkNotesStore", () => { })` block:

```ts
  it("loads suggested actions into review queue state", async () => {
    const api = testApi({
      listSuggestedActions: vi.fn().mockResolvedValue([reviewItem({ id: "action-queued" })]),
    });
    const store = createWorkNotesStore(api);

    await store.loadSuggestedActions();

    expect(api.listSuggestedActions).toHaveBeenCalledTimes(1);
    expect(get(store.suggestedActions).map((item) => item.id)).toEqual(["action-queued"]);
    expect(get(store.loadingSuggestedActions)).toBe(false);
  });

  it("refreshes selected note, inbox, and suggested queue after accepting an action", async () => {
    const api = testApi({
      listInbox: vi.fn().mockResolvedValue([note({ suggestedActionItemCount: 0 })]),
      listSuggestedActions: vi.fn().mockResolvedValue([]),
    });
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.acceptSuggestedAction("action-1");

    expect(api.acceptActionItem).toHaveBeenCalledWith("action-1");
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
    expect(api.listInbox).toHaveBeenCalled();
    expect(api.listSuggestedActions).toHaveBeenCalled();
  });

  it("completes and reopens actions through the workflow store", async () => {
    const api = testApi();
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.completeAction("action-1");
    await store.reopenAction("action-1");

    expect(api.completeActionItem).toHaveBeenCalledWith("action-1");
    expect(api.reopenActionItem).toHaveBeenCalledWith("action-1");
    expect(api.getNote).toHaveBeenLastCalledWith("note-1");
  });
```

- [ ] **Step 2: Run the failing store tests**

Run:

```powershell
npm test -- src/lib/stores/inbox.test.ts
```

Expected: FAIL because `suggestedActions`, `loadingSuggestedActions`, `loadSuggestedActions`, `completeAction`, and `reopenAction` are not returned by the store.

- [ ] **Step 3: Extend store API dependencies and state**

In `src/lib/stores/inbox.ts`, update imports from `$lib/api`:

```ts
  completeActionItem,
  listSuggestedActions,
  reopenActionItem,
```

Update the `$lib/types` import:

```ts
import type { ActionReviewItem, AppSettings, InboxFilters, NoteDetail, NoteListItem } from "$lib/types";
```

Add these fields to `WorkNotesApi`:

```ts
  completeActionItem: typeof completeActionItem;
  reopenActionItem: typeof reopenActionItem;
  listSuggestedActions: typeof listSuggestedActions;
```

Add these fields to `defaultApi`:

```ts
  completeActionItem,
  reopenActionItem,
  listSuggestedActions,
```

Add store state after `selectedNote`:

```ts
  const suggestedActions = writable<ActionReviewItem[]>([]);
```

Add loading state after `loadingNote`:

```ts
  const loadingSuggestedActions = writable(false);
```

- [ ] **Step 4: Add queue and action lifecycle methods**

In `src/lib/stores/inbox.ts`, add this method after `selectNote`:

```ts
  async function loadSuggestedActions(): Promise<void> {
    loadingSuggestedActions.set(true);
    error.set(null);

    try {
      suggestedActions.set(await api.listSuggestedActions());
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load suggested actions."));
    } finally {
      loadingSuggestedActions.set(false);
    }
  }
```

Add these methods after `dismissSuggestedAction`:

```ts
  async function completeAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.completeActionItem, false);
  }

  async function reopenAction(actionItemId: string): Promise<void> {
    await updateAction(actionItemId, api.reopenActionItem, false);
  }
```

Replace `updateAction` with this version:

```ts
  async function updateAction(
    actionItemId: string,
    update:
      | WorkNotesApi["acceptActionItem"]
      | WorkNotesApi["dismissActionItem"]
      | WorkNotesApi["completeActionItem"]
      | WorkNotesApi["reopenActionItem"],
    refreshSuggestedActions = true,
  ): Promise<void> {
    busyActionId.set(actionItemId);
    error.set(null);

    try {
      await update(actionItemId);
      const note = get(selectedNote);
      if (note) {
        selectedNote.set(await api.getNote(note.id));
      }
      await loadInbox();
      if (refreshSuggestedActions) {
        await loadSuggestedActions();
      }
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not update action."));
    } finally {
      busyActionId.set(null);
    }
  }
```

Add these values to the returned object:

```ts
    suggestedActions,
    loadingSuggestedActions,
    loadSuggestedActions,
    completeAction,
    reopenAction,
```

- [ ] **Step 5: Run store tests**

Run:

```powershell
npm test -- src/lib/stores/inbox.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit store workflow**

Run:

```powershell
git add src/lib/stores/inbox.ts src/lib/stores/inbox.test.ts
git commit -m "feat: manage action queue state"
```

---

### Task 6: Note Detail Complete And Reopen Controls

**Files:**
- Modify: `src/lib/components/NoteDetail.svelte`
- Modify: `src/lib/components/NoteDetail.test.ts`

- [ ] **Step 1: Add failing component test**

In `src/lib/components/NoteDetail.test.ts`, add this test:

```ts
  it("dispatches complete and reopen events for accepted and done actions", async () => {
    const completeAction = vi.fn();
    const reopenAction = vi.fn();

    render(NoteDetail, {
      props: { note: noteDetailWithAcceptedAndDoneActions() },
      events: { completeAction, reopenAction },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Complete action: Send the update" }));
    await fireEvent.click(screen.getByRole("button", { name: "Reopen action: Verify the fix" }));

    expect(completeAction.mock.calls[0][0].detail).toBe("action-accepted");
    expect(reopenAction.mock.calls[0][0].detail).toBe("action-done");
  });
```

Add this helper:

```ts
function noteDetailWithAcceptedAndDoneActions(): NoteDetailType {
  return {
    ...noteDetail(),
    parseStatus: "parsed",
    parseError: null,
    actionItemCount: 2,
    suggestedActionItemCount: 0,
    actionItems: [
      {
        id: "action-accepted",
        noteId: "note-1",
        text: "Send the update",
        owner: "me",
        dueDate: null,
        status: "accepted",
        source: "parser",
        noteTitle: "Robert local AI",
      },
      {
        id: "action-done",
        noteId: "note-1",
        text: "Verify the fix",
        owner: "me",
        dueDate: null,
        status: "done",
        source: "parser",
        noteTitle: "Robert local AI",
      },
    ],
  };
}
```

- [ ] **Step 2: Run the failing component test**

Run:

```powershell
npm test -- src/lib/components/NoteDetail.test.ts
```

Expected: FAIL because the component does not dispatch `completeAction` or `reopenAction`.

- [ ] **Step 3: Add events and action groups**

In `src/lib/components/NoteDetail.svelte`, replace the action derived values with:

```ts
  const suggestedActions = $derived(note?.actionItems.filter((action) => action.status === "suggested") ?? []);
  const acceptedActions = $derived(note?.actionItems.filter((action) => action.status === "accepted") ?? []);
  const doneActions = $derived(note?.actionItems.filter((action) => action.status === "done") ?? []);
```

Update the dispatcher type:

```ts
    completeAction: string;
    reopenAction: string;
```

Replace the completed action section in the actions block with:

```svelte
          {#if acceptedActions.length > 0}
            <div class="section-head muted-head">
              <span>Accepted</span>
            </div>
            {#each acceptedActions as action}
              <article class="action-row">
                <button
                  class="action-check"
                  type="button"
                  aria-label={`Complete action: ${action.text}`}
                  disabled={loading || busyActionId === action.id}
                  onclick={() => dispatch("completeAction", action.id)}
                >
                  OK
                </button>
                <div>
                  <p>{action.text}</p>
                  {#if actionMeta(action)}
                    <small>{actionMeta(action)}</small>
                  {/if}
                </div>
              </article>
            {/each}
          {/if}

          {#if doneActions.length > 0}
            <div class="section-head muted-head">
              <span>Done</span>
            </div>
            {#each doneActions as action}
              <article class="action-row done">
                <span class="action-check checked">OK</span>
                <div>
                  <p>{action.text}</p>
                </div>
                <button
                  class="action-dismiss"
                  type="button"
                  aria-label={`Reopen action: ${action.text}`}
                  disabled={loading || busyActionId === action.id}
                  onclick={() => dispatch("reopenAction", action.id)}
                >
                  x
                </button>
              </article>
            {/each}
          {/if}
```

Update the empty-state condition:

```svelte
          {#if suggestedActions.length === 0 && acceptedActions.length === 0 && doneActions.length === 0}
            <p class="muted">No actions</p>
          {/if}
```

- [ ] **Step 4: Run NoteDetail tests**

Run:

```powershell
npm test -- src/lib/components/NoteDetail.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit note detail controls**

Run:

```powershell
git add src/lib/components/NoteDetail.svelte src/lib/components/NoteDetail.test.ts
git commit -m "feat: complete and reopen note actions"
```

---

### Task 7: Review Queue Component And Mounting

**Files:**
- Modify: `src/lib/components/ReviewQueue.svelte`
- Create: `src/lib/components/ReviewQueue.test.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Add ReviewQueue component test**

Create `src/lib/components/ReviewQueue.test.ts`:

```ts
// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem } from "$lib/types";
import ReviewQueue from "./ReviewQueue.svelte";

afterEach(() => cleanup());

describe("ReviewQueue", () => {
  it("renders suggested actions with note context and dispatches queue events", async () => {
    const select = vi.fn();
    const accept = vi.fn();
    const dismiss = vi.fn();

    render(ReviewQueue, {
      props: {
        actions: actions(),
        busyActionId: null,
        loading: false,
      },
      events: { select, accept, dismiss },
    });

    expect(screen.getByText("Kiosk 7 telemetry IDs")).toBeTruthy();
    expect(screen.getByText("Bring serial list into the Tuesday sync.")).toBeTruthy();
    expect(screen.getByText("Maya")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Kiosk 7 telemetry IDs" }));
    await fireEvent.click(screen.getByRole("button", { name: "Accept action: Bring serial list into the Tuesday sync." }));
    await fireEvent.click(screen.getByRole("button", { name: "Dismiss action: Bring serial list into the Tuesday sync." }));

    expect(select.mock.calls[0][0].detail).toBe("note-1");
    expect(accept.mock.calls[0][0].detail).toBe("action-1");
    expect(dismiss.mock.calls[0][0].detail).toBe("action-1");
  });
});

function actions(): ActionReviewItem[] {
  return [
    {
      id: "action-1",
      noteId: "note-1",
      noteTitle: "Kiosk 7 telemetry IDs",
      text: "Bring serial list into the Tuesday sync.",
      owner: "Maya",
      dueDate: null,
      confidence: 0.82,
      createdAt: "2026-05-20T13:42:00.000Z",
    },
  ];
}
```

- [ ] **Step 2: Run the failing ReviewQueue test**

Run:

```powershell
npm test -- src/lib/components/ReviewQueue.test.ts
```

Expected: FAIL because `ReviewQueue.svelte` still consumes `ActionItem` and has no `select` event.

- [ ] **Step 3: Update ReviewQueue props and events**

In `src/lib/components/ReviewQueue.svelte`, change the type import:

```ts
  import type { ActionReviewItem } from "$lib/types";
```

Change props:

```ts
  type Props = {
    actions: ActionReviewItem[];
    busyActionId?: string | null;
    loading?: boolean;
  };

  let { actions, busyActionId = null, loading = false }: Props = $props();
```

Change dispatcher:

```ts
  const dispatch = createEventDispatcher<{
    select: string;
    accept: string;
    dismiss: string;
  }>();
```

Replace `actionMeta`:

```ts
  function actionMeta(action: ActionReviewItem): string {
    return [action.owner, action.dueDate, confidenceLabel(action.confidence)].filter(Boolean).join(" · ") || "Needs confirmation";
  }

  function confidenceLabel(confidence: number | null | undefined): string | null {
    return typeof confidence === "number" ? `${Math.round(confidence * 100)}%` : null;
  }
```

In the markup for each `action-card`, add an open-note button and accessible action labels:

```svelte
        <div class="action-copy">
          <button
            class="note-link"
            type="button"
            aria-label={`Open note: ${action.noteTitle}`}
            onclick={() => dispatch("select", action.noteId)}
          >
            {action.noteTitle}
          </button>
          <p>{action.text}</p>
          <small>{actionMeta(action)}</small>
        </div>
        <div class="action-buttons">
          <button
            type="button"
            aria-label={`Accept action: ${action.text}`}
            disabled={loading || busyActionId === action.id}
            onclick={() => dispatch("accept", action.id)}
          >
            Accept
          </button>
          <button
            type="button"
            aria-label={`Dismiss action: ${action.text}`}
            disabled={loading || busyActionId === action.id}
            onclick={() => dispatch("dismiss", action.id)}
          >
            Dismiss
          </button>
        </div>
```

Add this CSS next to `.action-copy span` styles:

```css
  .note-link {
    width: fit-content;
    border: 0;
    padding: 0;
    color: var(--color-accent-primary);
    background: transparent;
    font: inherit;
    font-size: 11px;
    font-weight: 800;
    text-align: left;
    text-transform: uppercase;
    cursor: pointer;
  }

  .note-link:hover,
  .note-link:focus-visible {
    text-decoration: underline;
    outline: none;
  }
```

- [ ] **Step 4: Mount ReviewQueue in the main route**

In `src/routes/+page.svelte`, import the component:

```ts
  import ReviewQueue from "$lib/components/ReviewQueue.svelte";
```

Destructure store state:

```ts
    suggestedActions,
    loadingSuggestedActions,
```

In `onMount`, load suggested actions alongside the inbox for browser fallback and the main Tauri window:

```ts
      void workNotes.loadSuggestedActions();
```

For the Tauri main-window branch, add it after `void workNotes.loadInbox();`:

```ts
      void workNotes.loadSuggestedActions();
```

After `NOTE_CAPTURED_EVENT` refresh handling, call the queue refresh:

```ts
          void workNotes.loadSuggestedActions();
```

Add handlers to `NoteDetail`:

```svelte
        on:completeAction={(event) => void workNotes.completeAction(event.detail)}
        on:reopenAction={(event) => void workNotes.reopenAction(event.detail)}
```

Wrap `NoteDetail` in a detail stack and mount the queue above it:

```svelte
      <div class="detail-stack">
        {#if $suggestedActions.length > 0 || $loadingSuggestedActions}
          <ReviewQueue
            actions={$suggestedActions}
            busyActionId={$busyActionId}
            loading={$loadingSuggestedActions}
            on:select={(event) => void workNotes.selectNote(event.detail)}
            on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
            on:dismiss={(event) => void workNotes.dismissSuggestedAction(event.detail)}
          />
        {/if}

        <NoteDetail
          note={$selectedNote}
          loading={$loadingNote}
          busyActionId={$busyActionId}
          on:retryParse={() => void workNotes.retrySelectedParse()}
          on:reparseWithFeedback={(event) => void workNotes.retrySelectedParseWithFeedback(event.detail)}
          on:deleteNote={() => void deleteSelectedNote()}
          on:acceptAction={(event) => void workNotes.acceptSuggestedAction(event.detail)}
          on:dismissAction={(event) => void workNotes.dismissSuggestedAction(event.detail)}
          on:completeAction={(event) => void workNotes.completeAction(event.detail)}
          on:reopenAction={(event) => void workNotes.reopenAction(event.detail)}
        />
      </div>
```

Add CSS:

```css
  .detail-stack {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 100vh;
    overflow: hidden;
  }

  :global(.detail-stack .review-queue) {
    max-height: 240px;
    border-top: 0;
    border-right: 0;
    border-left: 0;
    border-radius: 0;
  }
```

- [ ] **Step 5: Run component and route-related frontend checks**

Run:

```powershell
npm test -- src/lib/components/ReviewQueue.test.ts src/lib/components/NoteDetail.test.ts src/lib/stores/inbox.test.ts
npm run check
```

Expected: PASS.

- [ ] **Step 6: Commit review queue UI wiring**

Run:

```powershell
git add src/lib/components/ReviewQueue.svelte src/lib/components/ReviewQueue.test.ts src/lib/components/NoteDetail.svelte src/lib/components/NoteDetail.test.ts src/routes/+page.svelte
git commit -m "feat: wire suggested action review queue"
```

---

### Task 8: Final Verification And Handoff

**Files:**
- Modify only files required by failures found during verification.

- [ ] **Step 1: Run frontend test suite**

Run:

```powershell
npm test
```

Expected: PASS.

- [ ] **Step 2: Run Svelte and TypeScript checks**

Run:

```powershell
npm run check
```

Expected: PASS.

- [ ] **Step 3: Run frontend production build**

Run:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 4: Run Rust tests**

Run:

```powershell
scripts\cargo-test.cmd
```

Expected: PASS.

- [ ] **Step 5: Inspect staged and unstaged state**

Run:

```powershell
git status --short --branch
```

Expected: feature files from this plan are committed in focused commits. The pre-existing docs-pass edits may still appear as unstaged modifications unless the user asked to include them.

- [ ] **Step 6: Manual app smoke check**

Start the app:

```powershell
npm run tauri dev
```

Manual checks:

- Main window opens without localhost refusal.
- Existing notes load.
- Review queue appears when suggested actions exist.
- Selecting a review queue note opens that note in detail.
- Accepting the final suggested action marks the note reviewed.
- Completing an accepted action moves it to Done.
- Reopening a done action moves it back to Accepted.
- Raw note text does not change during action status transitions.

- [ ] **Step 7: Final status summary**

Run:

```powershell
git log --oneline -5
git status --short --branch
```

Expected: recent commits include the action workflow implementation commits. Report any remaining unstaged docs-pass edits separately from action workflow work.
