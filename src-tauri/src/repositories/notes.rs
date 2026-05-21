use rusqlite::{params, params_from_iter, OptionalExtension, Row, Transaction};

use crate::db::Database;
use crate::domain::{
    InboxFilters, Note, NoteId, NoteListItem, ParseJobId, ParseStatus, ReviewStatus,
};

use super::{now_db_string, parse_db_datetime, u32_from_i64, RepositoryError, RepositoryResult};

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

        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO notes (
                id, raw_text, cleaned_text, summary, created_at, updated_at,
                capture_source, parse_status, review_status, is_archived
             ) VALUES (?1, ?2, NULL, NULL, ?3, ?3, ?4, ?5, ?6, 0)",
            params![
                id_text,
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
            "UPDATE notes SET is_archived = 1, updated_at = ?2 WHERE id = ?1",
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

    pub fn get(&self, id: NoteId) -> RepositoryResult<Option<Note>> {
        let connection = self.db.connection()?;
        let id_text = id.to_string();
        let record = connection
            .query_row(
                "SELECT id, raw_text, cleaned_text, summary, created_at, updated_at,
                        capture_source, parse_status, review_status, is_archived
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

    pub fn apply_cleaned_text(
        &self,
        id: NoteId,
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
             SET cleaned_text = ?2, summary = ?3, updated_at = ?4
             WHERE id = ?1",
            params![id_text, cleaned_text, summary, now],
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
             WHERE notes_fts MATCH ?1 AND n.is_archived = 0
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
        n.raw_text,
        n.cleaned_text,
        n.summary,
        n.created_at,
        n.updated_at,
        n.parse_status,
        n.review_status,
        n.is_archived,
        (SELECT COUNT(*) FROM note_tags nt WHERE nt.note_id = n.id) AS tag_count,
        (SELECT COUNT(*) FROM action_items ai WHERE ai.note_id = n.id) AS action_item_count
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
    raw_text: String,
    cleaned_text: Option<String>,
    summary: Option<String>,
    created_at: String,
    updated_at: String,
    capture_source: String,
    parse_status: String,
    review_status: String,
    is_archived: i64,
}

impl NoteRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            raw_text: row.get(1)?,
            cleaned_text: row.get(2)?,
            summary: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            capture_source: row.get(6)?,
            parse_status: row.get(7)?,
            review_status: row.get(8)?,
            is_archived: row.get(9)?,
        })
    }

    fn into_note(self) -> RepositoryResult<Note> {
        Ok(Note {
            id: NoteId::parse(&self.id)?,
            raw_text: self.raw_text,
            cleaned_text: self.cleaned_text,
            summary: self.summary,
            created_at: parse_db_datetime("created_at", self.created_at)?,
            updated_at: parse_db_datetime("updated_at", self.updated_at)?,
            capture_source: self.capture_source,
            parse_status: ParseStatus::from_db(&self.parse_status)?,
            review_status: ReviewStatus::from_db(&self.review_status)?,
            is_archived: self.is_archived != 0,
        })
    }
}

struct NoteListItemRecord {
    id: String,
    raw_text: String,
    cleaned_text: Option<String>,
    summary: Option<String>,
    created_at: String,
    updated_at: String,
    parse_status: String,
    review_status: String,
    is_archived: i64,
    tag_count: i64,
    action_item_count: i64,
}

impl NoteListItemRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            raw_text: row.get(1)?,
            cleaned_text: row.get(2)?,
            summary: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            parse_status: row.get(6)?,
            review_status: row.get(7)?,
            is_archived: row.get(8)?,
            tag_count: row.get(9)?,
            action_item_count: row.get(10)?,
        })
    }

    fn into_list_item(self) -> RepositoryResult<NoteListItem> {
        Ok(NoteListItem {
            id: NoteId::parse(&self.id)?,
            raw_text: self.raw_text,
            cleaned_text: self.cleaned_text,
            summary: self.summary,
            created_at: parse_db_datetime("created_at", self.created_at)?,
            updated_at: parse_db_datetime("updated_at", self.updated_at)?,
            parse_status: ParseStatus::from_db(&self.parse_status)?,
            review_status: ReviewStatus::from_db(&self.review_status)?,
            is_archived: self.is_archived != 0,
            tag_count: u32_from_i64("tag_count", self.tag_count)?,
            action_item_count: u32_from_i64("action_item_count", self.action_item_count)?,
        })
    }
}
