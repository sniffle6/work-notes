use rusqlite::{params, Row};

use crate::db::Database;
use crate::domain::{ActionItem, ActionItemId, ActionStatus, NoteId};

use super::{RepositoryError, RepositoryResult};

#[derive(Clone)]
pub struct ActionItemRepository {
    db: Database,
}

impl ActionItemRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn create_suggested(
        &self,
        note_id: NoteId,
        text: &str,
        owner: Option<&str>,
        due_date: Option<&str>,
        confidence: Option<f64>,
    ) -> RepositoryResult<ActionItem> {
        self.create(
            note_id,
            text,
            owner,
            due_date,
            ActionStatus::Suggested,
            "parser",
            confidence,
        )
    }

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
        let id = ActionItemId::new();
        let id_text = id.to_string();
        let connection = self.db.connection()?;
        connection.execute(
            "INSERT INTO action_items (
                id, note_id, text, owner, due_date, status, source, confidence
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id_text,
                note_id.to_string(),
                text,
                owner,
                due_date,
                status.as_str(),
                source,
                confidence
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
        })
    }

    pub fn list_for_note(&self, note_id: NoteId) -> RepositoryResult<Vec<ActionItem>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, note_id, text, owner, due_date, status, source, confidence
             FROM action_items
             WHERE note_id = ?1
             ORDER BY rowid",
        )?;
        let records = statement
            .query_map([note_id.to_string()], ActionItemRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(ActionItemRecord::into_action_item)
            .collect()
    }

    pub fn set_status(&self, id: ActionItemId, status: ActionStatus) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        let changed = connection.execute(
            "UPDATE action_items SET status = ?2 WHERE id = ?1",
            params![id.to_string(), status.as_str()],
        )?;
        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    pub fn delete_for_note(&self, note_id: NoteId) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        connection.execute(
            "DELETE FROM action_items WHERE note_id = ?1",
            [note_id.to_string()],
        )?;
        Ok(())
    }
}

struct ActionItemRecord {
    id: String,
    note_id: String,
    text: String,
    owner: Option<String>,
    due_date: Option<String>,
    status: String,
    source: String,
    confidence: Option<f64>,
}

impl ActionItemRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            note_id: row.get(1)?,
            text: row.get(2)?,
            owner: row.get(3)?,
            due_date: row.get(4)?,
            status: row.get(5)?,
            source: row.get(6)?,
            confidence: row.get(7)?,
        })
    }

    fn into_action_item(self) -> RepositoryResult<ActionItem> {
        Ok(ActionItem {
            id: ActionItemId::parse(&self.id)?,
            note_id: NoteId::parse(&self.note_id)?,
            text: self.text,
            owner: self.owner,
            due_date: self.due_date,
            status: ActionStatus::from_db(&self.status)?,
            source: self.source,
            confidence: self.confidence,
        })
    }
}
