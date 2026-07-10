use crate::app_state::AppRepositories;
use crate::domain::{
    ActionItem, ActionItemId, ActionReviewItem, ActionStatus, FollowupItem, FollowupState, NoteId,
    ReviewStatus,
};

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

    pub fn list_followups(&self, limit: u32) -> ServiceResult<Vec<FollowupItem>> {
        self.repositories
            .action_items
            .list_followups(limit)
            .map_err(Into::into)
    }

    pub fn create_manual_followup(
        &self,
        note_id: NoteId,
        text: &str,
        lane_override: Option<&str>,
    ) -> ServiceResult<ActionItem> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ServiceError::InvalidInput("follow-up text is required"));
        }

        self.repositories
            .notes
            .get(note_id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "note",
                id: note_id.to_string(),
            })?;

        let lane_override = Self::normalize_followup_lane(lane_override)?;
        self.repositories
            .action_items
            .create_manual_followup(note_id, text, lane_override)
            .map_err(Into::into)
    }

    pub fn update_followup_state(
        &self,
        id: ActionItemId,
        state: FollowupState,
    ) -> ServiceResult<()> {
        let action = self.require_action(id)?;
        if action.status != ActionStatus::Accepted {
            return Err(ServiceError::InvalidInput(
                "action is not an active follow-up",
            ));
        }

        self.repositories
            .action_items
            .set_followup_state(id, state)?;
        Ok(())
    }

    pub fn update_followup_lane(
        &self,
        id: ActionItemId,
        lane_override: Option<&str>,
    ) -> ServiceResult<()> {
        let action = self.require_action(id)?;
        if !matches!(action.status, ActionStatus::Accepted | ActionStatus::Done) {
            return Err(ServiceError::InvalidInput(
                "action is not an active follow-up",
            ));
        }

        let lane_override = Self::normalize_followup_lane(lane_override)?;
        self.repositories
            .action_items
            .set_followup_lane(id, lane_override)?;
        Ok(())
    }

    fn transition(
        &self,
        id: ActionItemId,
        expected: ActionStatus,
        next: ActionStatus,
        resolves_suggestion: bool,
    ) -> ServiceResult<()> {
        let action =
            self.repositories
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

    fn require_action(&self, id: ActionItemId) -> ServiceResult<ActionItem> {
        self.repositories
            .action_items
            .get(id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "action_item",
                id: id.to_string(),
            })
    }

    fn normalize_followup_lane(lane_override: Option<&str>) -> ServiceResult<Option<&str>> {
        lane_override
            .map(|lane| {
                let lane = lane.trim();
                if lane.is_empty() {
                    Err(ServiceError::InvalidInput("follow-up lane cannot be empty"))
                } else {
                    Ok(lane)
                }
            })
            .transpose()
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
    use crate::domain::{ActionStatus, FollowupState, NoteId, ReviewStatus};

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
            .create_suggested(
                note.id,
                "Review the dashboard",
                Some("Maya"),
                None,
                Some(0.9),
            )
            .unwrap();

        ActionItemService::new(repositories.clone())
            .accept(action.id)
            .unwrap();

        let stored_action = repositories.action_items.get(action.id).unwrap().unwrap();
        let stored_note = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored_action.status, ActionStatus::Accepted);
        assert_eq!(stored_note.review_status, ReviewStatus::Reviewed);
    }

    #[test]
    fn accepting_suggested_action_makes_open_followup() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Ask Maya to review the dashboard")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(
                note.id,
                "Review the dashboard",
                Some("Maya"),
                None,
                Some(0.9),
            )
            .unwrap();

        ActionItemService::new(repositories.clone())
            .accept(action.id)
            .unwrap();

        let stored_action = repositories.action_items.get(action.id).unwrap().unwrap();
        assert_eq!(stored_action.status, ActionStatus::Accepted);
        assert_eq!(stored_action.followup_state, Some(FollowupState::Open));
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
        let completed = repositories.action_items.get(action.id).unwrap().unwrap();
        assert_eq!(completed.status, ActionStatus::Done);
        assert!(completed.completed_at.is_some());
        assert_eq!(
            repositories
                .notes
                .get(note.id)
                .unwrap()
                .unwrap()
                .review_status,
            ReviewStatus::Reviewed
        );

        service.reopen(action.id).unwrap();
        let reopened = repositories.action_items.get(action.id).unwrap().unwrap();
        assert_eq!(reopened.status, ActionStatus::Accepted);
        assert_eq!(reopened.completed_at, None);
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
            .create_suggested(
                note.id,
                "Check badge printer",
                Some("Maya"),
                None,
                Some(0.77),
            )
            .unwrap();

        let items = ActionItemService::new(repositories)
            .list_suggested(100)
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, action.id);
        assert_eq!(items[0].note_title, note.title);
        assert_eq!(items[0].owner.as_deref(), Some("Maya"));
    }

    #[test]
    fn creates_manual_followup_from_note() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Bring kiosk plan into Friday sync")
            .unwrap();

        let action = ActionItemService::new(repositories.clone())
            .create_manual_followup(
                note.id,
                "  Ask Rina for the latest kiosk count  ",
                Some(" Ops "),
            )
            .unwrap();

        assert_eq!(action.note_id, note.id);
        assert_eq!(action.text, "Ask Rina for the latest kiosk count");
        assert_eq!(action.status, ActionStatus::Accepted);
        assert_eq!(action.source, "user");
        assert_eq!(action.followup_state, Some(FollowupState::Open));
        assert_eq!(action.followup_lane.as_deref(), Some("Ops"));

        let followups = ActionItemService::new(repositories)
            .list_followups(100)
            .unwrap();
        assert_eq!(followups.len(), 1);
        assert_eq!(followups[0].id, action.id);
    }

    #[test]
    fn rejects_empty_manual_followup_text() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("No blank follow-up")
            .unwrap();

        let error = ActionItemService::new(repositories)
            .create_manual_followup(note.id, "   ", None)
            .unwrap_err();

        assert!(matches!(
            error,
            ServiceError::InvalidInput("follow-up text is required")
        ));
    }

    #[test]
    fn rejects_manual_followup_for_missing_note() {
        let repositories = repositories();
        let note_id = NoteId::new();

        let error = ActionItemService::new(repositories)
            .create_manual_followup(note_id, "Ask Rina for a count", None)
            .unwrap_err();

        assert!(matches!(
            error,
            ServiceError::NotFound { entity: "note", id } if id == note_id.to_string()
        ));
    }

    #[test]
    fn updates_followup_state_and_lane() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Follow up with facilities")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Ask about badge printer", None, None, Some(0.8))
            .unwrap();
        let service = ActionItemService::new(repositories.clone());
        service.accept(action.id).unwrap();

        service
            .update_followup_state(action.id, FollowupState::Waiting)
            .unwrap();
        service
            .update_followup_lane(action.id, Some(" Facilities "))
            .unwrap();

        let stored_action = repositories.action_items.get(action.id).unwrap().unwrap();
        assert_eq!(stored_action.followup_state, Some(FollowupState::Waiting));
        assert_eq!(stored_action.followup_lane.as_deref(), Some("Facilities"));

        service.update_followup_lane(action.id, None).unwrap();
        let cleared_lane = repositories
            .action_items
            .get(action.id)
            .unwrap()
            .unwrap()
            .followup_lane;
        assert_eq!(cleared_lane, None);

        service.complete(action.id).unwrap();
        service
            .update_followup_lane(action.id, Some("Completed"))
            .unwrap();
        let completed_action = repositories.action_items.get(action.id).unwrap().unwrap();
        assert_eq!(completed_action.status, ActionStatus::Done);
        assert_eq!(completed_action.followup_lane.as_deref(), Some("Completed"));
    }

    #[test]
    fn rejects_followup_state_change_for_suggested_action() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Suggested is not active yet")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Wait for acceptance", None, None, Some(0.8))
            .unwrap();

        let error = ActionItemService::new(repositories)
            .update_followup_state(action.id, FollowupState::Blocked)
            .unwrap_err();

        assert!(matches!(
            error,
            ServiceError::InvalidInput("action is not an active follow-up")
        ));
    }

    #[test]
    fn rejects_empty_followup_lane() {
        let repositories = repositories();
        let note = repositories
            .notes
            .create_raw_note("Lane cannot be blank")
            .unwrap();
        let action = repositories
            .action_items
            .create_suggested(note.id, "Check a lane value", None, None, Some(0.8))
            .unwrap();
        let service = ActionItemService::new(repositories.clone());
        service.accept(action.id).unwrap();

        let error = service
            .update_followup_lane(action.id, Some("   "))
            .unwrap_err();

        assert!(matches!(
            error,
            ServiceError::InvalidInput("follow-up lane cannot be empty")
        ));
    }
}
