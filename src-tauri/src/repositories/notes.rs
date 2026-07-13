use rusqlite::{params, params_from_iter, OptionalExtension, Row, Transaction};

use crate::db::Database;
use crate::domain::{
    InboxFilters, Note, NoteId, NoteListItem, ParseJobId, ParseStatus, ReviewStatus,
};

use super::{
    now_db_string, optional_db_datetime, parse_db_datetime, u32_from_i64, RepositoryError,
    RepositoryResult,
};

#[derive(Clone)]
pub struct NoteRepository {
    db: Database,
}

impl NoteRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn create_raw_note(&self, raw_text: &str) -> RepositoryResult<Note> {
        let id = NoteId::new();
        let id_text = id.to_string();
        let now = now_db_string();
        let parse_status = ParseStatus::Queued;
        let review_status = ReviewStatus::None;
        let capture_source = "quick_capture";
        let title = make_title(raw_text);

        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO notes (
                id, title, raw_text, cleaned_text, summary, created_at, updated_at,
                capture_source, parse_status, review_status, is_archived
             ) VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?4, ?5, ?6, ?7, 0)",
            params![
                id_text,
                title,
                raw_text,
                now,
                capture_source,
                parse_status.as_str(),
                review_status.as_str()
            ],
        )?;
        replace_fts(&transaction, &id_text, raw_text, None, None)?;
        transaction.execute(
            "INSERT INTO parse_jobs (
                id, note_id, status, attempt_count, last_error, created_at, started_at, finished_at
             ) VALUES (?1, ?2, ?3, 0, NULL, ?4, NULL, NULL)",
            params![
                ParseJobId::new().to_string(),
                id_text,
                ParseStatus::Queued.as_str(),
                now
            ],
        )?;
        transaction.commit()?;
        drop(connection);

        self.get(id)?.ok_or_else(|| RepositoryError::NotFound {
            entity: "note",
            id: id.to_string(),
        })
    }

    pub fn archive(&self, id: NoteId) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let connection = self.db.connection()?;
        let changed = connection.execute(
            "UPDATE notes
             SET is_archived = 1, completed_at = NULL, updated_at = ?2
             WHERE id = ?1",
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

    pub fn complete(&self, id: NoteId, completion_note: Option<&str>) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let connection = self.db.connection()?;
        let changed = connection.execute(
            "UPDATE notes
             SET completed_at = ?2, completion_note = ?3, is_archived = 0, updated_at = ?2
             WHERE id = ?1",
            params![id_text, now, completion_note],
        )?;

        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "note",
                id: id.to_string(),
            });
        }

        Ok(())
    }

    pub fn reopen_completed(&self, id: NoteId) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let connection = self.db.connection()?;
        let changed = connection.execute(
            "UPDATE notes SET completed_at = NULL, updated_at = ?2 WHERE id = ?1",
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

        transaction.execute(
            "DELETE FROM card_notes WHERE note_id = ?1",
            params![&id_text],
        )?;
        transaction.execute(
            "DELETE FROM note_tags WHERE note_id = ?1",
            params![&id_text],
        )?;
        transaction.execute(
            "DELETE FROM action_items WHERE note_id = ?1",
            params![&id_text],
        )?;
        transaction.execute(
            "DELETE FROM parse_jobs WHERE note_id = ?1",
            params![&id_text],
        )?;
        transaction.execute(
            "DELETE FROM parse_runs WHERE note_id = ?1",
            params![&id_text],
        )?;
        transaction.execute(
            "DELETE FROM notes_fts WHERE note_id = ?1",
            params![&id_text],
        )?;
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

    pub fn get(&self, id: NoteId) -> RepositoryResult<Option<Note>> {
        let connection = self.db.connection()?;
        let id_text = id.to_string();
        let record = connection
            .query_row(
                "SELECT id, title, raw_text, cleaned_text, summary, created_at, updated_at,
                        capture_source, parse_status, review_status, is_archived, completed_at,
                        completion_note, cleaned_edited
                 FROM notes
                 WHERE id = ?1",
                [id_text],
                NoteRecord::from_row,
            )
            .optional()?;

        record.map(NoteRecord::into_note).transpose()
    }

    pub fn list_inbox(&self, filters: InboxFilters) -> RepositoryResult<Vec<NoteListItem>> {
        let connection = self.db.connection()?;
        let mut sql = list_item_select();
        let mut values = Vec::new();
        let mut conditions = Vec::new();

        if !filters.include_archived {
            conditions.push("n.is_archived = 0".to_string());
        }

        if !filters.include_completed {
            conditions.push("n.completed_at IS NULL".to_string());
        }

        if let Some(status) = filters.parse_status {
            conditions.push("n.parse_status = ?".to_string());
            values.push(status.as_str().to_string());
        }

        if let Some(status) = filters.review_status {
            conditions.push("n.review_status = ?".to_string());
            values.push(status.as_str().to_string());
        }

        for tag_id in filters.tag_ids {
            conditions.push(
                "EXISTS (
                    SELECT 1
                    FROM note_tags nt
                    WHERE nt.note_id = n.id AND nt.tag_id = ?
                 )"
                .to_string(),
            );
            values.push(tag_id.to_string());
        }

        if let Some(query) = filters.query.as_deref().and_then(fts_query) {
            sql.push_str(" JOIN notes_fts ON notes_fts.note_id = n.id");
            conditions.push("notes_fts MATCH ?".to_string());
            values.push(query);
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY n.created_at DESC");
        let limit = filters.limit.unwrap_or(200).clamp(1, 1000);
        sql.push_str(&format!(" LIMIT {limit}"));

        let mut statement = connection.prepare(&sql)?;
        let records = statement
            .query_map(
                params_from_iter(values.iter()),
                NoteListItemRecord::from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(NoteListItemRecord::into_list_item)
            .collect()
    }

    pub fn apply_cleaned_note(
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
             SET title = ?2, cleaned_text = ?3, summary = ?4, updated_at = ?5
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

    pub fn update_raw_text_by_user(&self, id: NoteId, raw_text: &str) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;

        transaction
            .query_row(
                "SELECT id FROM notes WHERE id = ?1",
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
             SET title = ?2,
                 raw_text = ?3,
                 cleaned_text = NULL,
                 summary = NULL,
                 cleaned_edited = 0,
                 updated_at = ?4
             WHERE id = ?1",
            params![id_text, make_title(raw_text), raw_text, now],
        )?;
        replace_fts(&transaction, &id_text, raw_text, None, None)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_parse_status(&self, id: NoteId, status: ParseStatus) -> RepositoryResult<()> {
        self.update_status(id, "parse_status", status.as_str())
    }

    pub fn set_review_status(&self, id: NoteId, status: ReviewStatus) -> RepositoryResult<()> {
        self.update_status(id, "review_status", status.as_str())
    }

    pub fn search(&self, query: &str) -> RepositoryResult<Vec<NoteListItem>> {
        let Some(query) = fts_query(query) else {
            return Ok(Vec::new());
        };

        let connection = self.db.connection()?;
        let sql = format!(
            "{} JOIN notes_fts ON notes_fts.note_id = n.id
             WHERE notes_fts MATCH ?1 AND n.is_archived = 0 AND n.completed_at IS NULL
             ORDER BY bm25(notes_fts), n.created_at DESC
             LIMIT 200",
            list_item_select()
        );

        let mut statement = connection.prepare(&sql)?;
        let records = statement
            .query_map([query], NoteListItemRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(NoteListItemRecord::into_list_item)
            .collect()
    }

    fn update_status(&self, id: NoteId, column: &str, status: &str) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let connection = self.db.connection()?;
        let changed = connection.execute(
            &format!("UPDATE notes SET {column} = ?2, updated_at = ?3 WHERE id = ?1"),
            params![id_text, status, now],
        )?;

        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "note",
                id: id.to_string(),
            });
        }

        Ok(())
    }
}

