use crate::app_state::AppRepositories;
use crate::domain::{NoteId, ParseJob, ParseJobId};

use super::ServiceResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseQueueConfig {
    pub max_attempts: u32,
    pub idle_sleep_ms: u64,
}

impl Default for ParseQueueConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            idle_sleep_ms: 250,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserProviderConfig {
    pub provider_name: String,
    pub prompt_version: String,
    pub codex_command_path: String,
    pub schema_path: String,
    pub timeout_seconds: u64,
}

impl Default for ParserProviderConfig {
    fn default() -> Self {
        Self {
            provider_name: "codex".to_string(),
            prompt_version: "parse-note-v1".to_string(),
            codex_command_path: "codex".to_string(),
            schema_path: "schemas/parse-note.schema.json".to_string(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Clone)]
pub struct ParseQueue {
    repositories: AppRepositories,
    pub config: ParseQueueConfig,
    pub parser_provider_config: ParserProviderConfig,
}

impl ParseQueue {
    pub fn new(repositories: AppRepositories) -> Self {
        Self::with_config(
            repositories,
            ParseQueueConfig::default(),
            ParserProviderConfig::default(),
        )
    }

    pub fn with_config(
        repositories: AppRepositories,
        config: ParseQueueConfig,
        parser_provider_config: ParserProviderConfig,
    ) -> Self {
        Self {
            repositories,
            config,
            parser_provider_config,
        }
    }

    pub fn enqueue_note(&self, note_id: NoteId) -> ServiceResult<ParseJob> {
        self.repositories
            .parse_jobs
            .enqueue(note_id)
            .map_err(Into::into)
    }

    pub fn retry_note(&self, note_id: NoteId) -> ServiceResult<ParseJob> {
        self.enqueue_note(note_id)
    }

    pub fn claim_next(&self) -> ServiceResult<Option<ParseJob>> {
        self.repositories
            .parse_jobs
            .claim_next_queued()
            .map_err(Into::into)
    }

    pub fn mark_parsed(&self, job_id: ParseJobId) -> ServiceResult<()> {
        self.repositories
            .parse_jobs
            .mark_parsed(job_id)
            .map_err(Into::into)
    }

    pub fn mark_failed(&self, job_id: ParseJobId, error: &str) -> ServiceResult<()> {
        self.repositories
            .parse_jobs
            .mark_failed(job_id, error)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::domain::ParseStatus;
    use crate::services::parse_queue::ParseQueue;

    fn test_repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn parse_queue_marks_failed_job_without_modifying_raw_note() {
        let repositories = test_repositories();
        let note = repositories.notes.create_raw_note("raw note").unwrap();
        let job = repositories.parse_jobs.enqueue(note.id).unwrap();

        ParseQueue::new(repositories.clone())
            .mark_failed(job.id, "invalid json")
            .expect("parse job should be marked failed");

        let stored = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.raw_text, "raw note");
        assert_eq!(stored.parse_status, ParseStatus::Failed);
    }
}
