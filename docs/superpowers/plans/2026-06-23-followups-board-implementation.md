# Follow-ups Board Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a top-level Follow-ups board that shows accepted and note-created follow-ups grouped by project/topic lanes with Open, Waiting, Blocked, and Done states.

**Architecture:** Extend the existing `action_items` model with follow-up metadata instead of creating a separate task table. Backend services own validation and SQLite writes; `src/lib/api.ts` owns Tauri calls and browser fallback; `src/lib/stores/inbox.ts` owns workflow sequencing; Svelte components stay thin and event-driven.

**Tech Stack:** Tauri v2, Rust, rusqlite, SQLite, Svelte 5, TypeScript, Vitest, Testing Library Svelte.

**Repo rule:** Do not commit during execution unless the user explicitly asks. This plan intentionally omits commit steps.

---

## File Map

- Modify `src-tauri/src/domain.rs`: add `FollowupState`, extend `ActionItem`, and add `FollowupItem`.
- Modify `src-tauri/src/db/migrations.rs`: add `followup_state` and `followup_lane` columns plus indexes.
- Modify `src-tauri/src/db/mod.rs`: add migration coverage for new columns.
- Modify `src-tauri/src/repositories/actions.rs`: read/write follow-up metadata, create manual follow-ups, list board rows.
- Modify `src-tauri/src/services/actions.rs`: validate follow-up creation, state changes, lane changes, and existing lifecycle integration.
- Modify `src-tauri/src/commands.rs`: add follow-up DTOs and Tauri commands.
- Modify `src-tauri/src/lib.rs`: register new commands.
- Modify `src/lib/types.ts`: add follow-up frontend types and action metadata.
- Modify `src/lib/api.ts`: add wrappers, normalizers, and fallback behavior.
- Modify `src/lib/stores/inbox.ts`: add Follow-ups view state and workflow methods.
- Create `src/lib/followups.ts`: pure lane grouping and label helpers.
- Create `src/lib/followups.test.ts`: pure helper tests.
- Create `src/lib/components/FollowupsView.svelte`: board view component.
- Create `src/lib/components/FollowupsView.test.ts`: component behavior tests.
- Modify `src/lib/components/AppShell.svelte`: add Follow-ups nav item and metric.
- Modify `src/lib/components/AppShell.test.ts`: navigation test coverage.
- Modify `src/lib/components/NoteDetail.svelte`: add manual follow-up entry UI.
- Modify `src/lib/components/NoteDetail.test.ts`: manual follow-up event coverage.
- Modify `src/routes/+page.svelte`: render Follow-ups and wire events.

---

### Task 1: Backend Domain And Migration

**Files:**
- Modify: `src-tauri/src/domain.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/mod.rs`

- [ ] **Step 1: Add failing migration and enum tests**

In `src-tauri/src/db/mod.rs`, extend `db_migration_creates_required_tables` with column assertions:

```rust
        for column in ["followup_state", "followup_lane"] {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM pragma_table_info('action_items')
                     WHERE name = ?1",
                    [column],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "action_items.{column} should exist");
        }
```

In `src-tauri/src/domain.rs`, add this test module content to the existing file if no domain tests exist:

```rust
#[cfg(test)]
mod tests {
    use super::FollowupState;

    #[test]
    fn followup_state_round_trips_db_values() {
        assert_eq!(FollowupState::from_db("open").unwrap(), FollowupState::Open);
        assert_eq!(FollowupState::from_db("waiting").unwrap(), FollowupState::Waiting);
        assert_eq!(FollowupState::from_db("blocked").unwrap(), FollowupState::Blocked);
        assert!(FollowupState::from_db("done").is_err());
    }
}
```

- [ ] **Step 2: Run targeted Rust tests and verify they fail**

Run:

```powershell
scripts\cargo-test.cmd db_migration_creates_required_tables followup_state_round_trips_db_values
```

Expected: FAIL because `FollowupState` does not exist and the new columns are absent.

- [ ] **Step 3: Add domain types and action metadata**

In `src-tauri/src/domain.rs`, add this enum after `ActionStatus`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowupState {
    Open,
    Waiting,
    Blocked,
}

impl FollowupState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Waiting => "waiting",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "open" => Ok(Self::Open),
            "waiting" => Ok(Self::Waiting),
            "blocked" => Ok(Self::Blocked),
            _ => Err(DomainError::InvalidEnum {
                field: "followup_state",
                value: value.to_string(),
            }),
        }
    }
}
```

Update `ActionItem`:

```rust
pub struct ActionItem {
    pub id: ActionItemId,
    pub note_id: NoteId,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
}
```

Add `FollowupItem` after `ActionReviewItem`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FollowupItem {
    pub id: ActionItemId,
    pub note_id: NoteId,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
    pub tags: Vec<TagAssignment>,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Add migration columns and indexes**

In `src-tauri/src/db/migrations.rs`, add columns after the existing `ensure_column` calls:

```rust
    ensure_column(
        connection,
        "action_items",
        "followup_state",
        "followup_state TEXT",
    )?;
    ensure_column(
        connection,
        "action_items",
        "followup_lane",
        "followup_lane TEXT",
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_action_items_status_followup_state
         ON action_items(status, followup_state)",
        [],
    )?;
```

- [ ] **Step 5: Run targeted Rust tests and verify they pass**

Run:

```powershell
scripts\cargo-test.cmd db_migration_creates_required_tables followup_state_round_trips_db_values
```

Expected: PASS.

---

### Task 2: Repository Follow-up Persistence

**Files:**
- Modify: `src-tauri/src/repositories/actions.rs`
- Modify: `src-tauri/src/repositories/mod.rs`

- [ ] **Step 1: Add failing repository tests**

In `src-tauri/src/repositories/mod.rs`, add this test near the existing action repository tests:

```rust
    #[test]
    fn action_repository_lists_followups_with_lanes_and_tags() {
        let db = test_db();
        let notes = NoteRepository::new(db.clone());
        let tags = TagRepository::new(db.clone());
        let actions = ActionItemRepository::new(db);

        let project_note = notes.create_raw_note("Front desk printer").unwrap();
        let project = tags.upsert("Front desk", TagKind::Project).unwrap();
        tags.apply_to_note(project_note.id, project.id, "parser", Some(0.9))
            .unwrap();
        let accepted = actions
            .create(
                project_note.id,
                "Check printer alignment",
                Some("Rina"),
                None,
                ActionStatus::Accepted,
                "parser",
                Some(0.8),
            )
            .unwrap();
        actions
            .set_followup_state(accepted.id, crate::domain::FollowupState::Waiting)
            .unwrap();

        let manual = actions
            .create_manual_followup(project_note.id, "Ask Rina for a sample badge", Some("Ops"))
            .unwrap();

        let suggested = actions
            .create_suggested(project_note.id, "Not accepted yet", None, None, Some(0.5))
            .unwrap();

        let items = actions.list_followups(100).unwrap();

        assert_eq!(items.iter().map(|item| item.id).collect::<Vec<_>>(), vec![manual.id, accepted.id]);
        assert_eq!(items[0].followup_lane.as_deref(), Some("Ops"));
        assert_eq!(items[0].followup_state, Some(crate::domain::FollowupState::Open));
        assert_eq!(items[0].source, "user");
        assert_eq!(items[1].tags[0].tag.name, "Front desk");
        assert_eq!(items[1].followup_state, Some(crate::domain::FollowupState::Waiting));
        assert!(!items.iter().any(|item| item.id == suggested.id));
    }