fn replace_fts(
    transaction: &Transaction<'_>,
    id: &str,
    raw_text: &str,
    cleaned_text: Option<&str>,
    summary: Option<&str>,
) -> rusqlite::Result<()> {
    transaction.execute("DELETE FROM notes_fts WHERE note_id = ?1", [id])?;
    transaction.execute(
        "INSERT INTO notes_fts (note_id, raw_text, cleaned_text, summary)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, raw_text, cleaned_text, summary],
    )?;
    Ok(())
}

fn list_item_select() -> String {
    "SELECT DISTINCT
        n.id,
        n.title,
        n.raw_text,
        n.cleaned_text,
        n.summary,
        n.created_at,
        n.updated_at,
        n.parse_status,
        n.review_status,
        n.is_archived,
        n.completed_at,
        (SELECT COUNT(*) FROM note_tags nt WHERE nt.note_id = n.id) AS tag_count,
        (SELECT COUNT(*) FROM action_items ai WHERE ai.note_id = n.id) AS action_item_count,
        (
            SELECT COUNT(*)
            FROM action_items ai
            WHERE ai.note_id = n.id AND ai.status = 'suggested'
        ) AS suggested_action_item_count
     FROM notes n"
        .to_string()
}

fn fts_query(query: &str) -> Option<String> {
    let terms = query
        .split_whitespace()
        .filter_map(|term| {
            let sanitized = term
                .chars()
                .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>();
            (!sanitized.is_empty()).then(|| format!("{sanitized}*"))
        })
        .collect::<Vec<_>>();

    (!terms.is_empty()).then(|| terms.join(" "))
}

