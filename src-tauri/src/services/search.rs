use crate::app_state::AppRepositories;
use crate::domain::{ActionItem, InboxFilters, Note, NoteId, NoteListItem, TagAssignment};

use super::{ServiceError, ServiceResult};

#[derive(Debug, Clone, PartialEq)]
pub struct NoteDetail {
    pub note: Note,
    pub tags: Vec<TagAssignment>,
    pub action_items: Vec<ActionItem>,
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

        Ok(NoteDetail {
            note,
            tags,
            action_items,
        })
    }
}