```

- [ ] **Step 2: Run repository test and verify it fails**

Run:

```powershell
scripts\cargo-test.cmd action_repository_lists_followups_with_lanes_and_tags
```

Expected: FAIL with missing methods or fields.

- [ ] **Step 3: Update imports and create signature**

In `src-tauri/src/repositories/actions.rs`, update imports:

```rust
use crate::domain::{
    ActionItem, ActionItemId, ActionReviewItem, ActionStatus, FollowupItem, FollowupState, NoteId,
    Tag, TagAssignment, TagId, TagKind,
};

use super::{parse_db_datetime, RepositoryError, RepositoryResult};
```

Change `create` to insert follow-up columns:

```rust
    pub fn create(
        &self,
        note_id: NoteId,
        text: &str,
        owner: Option<&str>,
        due_date: Option<&str>,
        status: ActionStatus,
        source: &str,
        confidence: Option<f64>,
    ) -> RepositoryResult<ActionItem> {
        self.create_with_followup(note_id, text, owner, due_date, status, source, confidence, None, None)
    }
```

Add helper:

```rust
    fn create_with_followup(
        &self,
        note_id: NoteId,
        text: &str,
        owner: Option<&str>,
        due_date: Option<&str>,
        status: ActionStatus,
        source: &str,
        confidence: Option<f64>,
        followup_state: Option<FollowupState>,
        followup_lane: Option<&str>,
    ) -> RepositoryResult<ActionItem> {
        let id = ActionItemId::new();
        let id_text = id.to_string();
        let followup_state_text = followup_state.as_ref().map(FollowupState::as_str);
        let connection = self.db.connection()?;
        connection.execute(
            "INSERT INTO action_items (
                id, note_id, text, owner, due_date, status, source, confidence, followup_state, followup_lane
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id_text,
                note_id.to_string(),
                text,
                owner,
                due_date,
                status.as_str(),
                source,
                confidence,
                followup_state_text,
                followup_lane
            ],
        )?;

        Ok(ActionItem {
            id,
            note_id,
            text: text.to_string(),
            owner: owner.map(str::to_string),
            due_date: due_date.map(str::to_string),
            status,
            source: source.to_string(),
            confidence,
            followup_state,
            followup_lane: followup_lane.map(str::to_string),
        })
    }
```

- [ ] **Step 4: Add repository methods**

Add methods to `impl ActionItemRepository`:

```rust
    pub fn create_manual_followup(
        &self,
        note_id: NoteId,
        text: &str,
        lane_override: Option<&str>,
    ) -> RepositoryResult<ActionItem> {
        self.create_with_followup(
            note_id,
            text,
            None,
            None,
            ActionStatus::Accepted,
            "user",
            None,
            Some(FollowupState::Open),
            lane_override,
        )
    }

    pub fn set_followup_state(
        &self,
        id: ActionItemId,
        state: FollowupState,
    ) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        let changed = connection.execute(
            "UPDATE action_items SET followup_state = ?2 WHERE id = ?1",
            params![id.to_string(), state.as_str()],
        )?;
        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub fn set_followup_lane(
        &self,
        id: ActionItemId,
        lane_override: Option<&str>,
    ) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        let changed = connection.execute(
            "UPDATE action_items SET followup_lane = ?2 WHERE id = ?1",
            params![id.to_string(), lane_override],
        )?;
        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub fn list_followups(&self, limit: u32) -> RepositoryResult<Vec<FollowupItem>> {
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
                ai.status,
                ai.source,
                ai.confidence,
                ai.followup_state,
                ai.followup_lane,
                n.created_at
             FROM action_items ai
             JOIN notes n ON n.id = ai.note_id
             WHERE ai.status IN ('accepted', 'done') AND n.is_archived = 0
             ORDER BY n.created_at DESC, ai.rowid DESC
             LIMIT ?1",
        )?;

        let records = statement
            .query_map([limit], FollowupItemRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        records
            .into_iter()
            .map(|record| {
                let tags = self.list_tags_for_note(record.note_id.clone())?;
                record.into_followup_item(tags)
            })
            .collect()
    }
```

Add helper:

```rust
    fn list_tags_for_note(&self, note_id: String) -> RepositoryResult<Vec<TagAssignment>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT t.id, t.name, t.kind, t.created_at, nt.source, nt.confidence
             FROM note_tags nt
             JOIN tags t ON t.id = nt.tag_id
             WHERE nt.note_id = ?1
             ORDER BY
               CASE t.kind WHEN 'project' THEN 0 WHEN 'topic' THEN 1 ELSE 2 END,
               lower(t.name)",
        )?;
        let records = statement
            .query_map([note_id], FollowupTagRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        records.into_iter().map(FollowupTagRecord::into_assignment).collect()
    }
```

- [ ] **Step 5: Update row records**

Change all `SELECT ... FROM action_items` projections in `get` and `list_for_note` to include `followup_state, followup_lane`.

Update `ActionItemRecord`:

```rust
struct ActionItemRecord {
    id: String,
    note_id: String,
    text: String,
    owner: Option<String>,
    due_date: Option<String>,
    status: String,
    source: String,
    confidence: Option<f64>,
    followup_state: Option<String>,
    followup_lane: Option<String>,
}
```

Update `from_row` and `into_action_item`:

```rust
            confidence: row.get(7)?,
            followup_state: row.get(8)?,
            followup_lane: row.get(9)?,
```

```rust
            followup_state: self
                .followup_state
                .as_deref()
                .map(FollowupState::from_db)
                .transpose()?,
            followup_lane: self.followup_lane,
```

Add records:

```rust
struct FollowupItemRecord {
    id: String,
    note_id: String,
    note_title: String,
    text: String,
    owner: Option<String>,
    due_date: Option<String>,
    status: String,
    source: String,
    confidence: Option<f64>,
    followup_state: Option<String>,
    followup_lane: Option<String>,
    created_at: String,
}

impl FollowupItemRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            note_id: row.get(1)?,
            note_title: row.get(2)?,
            text: row.get(3)?,
            owner: row.get(4)?,
            due_date: row.get(5)?,
            status: row.get(6)?,
            source: row.get(7)?,
            confidence: row.get(8)?,
            followup_state: row.get(9)?,
            followup_lane: row.get(10)?,
            created_at: row.get(11)?,
        })
    }

    fn into_followup_item(self, tags: Vec<TagAssignment>) -> RepositoryResult<FollowupItem> {
        Ok(FollowupItem {
            id: ActionItemId::parse(&self.id)?,
            note_id: NoteId::parse(&self.note_id)?,
            note_title: self.note_title,
            text: self.text,
            owner: self.owner,
            due_date: self.due_date,
            status: ActionStatus::from_db(&self.status)?,
            source: self.source,
            confidence: self.confidence,
            followup_state: self
                .followup_state
                .as_deref()
                .map(FollowupState::from_db)
                .transpose()?,
            followup_lane: self.followup_lane,
            tags,
            created_at: parse_db_datetime("created_at", self.created_at)?,
        })
    }
}

struct FollowupTagRecord {
    id: String,
    name: String,
    kind: String,
    created_at: String,
    source: String,
    confidence: Option<f64>,
}