struct NoteRecord {
    id: String,
    title: String,
    raw_text: String,
    cleaned_text: Option<String>,
    summary: Option<String>,
    created_at: String,
    updated_at: String,
    capture_source: String,
    parse_status: String,
    review_status: String,
    is_archived: i64,
    completed_at: Option<String>,
    completion_note: Option<String>,
    cleaned_edited: i64,
}

impl NoteRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            raw_text: row.get(2)?,
            cleaned_text: row.get(3)?,
            summary: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            capture_source: row.get(7)?,
            parse_status: row.get(8)?,
            review_status: row.get(9)?,
            is_archived: row.get(10)?,
            completed_at: row.get(11)?,
            completion_note: row.get(12)?,
            cleaned_edited: row.get(13)?,
        })
    }

    fn into_note(self) -> RepositoryResult<Note> {
        Ok(Note {
            id: NoteId::parse(&self.id)?,
            title: self.title,
            raw_text: self.raw_text,
            cleaned_text: self.cleaned_text,
            summary: self.summary,
            created_at: parse_db_datetime("created_at", self.created_at)?,
            updated_at: parse_db_datetime("updated_at", self.updated_at)?,
            capture_source: self.capture_source,
            parse_status: ParseStatus::from_db(&self.parse_status)?,
            review_status: ReviewStatus::from_db(&self.review_status)?,
            is_archived: self.is_archived != 0,
            completed_at: optional_db_datetime("completed_at", self.completed_at)?,
            completion_note: self.completion_note,
            cleaned_edited: self.cleaned_edited != 0,
        })
    }
}

struct NoteListItemRecord {
    id: String,
    title: String,
    raw_text: String,
    cleaned_text: Option<String>,
    summary: Option<String>,
    created_at: String,
    updated_at: String,
    parse_status: String,
    review_status: String,
    is_archived: i64,
    completed_at: Option<String>,
    tag_count: i64,
    action_item_count: i64,
    suggested_action_item_count: i64,
}

