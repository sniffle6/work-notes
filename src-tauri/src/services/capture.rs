use crate::app_state::AppRepositories;
use crate::domain::Note;

use super::{ServiceError, ServiceResult};

#[derive(Clone)]
pub struct CaptureService {
    repositories: AppRepositories,
}

impl CaptureService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn capture(&self, raw_text: &str) -> ServiceResult<Note> {
        if raw_text.trim().is_empty() {
            return Err(ServiceError::InvalidInput("note text is required"));
        }

        let note = self.repositories.notes.create_raw_note(raw_text)?;
        self.repositories.parse_jobs.enqueue(note.id)?;
        Ok(note)
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::domain::ParseStatus;
    use crate::services::capture::CaptureService;

    fn test_repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn capture_service_saves_raw_note_and_enqueues_parse() {
        let repositories = test_repositories();
        let service = CaptureService::new(repositories.clone());

        let note = service
            .capture("Mike said deploy moved to Friday")
            .expect("capture should save raw note");

        assert_eq!(note.raw_text, "Mike said deploy moved to Friday");
        assert_eq!(note.parse_status, ParseStatus::Queued);
        assert!(repositories.parse_jobs.next_queued().unwrap().is_some());
    }
}