impl FollowupTagRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            created_at: row.get(3)?,
            source: row.get(4)?,
            confidence: row.get(5)?,
        })
    }

    fn into_assignment(self) -> RepositoryResult<TagAssignment> {
        Ok(TagAssignment {
            tag: Tag {
                id: TagId::parse(&self.id)?,
                name: self.name,
                kind: TagKind::from_db(&self.kind)?,
                created_at: parse_db_datetime("created_at", self.created_at)?,
            },
            source: self.source,
            confidence: self.confidence,
        })
    }
}
```

- [ ] **Step 6: Update `set_status` to initialize accepted follow-ups**

In `set_status`, use this SQL:

```rust
            "UPDATE action_items
             SET status = ?2,
                 followup_state = CASE
                   WHEN ?2 = 'accepted' AND followup_state IS NULL THEN 'open'
                   ELSE followup_state
                 END
             WHERE id = ?1",
```

- [ ] **Step 7: Run repository tests**

Run:

```powershell
scripts\cargo-test.cmd action_repository_lists_followups_with_lanes_and_tags tags_and_action_items_round_trip_for_note action_repository_gets_actions_and_lists_suggested_with_note_context
```

Expected: PASS.

---

### Task 3: Backend Service And Commands

**Files:**
- Modify: `src-tauri/src/services/actions.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add failing service tests**

In `src-tauri/src/services/actions.rs`, add tests:

```rust
    #[test]
    fn accepting_suggested_action_makes_open_followup() {
        let repositories = repositories();
        let note = repositories.notes.create_raw_note("Follow up with Finance").unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Check finance export", None, None, Some(0.8))
            .unwrap();

        ActionItemService::new(repositories.clone()).accept(action.id).unwrap();

        let followups = ActionItemService::new(repositories).list_followups(100).unwrap();
        assert_eq!(followups.len(), 1);
        assert_eq!(followups[0].id, action.id);
        assert_eq!(followups[0].followup_state, Some(crate::domain::FollowupState::Open));
    }

    #[test]
    fn creates_manual_followup_from_note() {
        let repositories = repositories();
        let note = repositories.notes.create_raw_note("Manual task source").unwrap();

        let created = ActionItemService::new(repositories.clone())
            .create_manual_followup(note.id, "  Ask for rollout notes  ", Some("Launch"))
            .unwrap();

        assert_eq!(created.text, "Ask for rollout notes");
        assert_eq!(created.source, "user");
        assert_eq!(created.status, ActionStatus::Accepted);
        assert_eq!(created.followup_state, Some(crate::domain::FollowupState::Open));
        assert_eq!(created.followup_lane.as_deref(), Some("Launch"));
    }

    #[test]
    fn rejects_empty_manual_followup_text() {
        let repositories = repositories();
        let note = repositories.notes.create_raw_note("Manual task source").unwrap();

        let error = ActionItemService::new(repositories)
            .create_manual_followup(note.id, "   ", None)
            .unwrap_err();

        assert!(matches!(error, ServiceError::InvalidInput("follow-up text is required")));
    }

    #[test]
    fn updates_followup_state_and_lane() {
        let repositories = repositories();
        let note = repositories.notes.create_raw_note("Board task").unwrap();
        let action = repositories
            .action_items
            .create(note.id, "Update vendor", None, None, ActionStatus::Accepted, "parser", None)
            .unwrap();
        let service = ActionItemService::new(repositories.clone());

        service
            .update_followup_state(action.id, crate::domain::FollowupState::Blocked)
            .unwrap();
        service.update_followup_lane(action.id, Some("Ops")).unwrap();

        let followup = service.list_followups(100).unwrap().remove(0);
        assert_eq!(followup.followup_state, Some(crate::domain::FollowupState::Blocked));
        assert_eq!(followup.followup_lane.as_deref(), Some("Ops"));
    }

    #[test]
    fn rejects_followup_state_change_for_suggested_action() {
        let repositories = repositories();
        let note = repositories.notes.create_raw_note("Suggested only").unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Review this first", None, None, Some(0.8))
            .unwrap();

        let error = ActionItemService::new(repositories)
            .update_followup_state(action.id, crate::domain::FollowupState::Waiting)
            .unwrap_err();

        assert!(matches!(error, ServiceError::InvalidInput("action is not an active follow-up")));
    }
```

- [ ] **Step 2: Run service tests and verify they fail**

Run:

```powershell
scripts\cargo-test.cmd accepting_suggested_action_makes_open_followup creates_manual_followup_from_note updates_followup_state_and_lane
```

Expected: FAIL with missing service methods.

- [ ] **Step 3: Add service methods**

In `src-tauri/src/services/actions.rs`, update imports:

```rust
use crate::domain::{
    ActionItem, ActionItemId, ActionReviewItem, ActionStatus, FollowupItem, FollowupState, NoteId,
    ReviewStatus,
};
```

Add methods to `impl ActionItemService`:

```rust
    pub fn list_followups(&self, limit: u32) -> ServiceResult<Vec<FollowupItem>> {
        self.repositories
            .action_items
            .list_followups(limit)
            .map_err(Into::into)
    }

    pub fn create_manual_followup(
        &self,
        note_id: NoteId,
        text: &str,
        lane_override: Option<&str>,
    ) -> ServiceResult<ActionItem> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ServiceError::InvalidInput("follow-up text is required"));
        }

        if self.repositories.notes.get(note_id)?.is_none() {
            return Err(ServiceError::NotFound {
                entity: "note",
                id: note_id.to_string(),
            });
        }

        let lane = normalize_lane(lane_override)?;
        self.repositories
            .action_items
            .create_manual_followup(note_id, text, lane.as_deref())
            .map_err(Into::into)
    }

    pub fn update_followup_state(
        &self,
        id: ActionItemId,
        state: FollowupState,
    ) -> ServiceResult<()> {
        self.ensure_active_followup(id)?;
        self.repositories.action_items.set_followup_state(id, state)?;
        Ok(())
    }

    pub fn update_followup_lane(
        &self,
        id: ActionItemId,
        lane_override: Option<&str>,
    ) -> ServiceResult<()> {
        self.ensure_followup(id)?;
        let lane = normalize_lane(lane_override)?;
        self.repositories
            .action_items
            .set_followup_lane(id, lane.as_deref())?;
        Ok(())
    }

    fn ensure_active_followup(&self, id: ActionItemId) -> ServiceResult<ActionItem> {
        let action = self.ensure_followup(id)?;
        if action.status != ActionStatus::Accepted {
            return Err(ServiceError::InvalidInput("action is not an active follow-up"));
        }
        Ok(action)
    }

    fn ensure_followup(&self, id: ActionItemId) -> ServiceResult<ActionItem> {
        let action = self
            .repositories
            .action_items
            .get(id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            })?;

        match action.status {
            ActionStatus::Accepted | ActionStatus::Done => Ok(action),
            ActionStatus::Suggested | ActionStatus::Dismissed => Err(ServiceError::InvalidInput(
                "action is not an active follow-up",
            )),
        }
    }
```

Add helper outside the impl:

```rust
fn normalize_lane(value: Option<&str>) -> ServiceResult<Option<String>> {
    match value.map(str::trim) {
        Some("") => Err(ServiceError::InvalidInput("follow-up lane cannot be empty")),
        Some(value) => Ok(Some(value.to_string())),
        None => Ok(None),
    }
}
```

- [ ] **Step 4: Add command DTOs and commands**

In `src-tauri/src/commands.rs`, update domain imports to include `FollowupItem` and `FollowupState`.

Add DTOs after `ActionReviewItemDto`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowupItemDto {
    pub id: String,
    pub note_id: String,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
    pub tags: Vec<TagAssignmentDto>,
    pub created_at: DateTime<Utc>,
}

