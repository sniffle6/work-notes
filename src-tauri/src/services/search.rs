use crate::app_state::AppRepositories;
use crate::domain::{
    ActionItem, CardNote, InboxFilters, Note, NoteId, NoteListItem, ParseStatus, TagAssignment,
};

use super::{ServiceError, ServiceResult};

#[derive(Debug, Clone, PartialEq)]
pub struct NoteDetail {
    pub note: Note,
    pub tags: Vec<TagAssignment>,
    pub action_items: Vec<ActionItem>,
    pub card_notes: Vec<CardNote>,
    pub parse_error: Option<String>,
}

#[derive(Clone)]
pub struct SearchService {
    repositories: AppRepositories,
}

impl SearchService {
    pub fn new(repositories: AppRepositories) -> Self {
        Self { repositories }
    }

    pub fn list_inbox(&self, filters: InboxFilters) -> ServiceResult<Vec<NoteListItem>> {
        self.repositories
            .notes
            .list_inbox(filters)
            .map_err(Into::into)
    }

    pub fn search(&self, query: &str) -> ServiceResult<Vec<NoteListItem>> {
        self.repositories.notes.search(query).map_err(Into::into)
    }

    pub fn get_note(&self, id: NoteId) -> ServiceResult<NoteDetail> {
        let note = self
            .repositories
            .notes
            .get(id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "note",
                id: id.to_string(),
            })?;
        let tags = self.repositories.tags.list_for_note(id)?;
        let action_items = self.repositories.action_items.list_for_note(id)?;
        let card_notes = self.repositories.card_notes.list_for_note(id)?;
        let parse_error = if note.parse_status == ParseStatus::Failed {
            self.repositories.parse_jobs.latest_error_for_note(id)?
        } else {
            None
        };

        Ok(NoteDetail {
            note,
            tags,
            action_items,
            card_notes,
            parse_error,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;

    use super::SearchService;

    fn test_repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn get_note_includes_latest_parse_failure_error() {
        let repositories = test_repositories();
        let note = repositories.notes.create_raw_note("raw note").unwrap();
        let job = repositories
            .parse_jobs
            .claim_next_queued()
            .unwrap()
            .unwrap();
        repositories
            .parse_jobs
            .mark_failed(job.id, "codex command timed out after 30s")
            .unwrap();

        let detail = SearchService::new(repositories).get_note(note.id).unwrap();

        assert_eq!(
            detail.parse_error.as_deref(),
            Some("codex command timed out after 30s")
        );
    }
}
