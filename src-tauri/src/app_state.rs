use crate::db::{Database, DatabaseError};
use crate::repositories::{
    ActionItemRepository, CardNoteRepository, JiraChatRepository, NoteJiraLinkRepository,
    NoteRepository, ParseJobRepository, TagRepository,
};
use crate::services::draft::DraftService;
use crate::services::jira_agent::AgentBusyThreads;
use crate::services::parse_queue::{ParseQueueConfig, ParserProviderConfig};
use crate::services::settings::{AppSettings, SettingsService};

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub repositories: AppRepositories,
    pub settings: SettingsService,
    pub drafts: DraftService,
    pub parse_queue_config: ParseQueueConfig,
    pub parser_provider_config: ParserProviderConfig,
    pub jira_agent_busy: AgentBusyThreads,
}

impl AppState {
    pub fn new(database: Database) -> Self {
        let settings = SettingsService::new(database.clone());
        let app_settings = settings.get().unwrap_or_default();
        Self::with_settings(database, settings, app_settings)
    }

    fn with_settings(
        database: Database,
        settings: SettingsService,
        app_settings: AppSettings,
    ) -> Self {
        Self {
            repositories: AppRepositories::new(database.clone()),
            settings,
            drafts: DraftService::default(),
            parse_queue_config: ParseQueueConfig::from_settings(&app_settings),
            parser_provider_config: ParserProviderConfig::from_settings(&app_settings),
            jira_agent_busy: AgentBusyThreads::default(),
            database,
        }
    }

    pub fn in_memory() -> Result<Self, DatabaseError> {
        Database::in_memory().map(Self::new)
    }
}

#[derive(Clone)]
pub struct AppRepositories {
    pub database: Database,
    pub notes: NoteRepository,
    pub tags: TagRepository,
    pub action_items: ActionItemRepository,
    pub card_notes: CardNoteRepository,
    pub parse_jobs: ParseJobRepository,
    pub jira_chat: JiraChatRepository,
    pub jira_links: NoteJiraLinkRepository,
}

impl AppRepositories {
    pub fn new(database: Database) -> Self {
        Self {
            database: database.clone(),
            notes: NoteRepository::new(database.clone()),
            tags: TagRepository::new(database.clone()),
            action_items: ActionItemRepository::new(database.clone()),
            card_notes: CardNoteRepository::new(database.clone()),
            jira_chat: JiraChatRepository::new(database.clone()),
            jira_links: NoteJiraLinkRepository::new(database.clone()),
            parse_jobs: ParseJobRepository::new(database),
        }
    }
}