impl NoteListItemRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            raw_text: row.get(2)?,
            cleaned_text: row.get(3)?,
            summary: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            parse_status: row.get(7)?,
            review_status: row.get(8)?,
            is_archived: row.get(9)?,
            completed_at: row.get(10)?,
            tag_count: row.get(11)?,
            action_item_count: row.get(12)?,
            suggested_action_item_count: row.get(13)?,
        })
    }

    fn into_list_item(self) -> RepositoryResult<NoteListItem> {
        Ok(NoteListItem {
            id: NoteId::parse(&self.id)?,
            title: self.title,
            raw_text: self.raw_text,
            cleaned_text: self.cleaned_text,
            summary: self.summary,
            created_at: parse_db_datetime("created_at", self.created_at)?,
            updated_at: parse_db_datetime("updated_at", self.updated_at)?,
            parse_status: ParseStatus::from_db(&self.parse_status)?,
            review_status: ReviewStatus::from_db(&self.review_status)?,
            is_archived: self.is_archived != 0,
            completed_at: optional_db_datetime("completed_at", self.completed_at)?,
            tag_count: u32_from_i64("tag_count", self.tag_count)?,
            action_item_count: u32_from_i64("action_item_count", self.action_item_count)?,
            suggested_action_item_count: u32_from_i64(
                "suggested_action_item_count",
                self.suggested_action_item_count,
            )?,
        })
    }
}

fn make_title(raw_text: &str) -> String {
    let first_line = raw_text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Untitled note");
    normalize_title(first_line)
}

