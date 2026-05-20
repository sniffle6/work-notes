use crate::db::{Database, DatabaseError};
use crate::repositories::{
    ActionItemRepository, NoteRepository, ParseJobRepository, TagRepository,
};
use crate::services::draft::DraftService;
use crate::services::parse_queue::{ParseQueueConfig, ParserProviderConfig};
use crate::services::settings::SettingsService;

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub repositories: AppRepositories,
    pub settings: SettingsService,
    pub drafts: DraftService,
    pub parse_queue_config: ParseQueueConfig,
    pub parser_provider_config: ParserProviderConfig,
}

impl AppState {
    pub fn new(database: Database) -> Self {
        Self {
            repositories: AppRepositories::new(database.clone()),
            settings: SettingsService::new(database.clone()),
            drafts: DraftService::default(),
            parse_queue_config: ParseQueueConfig::default(),
            parser_provider_config: ParserProviderConfig::default(),
            database,
        }
    }

    pub fn in_memory() -> Result<Self, DatabaseError> {
        Database::in_memory().map(Self::new)
    }
}

#[derive(Clone)]
pub struct AppRepositories {
    pub notes: NoteRepository,
    pub tags: TagRepository,
    pub action_items: ActionItemRepository,
    pub parse_jobs: ParseJobRepository,
}

impl AppRepositories {
    pub fn new(database: Database) -> Self {
        Self {
            notes: NoteRepository::new(database.clone()),
            tags: TagRepository::new(database.clone()),
            action_items: ActionItemRepository::new(database.clone()),
            parse_jobs: ParseJobRepository::new(database),
        }
    }
}
