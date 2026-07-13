use rusqlite::{params, Row};

use crate::db::Database;
use crate::domain::{CardNote, CardNoteId, NoteId};

use super::{now_db_string, parse_db_datetime, RepositoryError, RepositoryResult};

#[derive(Clone)]
pub struct CardNoteRepository {
    db: Database,
}

impl CardNoteRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn add(&self, note_id: NoteId, text: &str) -> RepositoryResult<CardNote> {
        let id = CardNoteId::new();
        let created_at = now_db_string();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE notes SET updated_at = ?2 WHERE id = ?1",
            params![note_id.to_string(), &created_at],
        )?;
        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "note",
                id: note_id.to_string(),
            });
        }
        transaction.execute(
            "INSERT INTO card_notes (id, note_id, text, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id.to_string(), note_id.to_string(), text, &created_at],
        )?;
        transaction.commit()?;

        Ok(CardNote {
            id,
            note_id,
            text: text.to_string(),
            created_at: parse_db_datetime("created_at", created_at)?,
        })
    }

    pub fn list_for_note(&self, note_id: NoteId) -> RepositoryResult<Vec<CardNote>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, note_id, text, created_at
             FROM card_notes
             WHERE note_id = ?1
             ORDER BY created_at ASC, rowid ASC",
        )?;
        let records = statement
            .query_map([note_id.to_string()], CardNoteRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(CardNoteRecord::into_card_note)
            .collect()
    }
}

struct CardNoteRecord {
    id: String,
    note_id: String,
    text: String,
    created_at: String,
}

impl CardNoteRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            note_id: row.get(1)?,
            text: row.get(2)?,
            created_at: row.get(3)?,
        })
    }

    fn into_card_note(self) -> RepositoryResult<CardNote> {
        Ok(CardNote {
            id: CardNoteId::parse(&self.id)?,
            note_id: NoteId::parse(&self.note_id)?,
            text: self.text,
            created_at: parse_db_datetime("created_at", self.created_at)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::repositories::NoteRepository;

    use super::CardNoteRepository;

    #[test]
    fn card_notes_are_appended_and_listed_in_creation_order() {
        let db = Database::in_memory().expect("create database");
        let notes = NoteRepository::new(db.clone());
        let card_notes = CardNoteRepository::new(db);
        let note = notes.create_raw_note("Track rollout").expect("create note");

        let first = card_notes
            .add(note.id, "Waiting on the pilot group.")
            .expect("add first card note");
        let second = card_notes
            .add(note.id, "Pilot group approved the change.")
            .expect("add second card note");

        let stored = card_notes.list_for_note(note.id).expect("list card notes");
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].id, first.id);
        assert_eq!(stored[1].id, second.id);
        assert_eq!(stored[0].text, "Waiting on the pilot group.");
        assert_eq!(stored[1].text, "Pilot group approved the change.");
    }
}
