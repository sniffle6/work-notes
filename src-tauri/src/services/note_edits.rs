use crate::app_state::AppRepositories;
use crate::domain::{NoteId, ParseJob};

use super::{ServiceError, ServiceResult};

#[derive(Clone)]
pub struct NoteEditService {
    repositories: AppRepositories,
}

impl NoteEditService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn update_raw_text(&self, note_id: NoteId, raw_text: &str) -> ServiceResult<ParseJob> {
        if raw_text.trim().is_empty() {
            return Err(ServiceError::InvalidInput("note text is required"));
        }

        self.repositories
            .notes
            .update_raw_text_by_user(note_id, raw_text)?;
        self.repositories
            .tags
            .remove_parser_assignments_for_note(note_id)?;
        self.repositories
            .action_items
            .delete_replaceable_parser_suggestions(note_id)?;
        self.repositories
            .parse_jobs
            .enqueue_after_raw_text_update(note_id)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::domain::{ActionStatus, ParseStatus, TagKind};
    use crate::services::note_edits::NoteEditService;

    fn test_repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn updating_raw_text_invalidates_parser_output_and_queues_reparse() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("Jordan said export totals mismatch")
            .unwrap();
        let first_job = repositories
            .parse_jobs
            .claim_next_queued()
            .unwrap()
            .unwrap();
        repositories.parse_jobs.mark_parsed(first_job.id).unwrap();
        repositories
            .notes
            .apply_cleaned_note(
                note.id,
                "Pricing export mismatch",
                "Finance sees different CSV totals.",
                "Check the pricing export.",
            )
            .unwrap();
        let tag = repositories.tags.upsert("Finance", TagKind::Topic).unwrap();
        repositories
            .tags
            .apply_to_note(note.id, tag.id, "parser", Some(0.9))
            .unwrap();
        let suggested = repositories
            .action_items
            .create_suggested(note.id, "Check the pricing export", None, None, Some(0.8))
            .unwrap();
        let accepted = repositories
            .action_items
            .create(
                note.id,
                "Send the export sample",
                None,
                None,
                ActionStatus::Accepted,
                "parser",
                Some(0.7),
            )
            .unwrap();

        NoteEditService::new(repositories.clone())
            .update_raw_text(note.id, "Jordan said pricing export is fixed")
            .unwrap();

        let stored = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.raw_text, "Jordan said pricing export is fixed");
        assert_eq!(stored.parse_status, ParseStatus::Queued);
        assert_eq!(stored.cleaned_text, None);
        assert_eq!(stored.summary, None);
        assert!(repositories.tags.list_for_note(note.id).unwrap().is_empty());

        let actions = repositories.action_items.list_for_note(note.id).unwrap();
        assert!(!actions.iter().any(|action| action.id == suggested.id));
        assert!(actions.iter().any(|action| action.id == accepted.id));
        assert_eq!(
            repositories
                .parse_jobs
                .next_queued()
                .unwrap()
                .unwrap()
                .note_id,
            note.id
        );
    }

    #[test]
    fn updating_raw_text_queues_followup_reparse_when_existing_job_is_parsing() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("Maya said rollout moved")
            .unwrap();
        let parsing_job = repositories
            .parse_jobs
            .claim_next_queued()
            .unwrap()
            .unwrap();

        NoteEditService::new(repositories.clone())
            .update_raw_text(note.id, "Maya said rollout moved to Friday")
            .unwrap();

        let queued_job = repositories.parse_jobs.next_queued().unwrap().unwrap();
        assert_eq!(queued_job.note_id, note.id);
        assert_ne!(queued_job.id, parsing_job.id);
    }
}
