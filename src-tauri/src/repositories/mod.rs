pub mod actions;
pub mod notes;
pub mod parse_jobs;
pub mod tags;

use chrono::{DateTime, SecondsFormat, Utc};

pub use actions::ActionItemRepository;
pub use notes::NoteRepository;
pub use parse_jobs::ParseJobRepository;
pub use tags::TagRepository;

use crate::db::DatabaseError;
use crate::domain::DomainError;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("invalid database value for {field}: `{value}`")]
    InvalidDatabaseValue { field: &'static str, value: String },
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

pub(crate) fn now_db_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn parse_db_datetime(
    field: &'static str,
    value: String,
) -> RepositoryResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|_| RepositoryError::InvalidDatabaseValue { field, value })
}

pub(crate) fn optional_db_datetime(
    field: &'static str,
    value: Option<String>,
) -> RepositoryResult<Option<DateTime<Utc>>> {
    value
        .map(|value| parse_db_datetime(field, value))
        .transpose()
}

pub(crate) fn u32_from_i64(field: &'static str, value: i64) -> RepositoryResult<u32> {
    u32::try_from(value).map_err(|_| RepositoryError::InvalidDatabaseValue {
        field,
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::domain::{ActionStatus, ParseStatus, ReviewStatus, TagKind};

    use super::{ActionItemRepository, NoteRepository, ParseJobRepository, TagRepository};

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn save_note_creates_raw_note_and_parse_job() {
        let db = test_db();
        let notes = NoteRepository::new(db.clone());
        let jobs = ParseJobRepository::new(db.clone());

        let note = notes
            .create_raw_note("Mike says deploy moved to Friday")
            .unwrap();

        let stored = notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.raw_text, "Mike says deploy moved to Friday");
        assert_eq!(stored.parse_status, ParseStatus::Queued);
        assert_eq!(stored.review_status, ReviewStatus::None);
        assert_eq!(jobs.next_queued().unwrap().unwrap().note_id, note.id);
    }

    #[test]
    fn enqueue_reuses_existing_active_parse_job_for_note() {
        let db = test_db();
        let notes = NoteRepository::new(db.clone());
        let jobs = ParseJobRepository::new(db.clone());

        let note = notes.create_raw_note("Only parse once").unwrap();
        let first = jobs.enqueue(note.id).unwrap();
        let second = jobs.enqueue(note.id).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(jobs.claim_next_queued().unwrap().unwrap().id, first.id);
        assert!(jobs.next_queued().unwrap().is_none());
    }

    #[test]
    fn fts_search_matches_raw_and_cleaned_text() {
        let db = test_db();
        let notes = NoteRepository::new(db);

        let note = notes.create_raw_note("cfg flag before qa").unwrap();
        notes
            .apply_cleaned_text(
                note.id,
                "Check the config flag before QA gets the build.",
                "Verify config flag.",
            )
            .unwrap();

        let raw_results = notes.search("cfg").unwrap();
        assert_eq!(raw_results.len(), 1);
        assert_eq!(raw_results[0].id, note.id);

        let cleaned_results = notes.search("config").unwrap();
        assert_eq!(cleaned_results.len(), 1);
        assert_eq!(cleaned_results[0].id, note.id);
    }

    #[test]
    fn parse_job_claim_marks_job_and_note_parsing() {
        let db = test_db();
        let notes = NoteRepository::new(db.clone());
        let jobs = ParseJobRepository::new(db.clone());

        let note = notes.create_raw_note("Parse this next").unwrap();
        jobs.enqueue(note.id).unwrap();

        let claimed = jobs.claim_next_queued().unwrap().unwrap();
        assert_eq!(claimed.note_id, note.id);
        assert_eq!(claimed.status, ParseStatus::Parsing);
        assert!(claimed.started_at.is_some());

        let stored = notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.parse_status, ParseStatus::Parsing);
        assert!(jobs.next_queued().unwrap().is_none());
    }

    #[test]
    fn tags_and_action_items_round_trip_for_note() {
        let db = test_db();
        let notes = NoteRepository::new(db.clone());
        let tags = TagRepository::new(db.clone());
        let actions = ActionItemRepository::new(db);

        let note = notes
            .create_raw_note("Ask Alice to review payroll")
            .unwrap();
        let tag = tags.upsert("Alice", TagKind::Person).unwrap();
        tags.apply_to_note(note.id, tag.id, "parser", Some(0.91))
            .unwrap();

        let action = actions
            .create_suggested(
                note.id,
                "Ask Alice to review payroll",
                Some("Alice"),
                None,
                Some(0.86),
            )
            .unwrap();
        actions
            .set_status(action.id, ActionStatus::Accepted)
            .unwrap();

        let note_tags = tags.list_for_note(note.id).unwrap();
        assert_eq!(note_tags.len(), 1);
        assert_eq!(note_tags[0].tag.name, "Alice");
        assert_eq!(note_tags[0].confidence, Some(0.91));

        let note_actions = actions.list_for_note(note.id).unwrap();
        assert_eq!(note_actions.len(), 1);
        assert_eq!(note_actions[0].status, ActionStatus::Accepted);
    }
}
