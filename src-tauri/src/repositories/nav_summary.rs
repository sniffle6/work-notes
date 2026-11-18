use rusqlite::params;

use crate::db::Database;
use crate::domain::{ActionStatus, NavSummary, ParseStatus};

use super::{u32_from_i64, RepositoryResult};

/// Reads the counts behind the navigation badges with aggregate queries, so
/// refreshing them never loads note rows.
#[derive(Clone)]
pub struct NavSummaryRepository {
    db: Database,
}

impl NavSummaryRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Counts the whole workspace. Tag names are sorted by SQLite's binary
    /// collation; the UI sorts them again for display.
    pub fn load(&self) -> RepositoryResult<NavSummary> {
        let connection = self.db.connection()?;
        // Action counts follow the review and follow-up lists: they skip
        // archived notes but keep actions on completed notes. An open
        // follow-up is accepted; done and dropped ones have other statuses.
        let (inbox, parse_failed, parse_queue, needs_review, followups) = connection.query_row(
            "SELECT
                (SELECT COUNT(*) FROM notes
                 WHERE is_archived = 0 AND completed_at IS NULL),
                (SELECT COUNT(*) FROM notes
                 WHERE is_archived = 0 AND completed_at IS NULL AND parse_status = ?1),
                (SELECT COUNT(*) FROM notes
                 WHERE is_archived = 0 AND completed_at IS NULL AND parse_status IN (?2, ?3)),
                (SELECT COUNT(*) FROM action_items ai JOIN notes n ON n.id = ai.note_id
                 WHERE n.is_archived = 0 AND ai.status = ?4),
                (SELECT COUNT(*) FROM action_items ai JOIN notes n ON n.id = ai.note_id
                 WHERE n.is_archived = 0 AND ai.status = ?5)",
            params![
                ParseStatus::Failed.as_str(),
                ParseStatus::Queued.as_str(),
                ParseStatus::Parsing.as_str(),
                ActionStatus::Suggested.as_str(),
                ActionStatus::Accepted.as_str(),
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )?;

        let mut statement = connection.prepare(
            "SELECT DISTINCT t.name
             FROM tags t
             JOIN note_tags nt ON nt.tag_id = t.id
             JOIN notes n ON n.id = nt.note_id
             WHERE n.is_archived = 0 AND n.completed_at IS NULL
             ORDER BY t.name",
        )?;
        let tags = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(NavSummary {
            inbox: u32_from_i64("inbox", inbox)?,
            needs_review: u32_from_i64("needs_review", needs_review)?,
            followups: u32_from_i64("followups", followups)?,
            parse_failed: u32_from_i64("parse_failed", parse_failed)?,
            parse_queue: u32_from_i64("parse_queue", parse_queue)?,
            tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::domain::{ActionStatus, ParseStatus, TagKind};
    use crate::repositories::{ActionItemRepository, NoteRepository, TagRepository};

    use super::NavSummaryRepository;

    #[test]
    fn nav_summary_counts_the_whole_workspace_without_loading_rows() {
        let db = Database::in_memory().unwrap();
        let notes = NoteRepository::new(db.clone());
        let tags = TagRepository::new(db.clone());
        let actions = ActionItemRepository::new(db.clone());
        let summary = NavSummaryRepository::new(db);

        let parsed = notes.create_raw_note("parsed").unwrap();
        notes
            .set_parse_status(parsed.id, ParseStatus::Parsed)
            .unwrap();
        let failed = notes.create_raw_note("failed").unwrap();
        notes
            .set_parse_status(failed.id, ParseStatus::Failed)
            .unwrap();
        notes.create_raw_note("queued").unwrap();
        let parsing = notes.create_raw_note("parsing").unwrap();
        notes
            .set_parse_status(parsing.id, ParseStatus::Parsing)
            .unwrap();

        let archived = notes.create_raw_note("archived").unwrap();
        notes
            .set_parse_status(archived.id, ParseStatus::Failed)
            .unwrap();
        notes.archive(archived.id).unwrap();
        let completed = notes.create_raw_note("completed").unwrap();
        notes.complete(completed.id, None).unwrap();

        let finance = tags.upsert("Finance", TagKind::Topic).unwrap();
        let hidden = tags.upsert("Hidden", TagKind::Topic).unwrap();
        let closed = tags.upsert("Closed", TagKind::Topic).unwrap();
        let finance_person = tags.upsert("Finance", TagKind::Person).unwrap();
        tags.apply_to_note(parsed.id, finance.id, "parser", None)
            .unwrap();
        tags.apply_to_note(failed.id, finance_person.id, "parser", None)
            .unwrap();
        tags.apply_to_note(archived.id, hidden.id, "parser", None)
            .unwrap();
        tags.apply_to_note(completed.id, closed.id, "parser", None)
            .unwrap();

        for note_id in [parsed.id, archived.id, completed.id] {
            actions
                .create_suggested(note_id, "review", None, None, None)
                .unwrap();
            for status in [
                ActionStatus::Accepted,
                ActionStatus::Done,
                ActionStatus::Dismissed,
            ] {
                actions
                    .create(note_id, "tracked", None, None, status, "parser", None)
                    .unwrap();
            }
        }

        let counts = summary.load().unwrap();

        // Inbox counts and tags cover active notes: not archived, not completed.
        assert_eq!(counts.inbox, 4);
        assert_eq!(counts.parse_failed, 1);
        assert_eq!(counts.parse_queue, 2);
        assert_eq!(counts.tags, vec!["Finance".to_string()]);
        // Action counts match the review and follow-up lists, which exclude
        // archived notes but keep actions on completed notes.
        assert_eq!(counts.needs_review, 2);
        assert_eq!(counts.followups, 2);
    }

    #[test]
    fn nav_summary_of_an_empty_workspace_is_all_zero() {
        let summary = NavSummaryRepository::new(Database::in_memory().unwrap())
            .load()
            .unwrap();

        assert_eq!(summary.inbox, 0);
        assert_eq!(summary.needs_review, 0);
        assert_eq!(summary.followups, 0);
        assert_eq!(summary.parse_failed, 0);
        assert_eq!(summary.parse_queue, 0);
        assert!(summary.tags.is_empty());
    }
}