impl From<FollowupItem> for FollowupItemDto {
    fn from(item: FollowupItem) -> Self {
        Self {
            id: item.id.to_string(),
            note_id: item.note_id.to_string(),
            note_title: item.note_title,
            text: item.text,
            owner: item.owner,
            due_date: item.due_date,
            status: item.status,
            source: item.source,
            confidence: item.confidence,
            followup_state: item.followup_state,
            followup_lane: item.followup_lane,
            tags: item.tags.into_iter().map(Into::into).collect(),
            created_at: item.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualFollowupDto {
    pub note_id: String,
    pub text: String,
    pub lane: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFollowupStateDto {
    pub action_id: String,
    pub state: FollowupState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFollowupLaneDto {
    pub action_id: String,
    pub lane: Option<String>,
}
```

Add commands before `get_settings`:

```rust
#[tauri::command]
pub async fn list_followups(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<FollowupItemDto>, CommandError> {
    let items = ActionItemService::new(state.repositories.clone())
        .list_followups(limit.unwrap_or(200))?;
    Ok(items.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn create_manual_followup(
    state: tauri::State<'_, AppState>,
    input: CreateManualFollowupDto,
) -> Result<ActionItemDto, CommandError> {
    let note_id = parse_note_id(&input.note_id)?;
    let created = ActionItemService::new(state.repositories.clone())
        .create_manual_followup(note_id, &input.text, input.lane.as_deref())?;
    Ok(created.into())
}

#[tauri::command]
pub async fn update_followup_state(
    state: tauri::State<'_, AppState>,
    input: UpdateFollowupStateDto,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&input.action_id)?;
    ActionItemService::new(state.repositories.clone())
        .update_followup_state(action_id, input.state)?;
    Ok(())
}

#[tauri::command]
pub async fn update_followup_lane(
    state: tauri::State<'_, AppState>,
    input: UpdateFollowupLaneDto,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&input.action_id)?;
    ActionItemService::new(state.repositories.clone())
        .update_followup_lane(action_id, input.lane.as_deref())?;
    Ok(())
}
```

- [ ] **Step 5: Add command serialization test**

In `src-tauri/src/commands.rs` tests, import `FollowupItemDto` and `FollowupState`, then add:

```rust
    #[test]
    fn followup_item_dto_serializes_camel_case_fields() {
        let item = crate::domain::FollowupItem {
            id: ActionItemId::new(),
            note_id: NoteId::new(),
            note_title: "Visitor badge printer".to_string(),
            text: "Check badge printer template alignment.".to_string(),
            owner: Some("Rina".to_string()),
            due_date: None,
            status: crate::domain::ActionStatus::Accepted,
            source: "parser".to_string(),
            confidence: Some(0.78),
            followup_state: Some(FollowupState::Blocked),
            followup_lane: Some("Front desk".to_string()),
            tags: Vec::new(),
            created_at: Utc::now(),
        };

        let serialized = serde_json::to_value(FollowupItemDto::from(item)).unwrap();

        assert!(serialized.get("noteId").is_some());
        assert!(serialized.get("noteTitle").is_some());
        assert!(serialized.get("dueDate").is_some());
        assert!(serialized.get("followupState").is_some());
        assert!(serialized.get("followupLane").is_some());
        assert!(serialized.get("createdAt").is_some());
    }
```

- [ ] **Step 6: Register commands**

In `src-tauri/src/lib.rs`, add to `generate_handler!`:

```rust
            commands::list_followups,
            commands::create_manual_followup,
            commands::update_followup_state,
            commands::update_followup_lane,
```

- [ ] **Step 7: Run backend targeted tests**

Run:

```powershell
scripts\cargo-test.cmd accepting_suggested_action_makes_open_followup creates_manual_followup_from_note updates_followup_state_and_lane followup_item_dto_serializes_camel_case_fields
```

Expected: PASS.

---

### Task 4: Frontend Types, API, And Pure Grouping

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`
- Create: `src/lib/followups.ts`
- Create: `src/lib/followups.test.ts`

- [ ] **Step 1: Add failing helper tests**

Create `src/lib/followups.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import type { FollowupItem } from "./types";
import { displayFollowupState, groupFollowupsByLane, resolveFollowupLane } from "./followups";

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Visitor badge printer",
    text: "Check badge printer template alignment.",
    owner: "Rina",
    dueDate: null,
    status: "accepted",
    source: "parser",
    confidence: 0.78,
    followupState: "open",
    followupLane: null,
    createdAt: "2026-05-20T13:42:00.000Z",
    tags: [{ id: "tag-topic", name: "Badges", kind: "topic", source: "ai", confidence: 0.8 }],
    ...overrides,
  };
}

describe("resolveFollowupLane", () => {
  it("uses explicit lane, then project, then topic, then Unsorted", () => {
    expect(resolveFollowupLane(followup({ followupLane: "Ops" }))).toBe("Ops");
    expect(
      resolveFollowupLane(
        followup({
          tags: [
            { id: "topic", name: "Badges", kind: "topic", source: "ai", confidence: 0.8 },
            { id: "project", name: "Front desk", kind: "project", source: "ai", confidence: 0.9 },
          ],
        }),
      ),
    ).toBe("Front desk");
    expect(resolveFollowupLane(followup())).toBe("Badges");
    expect(resolveFollowupLane(followup({ tags: [] }))).toBe("Unsorted");
  });
});

describe("groupFollowupsByLane", () => {
  it("groups and counts active follow-ups by lane", () => {
    const groups = groupFollowupsByLane([
      followup({ id: "open", followupLane: "Ops", followupState: "open" }),
      followup({ id: "blocked", followupLane: "Ops", followupState: "blocked" }),
      followup({ id: "done", followupLane: "Ops", status: "done", followupState: null }),
    ]);

    expect(groups).toEqual([
      {
        name: "Ops",
        activeCount: 2,
        followups: [
          expect.objectContaining({ id: "open" }),
          expect.objectContaining({ id: "blocked" }),
          expect.objectContaining({ id: "done" }),
        ],
      },
    ]);
  });
});

describe("displayFollowupState", () => {
  it("maps done status to Done and active metadata to labels", () => {
    expect(displayFollowupState(followup({ status: "done", followupState: null }))).toBe("Done");
    expect(displayFollowupState(followup({ followupState: "waiting" }))).toBe("Waiting");
    expect(displayFollowupState(followup({ followupState: null }))).toBe("Open");
  });
});
```

- [ ] **Step 2: Run helper tests and verify they fail**

Run:

```powershell
npm test -- followups
```

Expected: FAIL because types/helpers do not exist.

- [ ] **Step 3: Add frontend types**

In `src/lib/types.ts`, add:

```ts
export type FollowupState = "open" | "waiting" | "blocked";

export type FollowupDisplayState = FollowupState | "done";
```

Extend `ActionItem`:

```ts
  followupState?: FollowupState | null;
  followupLane?: string | null;
```

Add:

```ts
export type FollowupItem = {
  id: string;
  noteId: string;
  noteTitle: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  status: Extract<ActionStatus, "accepted" | "done">;
  source: "parser" | "user" | string;
  confidence?: number | null;
  followupState?: FollowupState | null;
  followupLane?: string | null;
  tags: Tag[];
  createdAt: string;
};

export type FollowupLane = {
  name: string;
  activeCount: number;
  followups: FollowupItem[];
};
```

- [ ] **Step 4: Add pure helpers**

Create `src/lib/followups.ts`:

```ts
import type { FollowupItem, FollowupLane } from "./types";

export function resolveFollowupLane(followup: FollowupItem): string {
  const explicit = followup.followupLane?.trim();
  if (explicit) return explicit;

  const project = followup.tags.find((tag) => tag.kind === "project");
  if (project?.name.trim()) return project.name;

  const topic = followup.tags.find((tag) => tag.kind === "topic");
  if (topic?.name.trim()) return topic.name;

  return "Unsorted";
}

export function displayFollowupState(followup: FollowupItem): "Open" | "Waiting" | "Blocked" | "Done" {
  if (followup.status === "done") return "Done";
  switch (followup.followupState ?? "open") {
    case "waiting":
      return "Waiting";
    case "blocked":
      return "Blocked";
    case "open":
      return "Open";
  }
}

export function groupFollowupsByLane(followups: FollowupItem[]): FollowupLane[] {
  const lanes = new Map<string, FollowupItem[]>();

  for (const followup of followups) {
    const lane = resolveFollowupLane(followup);
    lanes.set(lane, [...(lanes.get(lane) ?? []), followup]);
  }

  return Array.from(lanes.entries())
    .map(([name, laneFollowups]) => ({
      name,
      activeCount: laneFollowups.filter((followup) => followup.status !== "done").length,
      followups: laneFollowups,
    }))
    .sort((left, right) => {
      if (left.name === "Unsorted") return 1;
      if (right.name === "Unsorted") return -1;
      return left.name.localeCompare(right.name);
    });
}
```

- [ ] **Step 5: Add API wrappers and normalizers**

In `src/lib/api.ts`, import `FollowupItem` and `FollowupState`.

Add wrappers:

```ts
export async function listFollowups(limit = 200): Promise<FollowupItem[]> {
  const followups = await invokeCommand<unknown[]>("list_followups", { limit });
  return followups.map(normalizeFollowupItem);
}

export async function createManualFollowup(noteId: string, text: string, lane?: string | null): Promise<ActionItem> {
  const action = await invokeCommand<unknown>("create_manual_followup", {
    input: { noteId, text, lane: lane?.trim() || null },
  });
  return normalizeActionItem(action);
}

export async function updateFollowupState(actionItemId: string, state: FollowupState): Promise<void> {
  await invokeCommand<void>("update_followup_state", { input: { actionId: actionItemId, state } });
}

export async function updateFollowupLane(actionItemId: string, lane?: string | null): Promise<void> {
  await invokeCommand<void>("update_followup_lane", {
    input: { actionId: actionItemId, lane: lane?.trim() || null },
  });
}
```

Add these functions to `api`.

Extend `normalizeActionItem`:

```ts
    followupState: normalizeFollowupState(getString(record, "followupState", "followup_state")),
    followupLane: getNullableString(record, "followupLane", "followup_lane"),
```

Add normalizers:

```ts
function normalizeFollowupItem(value: unknown): FollowupItem {
  const record = asRecord(value);
  const status = normalizeActionStatus(getString(record, "status"));

  return {
    id: getString(record, "id") ?? crypto.randomUUID(),
    noteId: getString(record, "noteId", "note_id") ?? "",
    noteTitle: getString(record, "noteTitle", "note_title") ?? "Untitled note",
    text: getString(record, "text") ?? "",
    owner: getNullableString(record, "owner"),
    dueDate: getNullableString(record, "dueDate", "due_date"),
    status: status === "done" ? "done" : "accepted",
    source: getString(record, "source") ?? "parser",
    confidence: getNumber(record, "confidence"),
    followupState: normalizeFollowupState(getString(record, "followupState", "followup_state")),
    followupLane: getNullableString(record, "followupLane", "followup_lane"),
    tags: getArray(record, "tags").map(normalizeTag),
    createdAt: getString(record, "createdAt", "created_at") ?? fallbackNow,
  };
}

function normalizeFollowupState(value: string | undefined): FollowupState | null {
  return value === "open" || value === "waiting" || value === "blocked" ? value : null;
}
```

- [ ] **Step 6: Add fallback behavior**

In `fallbackNotes`, add follow-up metadata to accepted action `a-1022`:

```ts
        followupState: "open",
        followupLane: null,
```

In `fallbackCommand`, add cases:

```ts
    case "list_followups":
      return fallbackNotes
        .filter((note) => !note.isArchived)
        .flatMap((note) =>
          note.actionItems
            .filter((action) => action.status === "accepted" || action.status === "done")
            .map((action) =>
              normalizeFollowupItem({
                ...action,
                noteTitle: note.title,
                tags: note.tags,
                createdAt: note.createdAt,
              }),
            ),
        )
        .slice(0, Number(args?.limit ?? 200)) as T;
    case "create_manual_followup": {
      const input = asRecord(args?.input);
      const note = fallbackNotes.find((item) => item.id === input.noteId);
      const text = String(input.text ?? "").trim();
      if (!note) throw new Error("note not found");
      if (!text) throw new Error("follow-up text is required");
      const action: ActionItem = {
        id: `manual-${Date.now()}`,
        noteId: note.id,
        text,
        owner: null,
        dueDate: null,
        status: "accepted",
        source: "user",
        confidence: null,
        noteTitle: note.title,
        followupState: "open",
        followupLane: getNullableString(input, "lane"),
      };
      note.actionItems = [...note.actionItems, action];
      note.actionItemCount = note.actionItems.length;
      note.updatedAt = new Date().toISOString();
      return normalizeActionItem(action) as T;
    }
    case "update_followup_state": {
      const input = asRecord(args?.input);
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === input.actionId);
      if (!action || action.status !== "accepted") throw new Error("action is not an active follow-up");
      action.followupState = normalizeFollowupState(getString(input, "state")) ?? "open";
      return undefined as T;
    }
    case "update_followup_lane": {
      const input = asRecord(args?.input);
      const action = fallbackNotes.flatMap((note) => note.actionItems).find((item) => item.id === input.actionId);
      if (!action || (action.status !== "accepted" && action.status !== "done")) throw new Error("action is not an active follow-up");
      action.followupLane = getNullableString(input, "lane");
      return undefined as T;
    }
```

In `accept_action_item` fallback, set:

```ts
        if (status === "accepted") {
          action.followupState = action.followupState ?? "open";
        }
```

- [ ] **Step 7: Run frontend helper tests**

Run:

```powershell
npm test -- followups
```

Expected: PASS.

---

### Task 5: Store Follow-ups Workflow

**Files:**
- Modify: `src/lib/stores/inbox.ts`
- Modify: `src/lib/stores/inbox.test.ts`

- [ ] **Step 1: Add failing store tests**

In `src/lib/stores/inbox.test.ts`, import `FollowupItem` and add helper:

```ts
function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "followup-1",
    noteId: "note-1",
    noteTitle: "Pricing export mismatch",
    text: "Check the export",
    owner: "Jordan",
    dueDate: null,
    status: "accepted",
    source: "parser",
    confidence: 0.8,
    followupState: "open",
    followupLane: null,
    createdAt: "2026-05-20T13:42:00.000Z",
    tags: [{ id: "tag-1", name: "Finance", kind: "topic", source: "ai", confidence: 0.92 }],
    ...overrides,
  };
}
```

Extend `testApi` with:

```ts
    listFollowups: vi.fn<(limit?: number) => Promise<FollowupItem[]>>().mockResolvedValue([]),
    createManualFollowup: vi
      .fn<(noteId: string, text: string, lane?: string | null) => Promise<ActionItem>>()
      .mockResolvedValue({
        id: "manual-action",
        noteId: "note-1",
        text: "Manual follow-up",
        status: "accepted",
        source: "user",
        followupState: "open",
        followupLane: null,
      }),
    updateFollowupState: vi.fn<(actionItemId: string, state: FollowupState) => Promise<void>>().mockResolvedValue(undefined),
    updateFollowupLane: vi.fn<(actionItemId: string, lane?: string | null) => Promise<void>>().mockResolvedValue(undefined),
```

Add tests:

```ts
  it("enters follow-ups view and loads follow-ups", async () => {
    const api = testApi({
      listFollowups: vi.fn().mockResolvedValue([followup({ id: "followup-loaded" })]),
    });
    const store = createWorkNotesStore(api);

    await store.showFollowups();

    expect(get(store.viewMode)).toBe("followups");
    expect(api.listFollowups).toHaveBeenCalledWith(200);
    expect(get(store.followups).map((item) => item.id)).toEqual(["followup-loaded"]);
  });

  it("refreshes follow-ups after accepting when follow-ups view is active", async () => {
    const api = testApi({
      listFollowups: vi.fn().mockResolvedValue([followup({ id: "after-accept" })]),
    });
    const store = createWorkNotesStore(api);

    await store.showFollowups();
    await store.acceptSuggestedAction("action-1");

    expect(api.acceptActionItem).toHaveBeenCalledWith("action-1");
    expect(api.listFollowups).toHaveBeenCalledTimes(2);
    expect(get(store.followups).map((item) => item.id)).toEqual(["after-accept"]);
  });

  it("creates a manual follow-up from the selected note and refreshes the board", async () => {
    const api = testApi({
      listFollowups: vi.fn().mockResolvedValue([followup({ id: "manual-action" })]),
    });
    const store = createWorkNotesStore(api);

    await store.selectNote("note-1");
    await store.createFollowupFromSelectedNote("  Ask Jordan for query  ", "Finance");

    expect(api.createManualFollowup).toHaveBeenCalledWith("note-1", "Ask Jordan for query", "Finance");
    expect(api.listFollowups).toHaveBeenCalled();
  });

  it("updates follow-up state and lane", async () => {
    const api = testApi();
    const store = createWorkNotesStore(api);

    await store.updateFollowupState("followup-1", "blocked");
    await store.updateFollowupLane("followup-1", "Ops");

    expect(api.updateFollowupState).toHaveBeenCalledWith("followup-1", "blocked");
    expect(api.updateFollowupLane).toHaveBeenCalledWith("followup-1", "Ops");
  });
```

- [ ] **Step 2: Run store tests and verify they fail**

Run:

```powershell
npm test -- inbox
```

Expected: FAIL with missing store properties and API methods.

- [ ] **Step 3: Extend store imports and types**

In `src/lib/stores/inbox.ts`, import new API methods and types:

```ts
  createManualFollowup,
  listFollowups,
  updateFollowupLane,
  updateFollowupState,
```

```ts
import type {
  ActionReviewItem,
  AppSettings,
  FollowupItem,
  FollowupState,
  InboxFilters,
  NoteDetail,
  NoteListItem,
} from "$lib/types";
```

Update `InboxViewMode`:

```ts
export type InboxViewMode = "inbox" | "archive" | "actions" | "today" | "people" | "followups";
```

Update `WorkNotesApi` and `defaultApi` with new methods.

- [ ] **Step 4: Add follow-up state and methods**

Inside `createWorkNotesStore`, add stores:

```ts
  const followups = writable<FollowupItem[]>([]);
  const loadingFollowups = writable(false);
```

Add methods:

```ts
  async function loadFollowups(limit = 200): Promise<void> {
    loadingFollowups.set(true);
    error.set(null);

    try {
      followups.set(await api.listFollowups(limit));
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not load follow-ups."));
    } finally {
      loadingFollowups.set(false);
    }
  }

  async function showFollowups(): Promise<void> {
    viewMode.set("followups");
    filters.set(createInboxFilters({ includeArchived: false }));
    await loadFollowups();
  }

  async function createFollowupFromSelectedNote(text: string, lane?: string | null): Promise<void> {
    const note = get(selectedNote);
    const trimmed = text.trim();
    if (!note || !trimmed) {
      return;
    }

    error.set(null);

    try {
      await api.createManualFollowup(note.id, trimmed, lane?.trim() || null);
      selectedNote.set(await api.getNote(note.id));
      await loadInbox();
      await loadFollowups();
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not create follow-up."));
      throw unknownError;
    }
  }

  async function changeFollowupState(actionItemId: string, state: FollowupState): Promise<void> {
    busyActionId.set(actionItemId);
    error.set(null);

    try {
      await api.updateFollowupState(actionItemId, state);
      await loadFollowups();
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not update follow-up."));
    } finally {
      busyActionId.set(null);
    }
  }

  async function changeFollowupLane(actionItemId: string, lane?: string | null): Promise<void> {
    busyActionId.set(actionItemId);
    error.set(null);

    try {
      await api.updateFollowupLane(actionItemId, lane?.trim() || null);
      await loadFollowups();
    } catch (unknownError) {
      error.set(errorMessage(unknownError, "Could not update follow-up lane."));
    } finally {
      busyActionId.set(null);
    }
  }
```

Extend `updateAction` after `loadInbox()`:

```ts
      if (get(viewMode) === "followups") {
        await loadFollowups();
      }
```

Return new stores and methods:

```ts
    followups,
    loadingFollowups,
    loadFollowups,
    showFollowups,
    createFollowupFromSelectedNote,
    updateFollowupState: changeFollowupState,
    updateFollowupLane: changeFollowupLane,
```

- [ ] **Step 5: Run store tests**

Run:

```powershell
npm test -- inbox
```

Expected: PASS.

---

### Task 6: FollowupsView Component

**Files:**
- Create: `src/lib/components/FollowupsView.svelte`
- Create: `src/lib/components/FollowupsView.test.ts`

- [ ] **Step 1: Add failing component tests**

Create `src/lib/components/FollowupsView.test.ts`:

```ts
import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { FollowupItem } from "$lib/types";
import FollowupsView from "./FollowupsView.svelte";

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "followup-1",
    noteId: "note-1",
    noteTitle: "Visitor badge printer",
    text: "Check badge printer template alignment.",
    owner: "Rina",
    dueDate: null,
    status: "accepted",
    source: "parser",
    confidence: 0.78,
    followupState: "open",
    followupLane: null,
    createdAt: "2026-05-20T13:42:00.000Z",
    tags: [{ id: "project", name: "Front desk", kind: "project", source: "ai", confidence: 0.9 }],
    ...overrides,
  };
}

describe("FollowupsView", () => {
  it("groups follow-ups by lane and opens source notes", async () => {
    const openNote = vi.fn();
    render(FollowupsView, {
      props: { followups: [followup()] },
      events: { openNote },
    });

    expect(screen.getByRole("heading", { name: "Follow-ups" })).toBeTruthy();
    expect(screen.getByText("Front desk")).toBeTruthy();
    expect(screen.getByText("Check badge printer template alignment.")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: /Open note:/ }));
    expect(openNote.mock.calls[0][0].detail).toBe("note-1");
  });

  it("dispatches state, lane, complete, and reopen events", async () => {
    const updateState = vi.fn();
    const updateLane = vi.fn();
    const complete = vi.fn();
    const reopen = vi.fn();
    render(FollowupsView, {
      props: {
        followups: [
          followup({ id: "active", status: "accepted", followupState: "open" }),
          followup({ id: "done", status: "done", followupState: null }),
        ],
      },
      events: { updateState, updateLane, complete, reopen },
    });

    await fireEvent.change(screen.getByLabelText("State for Check badge printer template alignment."), {
      target: { value: "blocked" },
    });
    await fireEvent.change(screen.getAllByLabelText("Lane for Check badge printer template alignment.")[0], {
      target: { value: "Ops" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Complete follow-up: Check badge printer template alignment." }));
    await fireEvent.click(screen.getByRole("button", { name: "Reopen follow-up: Check badge printer template alignment." }));

    expect(updateState.mock.calls[0][0].detail).toEqual({ id: "active", state: "blocked" });
    expect(updateLane.mock.calls[0][0].detail).toEqual({ id: "active", lane: "Ops" });
    expect(complete.mock.calls[0][0].detail).toBe("active");
    expect(reopen.mock.calls[0][0].detail).toBe("done");
  });

  it("shows a compact empty state", () => {
    render(FollowupsView, { props: { followups: [], loading: false } });
    expect(screen.getByText("No follow-ups yet")).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run component test and verify it fails**

Run:

```powershell
npm test -- FollowupsView
```

Expected: FAIL because component does not exist.

- [ ] **Step 3: Create component**

Create `src/lib/components/FollowupsView.svelte`:

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { displayFollowupState, groupFollowupsByLane } from "$lib/followups";
  import type { FollowupItem, FollowupState } from "$lib/types";

  type Props = {
    followups: FollowupItem[];
    loading?: boolean;
    busyActionId?: string | null;
  };

  let { followups, loading = false, busyActionId = null }: Props = $props();
  const lanes = $derived(groupFollowupsByLane(followups));

  const dispatch = createEventDispatcher<{
    openNote: string;
    updateState: { id: string; state: FollowupState };
    updateLane: { id: string; lane: string | null };
    complete: string;
    reopen: string;
  }>();

  function updateLane(id: string, value: string): void {
    dispatch("updateLane", { id, lane: value.trim() || null });
  }
</script>

<section class="followups-view" aria-label="Follow-ups">
  <header class="followups-head">
    <div>
      <p class="eyebrow">Project lanes</p>
      <h1>Follow-ups</h1>
    </div>
    <span class="load-state">{loading ? "Loading" : `${followups.length} total`}</span>
  </header>

  {#if lanes.length === 0}
    <div class="empty-state">
      <div class="empty-mark">WN</div>
      <h2>No follow-ups yet</h2>
      <p>Accepted actions and manual note follow-ups will appear here.</p>
    </div>
  {:else}
    <div class="lanes">
      {#each lanes as lane}
        <section class="lane" aria-label={lane.name}>
          <header class="lane-head">
            <h2>{lane.name}</h2>
            <span>{lane.activeCount} active</span>
          </header>

          <div class="followup-list">
            {#each lane.followups as followup}
              <article class:done={followup.status === "done"} class="followup-row">
                <button
                  class="row-open"
                  type="button"
                  aria-label={`Open note: ${followup.text}`}
                  onclick={() => dispatch("openNote", followup.noteId)}
                ></button>

                <div class="followup-body">
                  <strong>{followup.text}</strong>
                  <span>from "{followup.noteTitle}"</span>
                </div>

                {#if followup.status === "done"}
                  <span class="state-pill">{displayFollowupState(followup)}</span>
                  <button
                    class="icon-button"
                    type="button"
                    aria-label={`Reopen follow-up: ${followup.text}`}
                    disabled={loading || busyActionId === followup.id}
                    onclick={() => dispatch("reopen", followup.id)}
                  >
                    Reopen
                  </button>
                {:else}
                  <select
                    aria-label={`State for ${followup.text}`}
                    value={followup.followupState ?? "open"}
                    disabled={loading || busyActionId === followup.id}
                    onchange={(event) =>
                      dispatch("updateState", {
                        id: followup.id,
                        state: event.currentTarget.value as FollowupState,
                      })}
                  >
                    <option value="open">Open</option>
                    <option value="waiting">Waiting</option>
                    <option value="blocked">Blocked</option>
                  </select>
                  <input
                    aria-label={`Lane for ${followup.text}`}
                    value={followup.followupLane ?? lane.name}
                    disabled={loading || busyActionId === followup.id}
                    onblur={(event) => updateLane(followup.id, event.currentTarget.value)}
                  />
                  <button
                    class="icon-button"
                    type="button"
                    aria-label={`Complete follow-up: ${followup.text}`}
                    disabled={loading || busyActionId === followup.id}
                    onclick={() => dispatch("complete", followup.id)}
                  >
                    Done
                  </button>
                {/if}
              </article>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</section>

<style>
  .followups-view {
    min-height: 100vh;
    padding: 18px;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
  }

  .followups-head,
  .lane-head,
  .followup-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .followups-head {
    justify-content: space-between;
    margin-bottom: 14px;
  }

  .eyebrow,
  .load-state,
  .lane-head span,
  .followup-body span {
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 800;
  }

  h1,
  h2 {
    margin: 0;
    letter-spacing: 0;
  }

  h1 {
    font-size: 20px;
  }

  .lanes {
    display: grid;
    gap: 12px;
  }

  .lane {
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
  }

  .lane-head {
    justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-border-default);
  }

  .followup-list {
    display: grid;
  }

  .followup-row {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 120px 150px auto;
    padding: 9px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border-default) 60%, transparent);
  }

  .followup-row:last-child {
    border-bottom: 0;
  }

  .followup-row.done {
    opacity: 0.65;
  }

  .row-open {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .followup-body,
  .followup-row select,
  .followup-row input,
  .icon-button,
  .state-pill {
    position: relative;
    z-index: 1;
  }

  .followup-body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .followup-body strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  select,
  input,
  .icon-button,
  .state-pill {
    min-height: 28px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
  }

  input {
    min-width: 0;
    padding: 0 8px;
  }

  .icon-button,
  .state-pill {
    display: inline-grid;
    place-items: center;
    padding: 0 9px;
    font-weight: 800;
  }

  .empty-state {
    display: grid;
    place-items: center;
    min-height: 360px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-mark {
    display: grid;
    width: 52px;
    height: 52px;
    place-items: center;
    border-radius: 12px;
    color: var(--color-text-primary);
    background: var(--color-surface-2);
    font-weight: 900;
  }
</style>
```

- [ ] **Step 4: Run component tests**

Run:

```powershell
npm test -- FollowupsView
```

Expected: PASS. If the blur event does not fire in the second test, change the test to `await fireEvent.blur(...)` after setting the input value.

---

### Task 7: Navigation, Route Wiring, And Manual Entry

**Files:**
- Modify: `src/lib/components/AppShell.svelte`
- Modify: `src/lib/components/AppShell.test.ts`
- Modify: `src/lib/components/NoteDetail.svelte`
- Modify: `src/lib/components/NoteDetail.test.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Add failing AppShell nav test**

In `src/lib/components/AppShell.test.ts`, add:

```ts
  it("emits follow-ups navigation and marks it active", async () => {
    const navigate = vi.fn();
    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local",
        metrics: [{ label: "Follow-ups", value: "3" }],
        activeView: "followups",
      },
      events: { navigate },
    });

    const button = screen.getByRole("button", { name: "Follow-ups" });
    expect(button.getAttribute("aria-current")).toBe("page");

    await fireEvent.click(button);
    expect(navigate.mock.calls[0][0].detail).toBe("followups");
  });
```

- [ ] **Step 2: Add failing NoteDetail manual follow-up test**

In `src/lib/components/NoteDetail.test.ts`, add:

```ts
  it("dispatches manual follow-up text from the selected note and clears only after success", async () => {
    const createFollowup = vi.fn();
    render(NoteDetail, {
      props: { note: noteDetail() },
      events: { createFollowup },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Add follow-up" }));
    await fireEvent.input(screen.getByLabelText("Follow-up text"), {
      target: { value: "Ask Jordan for the export query" },
    });
    await fireEvent.input(screen.getByLabelText("Follow-up lane"), {
      target: { value: "Finance" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Create follow-up" }));

    expect(createFollowup.mock.calls[0][0].detail).toEqual(expect.objectContaining({
      text: "Ask Jordan for the export query",
      lane: "Finance",
    }));
    expect(screen.getByLabelText("Follow-up text")).toHaveValue("Ask Jordan for the export query");

    createFollowup.mock.calls[0][0].detail.done();
    expect(screen.queryByLabelText("Follow-up text")).toBeNull();
  });

  it("does not show manual follow-up creation for archived notes", () => {
    render(NoteDetail, {
      props: { note: { ...noteDetail(), isArchived: true } },
    });

    expect(screen.queryByRole("button", { name: "Add follow-up" })).toBeNull();
  });
```

- [ ] **Step 3: Run tests and verify they fail**

Run:

```powershell
npm test -- AppShell NoteDetail
```

Expected: FAIL because Follow-ups nav and `createFollowup` event do not exist.

- [ ] **Step 4: Update AppShell navigation**

In `src/lib/components/AppShell.svelte`, add:

```ts
  const followupsMetric = $derived(metrics.find((metric) => metric.label === "Follow-ups")?.value ?? "0");
```

Add nav button after Actions:

```svelte
      <button
        class:active={activeView === "followups"}
        type="button"
        aria-current={activeView === "followups" ? "page" : undefined}
        aria-label="Follow-ups"
        onclick={() => navigate("followups")}
      >
        <span aria-hidden="true">F</span>
        <span>Follow-ups</span>
        <strong aria-hidden="true">{followupsMetric}</strong>
      </button>
```

- [ ] **Step 5: Update NoteDetail manual entry**

In `src/lib/components/NoteDetail.svelte`, add state:

```ts
  let followupText = $state("");
  let followupLane = $state("");
  let followupOpen = $state(false);
```

Add event type:

```ts
    createFollowup: { text: string; lane: string | null; done: () => void };
```

Add helper:

```ts
  function clearFollowupForm() {
    followupText = "";
    followupLane = "";
    followupOpen = false;
  }

  function dispatchCreateFollowup() {
    const text = followupText.trim();
    if (!text) {
      return;
    }
    dispatch("createFollowup", {
      text,
      lane: followupLane.trim() || null,
      done: clearFollowupForm,
    });
  }
```

Add button in `.header-actions` before Archive, only for non-archived notes:

```svelte
        {#if !note.isArchived}
          <button class="ghost-button" type="button" onclick={() => (followupOpen = !followupOpen)} disabled={loading}>
            Add follow-up
          </button>
        {/if}
```

Add form before the Actions section:

```svelte
      {#if followupOpen}
        <section class="detail-section" aria-label="Create follow-up">
          <div class="section-head">
            <span>Add follow-up</span>
          </div>
          <div class="followup-form">
            <label>
              <span>Text</span>
              <input aria-label="Follow-up text" bind:value={followupText} disabled={loading} />
            </label>
            <label>
              <span>Lane</span>
              <input aria-label="Follow-up lane" bind:value={followupLane} disabled={loading} />
            </label>
            <button class="primary-action" type="button" onclick={dispatchCreateFollowup} disabled={loading || !followupText.trim()}>
              Create follow-up
            </button>
          </div>
        </section>
      {/if}
```

Add CSS:

```css
  .followup-form {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(120px, 0.7fr) auto;
    gap: 8px;
    align-items: end;
  }

  .followup-form label {
    display: grid;
    gap: 5px;
  }

  .followup-form label span {
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 800;
  }

  .followup-form input {
    min-height: 30px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 0 9px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
  }
```

- [ ] **Step 6: Wire route**

In `src/routes/+page.svelte`, import `FollowupsView`:

```ts
  import FollowupsView from "$lib/components/FollowupsView.svelte";
```

Destructure store state:

```ts
    followups,
    loadingFollowups,
```

Add metric:

```ts
    { label: "Follow-ups", value: String($followups.filter((item) => item.status !== "done").length) },
```

Update `onMount` initial loads:

```ts
      void workNotes.loadFollowups();
```

Update note captured listener:

```ts
        if (get(viewMode) === "followups") {
          void workNotes.showFollowups();
          return;
        }
```

Update `navigatePrimary`:

```ts
    if (event.detail === "followups") {
      await workNotes.showFollowups();
      return;
    }
```

Add helper:

```ts
  async function openNoteFromFollowup(noteId: string) {
    await workNotes.showInbox();
    await workNotes.selectNote(noteId);
  }

  async function createFollowupFromNote(event: CustomEvent<{ text: string; lane: string | null; done: () => void }>) {
    try {
      await workNotes.createFollowupFromSelectedNote(event.detail.text, event.detail.lane);
      event.detail.done();
    } catch {
      // Store owns the visible error; leaving the form open preserves entered text.
    }
  }
```

Add render branch before Today:

```svelte
    {#if $viewMode === "followups"}
      <FollowupsView
        followups={$followups}
        loading={$loadingFollowups}
        busyActionId={$busyActionId}
        on:openNote={(event) => void openNoteFromFollowup(event.detail)}
        on:updateState={(event) => void workNotes.updateFollowupState(event.detail.id, event.detail.state)}
        on:updateLane={(event) => void workNotes.updateFollowupLane(event.detail.id, event.detail.lane)}
        on:complete={(event) => void workNotes.completeAction(event.detail)}
        on:reopen={(event) => void workNotes.reopenAction(event.detail)}
      />
    {:else if $viewMode === "today"}
```

Wire `NoteDetail` event:

```svelte
            on:createFollowup={(event) => void createFollowupFromNote(event)}
```

- [ ] **Step 7: Run targeted frontend tests**

Run:

```powershell
npm test -- AppShell NoteDetail FollowupsView inbox followups
```

Expected: PASS.

---

### Task 8: Final Verification

**Files:**
- All files changed above.

- [ ] **Step 1: Run frontend tests**

Run:

```powershell
npm test
```

Expected: PASS.

- [ ] **Step 2: Run Svelte/TypeScript check**

Run:

```powershell
npm run check
```

Expected: PASS.

- [ ] **Step 3: Run frontend build**

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

- [ ] **Step 5: Check whitespace and worktree**

Run:

```powershell
git diff --check
git status --short --branch
```

Expected: `git diff --check` exits 0. `git status` shows only intentional Follow-ups implementation files plus pre-existing untracked files such as `work-notes-handoff.zip`.

---

## Manual Smoke Check

Run:

```powershell
npm run tauri dev
```

Check:

- Accept a suggested action in Note Detail or Actions.
- Open Follow-ups and confirm it appears under the project lane, topic lane, or `Unsorted`.
- Change the state to Waiting and Blocked.
- Complete it and confirm it displays as Done.
- Reopen it and confirm it returns to active state.
- Add a manual follow-up from a note.
- Confirm raw note text remains unchanged.