fn normalize_title(title: &str) -> String {
    let title = title.trim();
    let title = if title.is_empty() {
        "Untitled note"
    } else {
        title
    };

    if title.chars().count() > 80 {
        format!("{}...", title.chars().take(77).collect::<String>())
    } else {
        title.to_string()
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use crate::db::Database;
    use crate::domain::{InboxFilters, TagKind};
    use crate::repositories::{
        ActionItemRepository, NoteRepository, ParseJobRepository, TagRepository,
    };

    fn setup_notes() -> (Database, NoteRepository) {
        let db = Database::in_memory().unwrap();
        let notes = NoteRepository::new(db.clone());
        (db, notes)
    }

    #[test]
    fn new_note_defaults_to_not_cleaned_edited() {
        let (db, notes) = setup_notes();
        let _keep_db_alive = db;
        let note = notes.create_raw_note("test note").expect("create note");
        let stored = notes.get(note.id).expect("get note").expect("note exists");
        assert!(!stored.cleaned_edited);
    }

    #[test]
    fn update_cleaned_by_user_sets_fields_and_marks_edited() {
        let (db, notes) = setup_notes();
        let _keep_db_alive = db;
        let note = notes.create_raw_note("original note").expect("create note");
        let id = note.id;
        notes
            .apply_cleaned_note(id, "Parsed Title", "## Parsed body", "Parsed summary")
            .expect("apply cleaned note");

        notes
            .update_cleaned_by_user(id, "My Title", "## Edited body", "Edited summary")
            .expect("update cleaned by user");

        let stored = notes.get(id).expect("get note").expect("note exists");
        assert_eq!(stored.title, "My Title");
        assert_eq!(stored.cleaned_text.as_deref(), Some("## Edited body"));
        assert_eq!(stored.summary.as_deref(), Some("Edited summary"));
        assert!(stored.cleaned_edited);
    }

    #[test]
    fn update_raw_text_by_user_invalidates_cleaned_output_and_fts() {
        let (db, notes) = setup_notes();
        let _keep_db_alive = db;
        let note = notes.create_raw_note("original note").expect("create note");
        let id = note.id;
        notes
            .apply_cleaned_note(id, "Parsed Title", "## Parsed body", "Parsed summary")
            .expect("apply cleaned note");

        notes
            .update_raw_text_by_user(id, "updated source mentions calico rollout")
            .expect("update raw text");

        let stored = notes.get(id).expect("get note").expect("note exists");
        assert_eq!(stored.raw_text, "updated source mentions calico rollout");
        assert_eq!(stored.title, "updated source mentions calico rollout");
        assert_eq!(stored.cleaned_text, None);
        assert_eq!(stored.summary, None);
        assert!(!stored.cleaned_edited);
        assert_eq!(
            notes.search("calico").expect("search updated raw text")[0].id,
            id
        );
        assert!(notes
            .search("original")
            .expect("search old raw text")
            .is_empty());
        assert!(notes
            .search("parsed")
            .expect("search old cleaned text")
            .is_empty());
    }

    #[test]
    fn update_cleaned_by_user_with_empty_title_defaults_to_untitled_note() {
        let (db, notes) = setup_notes();
        let _keep_db_alive = db;
        let note = notes.create_raw_note("original note").expect("create note");
        let id = note.id;
        notes
            .apply_cleaned_note(id, "Parsed Title", "## Parsed body", "Parsed summary")
            .expect("apply cleaned note");

        notes
            .update_cleaned_by_user(id, "", "body", "summary")
            .expect("update cleaned by user");

        let stored = notes.get(id).expect("get note").expect("note exists");
        assert_eq!(stored.title, "Untitled note");
    }

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
            .list_inbox(InboxFilters::default())
            .expect("list default inbox");
        assert!(inbox.iter().any(|item| item.id == note.id));
    }

    #[test]
    fn completed_note_is_hidden_from_inbox_without_becoming_archived() {
        let (db, notes) = setup_notes();
        let _keep_db_alive = db;
        let note = notes.create_raw_note("finish this").expect("create note");

        notes
            .complete(note.id, Some("Deployed and verified."))
            .expect("complete note");

        let completed = notes.get(note.id).expect("get note").expect("note exists");
        assert!(completed.completed_at.is_some());
        assert_eq!(
            completed.completion_note.as_deref(),
            Some("Deployed and verified.")
        );
        assert!(!completed.is_archived);
        assert!(notes
            .list_inbox(InboxFilters::default())
            .expect("list default inbox")
            .is_empty());
    }

    #[test]
    fn completed_note_can_be_listed_and_reopened() {
        let (db, notes) = setup_notes();
        let _keep_db_alive = db;
        let note = notes.create_raw_note("finish this").expect("create note");

        notes.complete(note.id, None).expect("complete note");
        let completed = notes
            .list_inbox(InboxFilters {
                include_completed: true,
                ..InboxFilters::default()
            })
            .expect("list completed notes");
        assert_eq!(completed.len(), 1);
        assert!(completed[0].completed_at.is_some());

        notes.reopen_completed(note.id).expect("reopen note");
        let reopened = notes.get(note.id).expect("get note").expect("note exists");
        assert_eq!(reopened.completed_at, None);
        assert!(notes
            .list_inbox(InboxFilters::default())
            .expect("list default inbox")
            .iter()
            .any(|item| item.id == note.id));
    }

    #[test]
    fn permanently_delete_archived_note_removes_dependents() {
        let (db, notes) = setup_notes();
        let tags = TagRepository::new(db.clone());
        let actions = ActionItemRepository::new(db.clone());
        let parse_jobs = ParseJobRepository::new(db.clone());
        let card_notes = crate::repositories::CardNoteRepository::new(db.clone());
        let note = notes.create_raw_note("delete me").expect("create note");

        let tag = tags.upsert("cleanup", TagKind::Topic).expect("create tag");
        actions
            .create_suggested(note.id, "Follow up", None, None, Some(0.8))
            .expect("add action");
        tags.apply_to_note(note.id, tag.id, "parser", Some(0.8))
            .expect("tag note");
        card_notes
            .add(note.id, "Temporary investigation note")
            .expect("add card note");

        parse_jobs
            .record_run(note.id, "test", "test-v1", "raw response", "{}", None)
            .expect("record parse run");

        notes.archive(note.id).expect("archive note");
        notes.permanently_delete(note.id).expect("delete note");

        assert!(notes.get(note.id).expect("get deleted note").is_none());

        let connection = db.connection().expect("connect db");
        for table in [
            "note_tags",
            "card_notes",
            "action_items",
            "parse_jobs",
            "parse_runs",
            "notes_fts",
        ] {
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
}
