use crate::app_state::AppRepositories;
use crate::domain::{ActionItemId, ActionReviewItem, ActionStatus, NoteId, ReviewStatus};

use super::{ServiceError, ServiceResult};

#[derive(Clone)]
pub struct ActionItemService {
    repositories: AppRepositories,
}

impl ActionItemService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn accept(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Suggested, ActionStatus::Accepted, true)
    }

    pub fn dismiss(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Suggested, ActionStatus::Dismissed, true)
    }

    pub fn complete(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Accepted, ActionStatus::Done, false)
    }

    pub fn reopen(&self, id: ActionItemId) -> ServiceResult<()> {
        self.transition(id, ActionStatus::Done, ActionStatus::Accepted, false)
    }

    pub fn list_suggested(&self, limit: u32) -> ServiceResult<Vec<ActionReviewItem>> {
        self.repositories
            .action_items
            .list_suggested_with_note_context(limit)
            .map_err(Into::into)
    }

    fn transition(
        &self,
        id: ActionItemId,
        expected: ActionStatus,
        next: ActionStatus,
        resolves_suggestion: bool,
    ) -> ServiceResult<()> {
        let action = self
            .repositories
            .action_items
            .get(id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            })?;

        if action.status != expected {
            return Err(ServiceError::InvalidInput(
                "invalid action status transition",
            ));
        }

        self.repositories.action_items.set_status(id, next)?;

        if resolves_suggestion {
            self.refresh_note_review_status(action.note_id)?;
        }

        Ok(())
    }

    fn refresh_note_review_status(&self, note_id: NoteId) -> ServiceResult<()> {
        if !self
            .repositories
            .action_items
            .has_suggested_for_note(note_id)?
        {
            self.repositories
                .notes
                .set_review_status(note_id, ReviewStatus::Reviewed)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::domain::{ActionStatus, ReviewStatus};

    use super::{ActionItemService, ServiceError};

    fn repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn accepting_last_suggested_action_marks_note_reviewed() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Ask Maya to review the dashboard")
            .unwrap();
        repositories
            .notes
            .set_review_status(note.id, ReviewStatus::NeedsReview)
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Review the dashboard", Some("Maya"), None, Some(0.9))
            .unwrap();

        ActionItemService::new(repositories.clone())
            .accept(action.id)
            .unwrap();

        let stored_action = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap();
        let stored_note = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored_action.status, ActionStatus::Accepted);
        assert_eq!(stored_note.review_status, ReviewStatus::Reviewed);
    }

    #[test]
    fn dismissing_one_of_multiple_suggested_actions_keeps_note_needing_review() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Two follow ups for Jordan")
            .unwrap();
        repositories
            .notes
            .set_review_status(note.id, ReviewStatus::NeedsReview)
            .unwrap();
        let first = repositories
            .action_items
            .create_suggested(note.id, "Check the export", Some("Jordan"), None, Some(0.8))
            .unwrap();
        repositories
            .action_items
            .create_suggested(note.id, "Send the filtered query", None, None, Some(0.7))
            .unwrap();

        ActionItemService::new(repositories.clone())
            .dismiss(first.id)
            .unwrap();

        let stored_note = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored_note.review_status, ReviewStatus::NeedsReview);
    }

    #[test]
    fn completing_and_reopening_actions_preserves_review_status() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Follow up after accepting action")
            .unwrap();
        repositories
            .notes
            .set_review_status(note.id, ReviewStatus::Reviewed)
            .unwrap();
        let action = repositories
            .action_items
            .create(
                note.id,
                "Follow up with Alice",
                Some("Alice"),
                None,
                ActionStatus::Accepted,
                "parser",
                Some(0.8),
            )
            .unwrap();
        let service = ActionItemService::new(repositories.clone());

        service.complete(action.id).unwrap();
        let completed = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, ActionStatus::Done);
        assert_eq!(
            repositories.notes.get(note.id).unwrap().unwrap().review_status,
            ReviewStatus::Reviewed
        );

        service.reopen(action.id).unwrap();
        let reopened = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap();
        assert_eq!(reopened.status, ActionStatus::Accepted);
    }

    #[test]
    fn invalid_lifecycle_transition_returns_invalid_input() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Cannot complete suggested directly")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Review before complete", None, None, Some(0.8))
            .unwrap();

        let error = ActionItemService::new(repositories)
            .complete(action.id)
            .unwrap_err();

        assert!(matches!(
            error,
            ServiceError::InvalidInput("invalid action status transition")
        ));
    }

    #[test]
    fn list_suggested_uses_repository_review_rows() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Maya owns badge printer follow up")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Check badge printer", Some("Maya"), None, Some(0.77))
            .unwrap();

        let items = ActionItemService::new(repositories)
            .list_suggested(100)
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, action.id);
        assert_eq!(items[0].note_title, note.title);
        assert_eq!(items[0].owner.as_deref(), Some("Maya"));
    }
}
