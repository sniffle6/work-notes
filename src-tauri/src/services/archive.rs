use crate::app_state::AppRepositories;
use crate::domain::NoteId;

use super::{ServiceError, ServiceResult};

#[derive(Clone)]
pub struct ArchiveService {
    repositories: AppRepositories,
}

impl ArchiveService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn restore(&self, note_id: NoteId) -> ServiceResult<()> {
        self.repositories.notes.restore(note_id)?;
        Ok(())
    }

    pub fn permanently_delete(&self, note_id: NoteId) -> ServiceResult<()> {
        let note = self
            .repositories
            .notes
            .get(note_id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "note",
                id: note_id.to_string(),
            })?;

        if !note.is_archived {
            return Err(ServiceError::InvalidInput(
                "note must be archived before permanent delete",
            ));
        }

        self.repositories.notes.permanently_delete(note_id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::services::archive::ArchiveService;
    use crate::services::ServiceError;

    fn repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn permanently_delete_rejects_non_archived_notes() {
        let repositories = repositories();
        let service = ArchiveService::new(repositories.clone());
        let note = repositories
            .notes
            .create_raw_note("active note")
            .expect("create note");

        let error = service
            .permanently_delete(note.id)
            .expect_err("active notes cannot be hard deleted");

        assert!(matches!(error, ServiceError::InvalidInput(_)));
        assert!(repositories.notes.get(note.id).expect("get note").is_some());
    }

    #[test]
    fn restore_archived_note_clears_archive_flag() {
        let repositories = repositories();
        let service = ArchiveService::new(repositories.clone());
        let note = repositories
            .notes
            .create_raw_note("archived note")
            .expect("create note");
        repositories.notes.archive(note.id).expect("archive note");

        service.restore(note.id).expect("restore note");

        let restored = repositories
            .notes
            .get(note.id)
            .expect("get note")
            .expect("note exists");
        assert!(!restored.is_archived);
    }
}
