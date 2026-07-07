use rusqlite::{params, OptionalExtension, Row};

use crate::db::Database;
use crate::domain::{NoteId, Tag, TagAssignment, TagId, TagKind};

use super::{now_db_string, parse_db_datetime, RepositoryError, RepositoryResult};

#[derive(Clone)]
pub struct TagRepository {
    db: Database,
}

impl TagRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, name: &str, kind: TagKind) -> RepositoryResult<Tag> {
        let normalized = name.trim();

        if let Some(tag) = self.find_by_name_and_kind(normalized, kind.clone())? {
            return Ok(tag);
        }

        let id = TagId::new();
        let id_text = id.to_string();
        let created_at = now_db_string();
        {
            let connection = self.db.connection()?;
            connection.execute(
                "INSERT INTO tags (id, name, kind, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(name, kind) DO NOTHING",
                params![id_text, normalized, kind.as_str(), created_at],
            )?;
        }

        self.find_by_name_and_kind(normalized, kind)?
            .ok_or_else(|| RepositoryError::NotFound {
                entity: "tag",
                id: id.to_string(),
            })
    }

    pub fn apply_to_note(
        &self,
        note_id: NoteId,
        tag_id: TagId,
        source: &str,
        confidence: Option<f64>,
    ) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        connection.execute(
            "INSERT INTO note_tags (note_id, tag_id, source, confidence)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(note_id, tag_id, source)
             DO UPDATE SET confidence = excluded.confidence",
            params![note_id.to_string(), tag_id.to_string(), source, confidence],
        )?;
        Ok(())
    }

    pub fn list_for_note(&self, note_id: NoteId) -> RepositoryResult<Vec<TagAssignment>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT t.id, t.name, t.kind, t.created_at, nt.source, nt.confidence
             FROM note_tags nt
             JOIN tags t ON t.id = nt.tag_id
             WHERE nt.note_id = ?1
             ORDER BY t.kind, t.name",
        )?;
        let records = statement
            .query_map([note_id.to_string()], TagAssignmentRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(TagAssignmentRecord::into_assignment)
            .collect()
    }

    pub fn remove_from_note(
        &self,
        note_id: NoteId,
        tag_id: TagId,
        source: &str,
    ) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        connection.execute(
            "DELETE FROM note_tags
             WHERE note_id = ?1 AND tag_id = ?2 AND source = ?3",
            params![note_id.to_string(), tag_id.to_string(), source],
        )?;
        Ok(())
    }

    pub fn remove_parser_assignments_for_note(&self, note_id: NoteId) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        connection.execute(
            "DELETE FROM note_tags WHERE note_id = ?1 AND source = ?2",
            params![note_id.to_string(), "parser"],
        )?;
        Ok(())
    }

    fn find_by_name_and_kind(&self, name: &str, kind: TagKind) -> RepositoryResult<Option<Tag>> {
        let connection = self.db.connection()?;
        let record = connection
            .query_row(
                "SELECT id, name, kind, created_at
                 FROM tags
                 WHERE name = ?1 AND kind = ?2",
                params![name, kind.as_str()],
                TagRecord::from_row,
            )
            .optional()?;
        record.map(TagRecord::into_tag).transpose()
    }
}

struct TagRecord {
    id: String,
    name: String,
    kind: String,
    created_at: String,
}

impl TagRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            created_at: row.get(3)?,
        })
    }

    fn into_tag(self) -> RepositoryResult<Tag> {
        Ok(Tag {
            id: TagId::parse(&self.id)?,
            name: self.name,
            kind: TagKind::from_db(&self.kind)?,
            created_at: parse_db_datetime("created_at", self.created_at)?,
        })
    }
}

struct TagAssignmentRecord {
    tag: TagRecord,
    source: String,
    confidence: Option<f64>,
}

impl TagAssignmentRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            tag: TagRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                created_at: row.get(3)?,
            },
            source: row.get(4)?,
            confidence: row.get(5)?,
        })
    }

    fn into_assignment(self) -> RepositoryResult<TagAssignment> {
        Ok(TagAssignment {
            tag: self.tag.into_tag()?,
            source: self.source,
            confidence: self.confidence,
        })
    }
}
