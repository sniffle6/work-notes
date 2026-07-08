use std::{thread, time::Duration};

use crate::app_state::AppRepositories;
use crate::domain::{ActionItemId, ParseRunId, TagId};
use crate::domain::{NoteId, ParseJob, ParseJobId, ReviewStatus, TagKind as DomainTagKind};
use crate::parser::{
    ActionItemApplication, CodexParserProvider, ParserOutput, ParserProvider, ParserResultApplier,
    ParserResultSink, TagApplication, TagKind as ParserTagKind, DEFAULT_CODEX_PROGRAM,
};
use crate::services::settings::{AppSettings, SettingsService, DEFAULT_PARSER_TIMEOUT_SECONDS};
use chrono::Utc;
use rusqlite::{params, Transaction};

use super::ServiceResult;

const INTERRUPTED_PARSE_ERROR: &str = "parser was interrupted before completion; retrying";

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

impl ParseQueueConfig {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            max_attempts: settings.parser_max_retries.max(1),
            ..Self::default()
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
    pub linked_workspace_paths: Vec<String>,
}

impl Default for ParserProviderConfig {
    fn default() -> Self {
        Self {
            provider_name: "codex".to_string(),
            prompt_version: "parse-note-v1".to_string(),
            codex_command_path: DEFAULT_CODEX_PROGRAM.to_string(),
            schema_path: "schemas/parse-note.schema.json".to_string(),
            timeout_seconds: DEFAULT_PARSER_TIMEOUT_SECONDS,
            linked_workspace_paths: Vec::new(),
        }
    }
}

impl ParserProviderConfig {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            codex_command_path: settings.codex_command_path.clone(),
            timeout_seconds: settings.parser_timeout_seconds,
            linked_workspace_paths: settings.linked_workspace_paths.clone(),
            ..Self::default()
        }
    }
}

#[derive(Clone)]
pub struct ParseQueue {
    repositories: AppRepositories,
    settings: Option<SettingsService>,
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
            settings: None,
            config,
            parser_provider_config,
        }
    }

    pub fn with_runtime_settings(
        repositories: AppRepositories,
        settings: SettingsService,
        config: ParseQueueConfig,
        parser_provider_config: ParserProviderConfig,
    ) -> Self {
        Self {
            repositories,
            settings: Some(settings),
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

    pub fn retry_note_with_feedback(
        &self,
        note_id: NoteId,
        feedback: &str,
    ) -> ServiceResult<ParseJob> {
        if feedback.trim().is_empty() {
            return Err(super::ServiceError::InvalidInput(
                "reparse feedback is required",
            ));
        }

        self.repositories
            .parse_jobs
            .enqueue_with_feedback(note_id, Some(feedback))
            .map_err(Into::into)
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

    pub fn requeue_interrupted_jobs(&self) -> ServiceResult<usize> {
        self.repositories
            .parse_jobs
            .requeue_interrupted(INTERRUPTED_PARSE_ERROR)
            .map_err(Into::into)
    }

    pub fn process_next_with_provider<P>(&self, provider: &P) -> ServiceResult<bool>
    where
        P: ParserProvider,
    {
        let Some(job) = self.claim_next()? else {
            return Ok(false);
        };

        self.process_claimed_job(&job, provider)?;
        Ok(true)
    }

    pub fn process_next(&self) -> ServiceResult<bool> {
        let provider_config = self.current_parser_provider_config();
        let provider = CodexParserProvider::new()
            .program(provider_config.codex_command_path)
            .schema_path(provider_config.schema_path)
            .timeout(Duration::from_secs(provider_config.timeout_seconds))
            .linked_workspace_paths(provider_config.linked_workspace_paths);

        let Some(job) = self.claim_next()? else {
            return Ok(false);
        };

        let note = self.repositories.notes.get(job.note_id)?.ok_or_else(|| {
            super::ServiceError::NotFound {
                entity: "note",
                id: job.note_id.to_string(),
            }
        })?;

        match provider.parse_output_with_feedback(&note.raw_text, job.feedback.as_deref()) {
            Ok(output) => self.apply_successful_parse(&job, &output),
            Err(error) => self.handle_parse_failure(&job, &error.to_string()),
        }?;

        Ok(true)
    }

    pub fn start_background_worker(self) -> std::io::Result<()> {
        thread::Builder::new()
            .name("work-notes-parse-worker".to_string())
            .spawn(move || {
                let idle_sleep = Duration::from_millis(self.config.idle_sleep_ms);

                loop {
                    match self.process_next() {
                        Ok(true) => {}
                        Ok(false) => thread::sleep(idle_sleep),
                        Err(error) => {
                            eprintln!("work-notes parse worker error: {error}");
                            thread::sleep(idle_sleep);
                        }
                    }
                }
            })
            .map(|_| ())
    }

    fn process_claimed_job<P>(&self, job: &ParseJob, provider: &P) -> ServiceResult<()>
    where
        P: ParserProvider,
    {
        let note = self.repositories.notes.get(job.note_id)?.ok_or_else(|| {
            super::ServiceError::NotFound {
                entity: "note",
                id: job.note_id.to_string(),
            }
        })?;

        match provider.parse_output(&note.raw_text) {
            Ok(output) => self.apply_successful_parse(job, &output),
            Err(error) => self.handle_parse_failure(job, &error.to_string()),
        }
    }

    fn apply_successful_parse(&self, job: &ParseJob, output: &ParserOutput) -> ServiceResult<()> {
        if let Err(error) = self.apply_successful_parse_transaction(job, output) {
            let _ = self.mark_failed(job.id, &error.to_string());
            return Err(error);
        }
        Ok(())
    }

    fn apply_successful_parse_transaction(
        &self,
        job: &ParseJob,
        output: &ParserOutput,
    ) -> ServiceResult<()> {
        let parsed_json = serde_json::to_string(&output.result)?;
        let provider_config = self.current_parser_provider_config();
        let now = Utc::now().to_rfc3339();
        let mut connection = self.repositories.database.connection()?;
        let transaction = connection.transaction()?;

        record_parse_run(
            &transaction,
            job.note_id,
            &provider_config.provider_name,
            &provider_config.prompt_version,
            &output.raw_response,
            &parsed_json,
            job.feedback.as_deref(),
            &now,
        )?;
        delete_replaceable_parser_suggestions(&transaction, job.note_id)?;
        let mut sink = RepositoryParserResultSink {
            transaction: &transaction,
            updated_at: now.as_str(),
        };
        ParserResultApplier::default().apply(&mut sink, job.note_id, &output.result)?;
        mark_job_parsed(&transaction, job.id, job.note_id, &now)?;
        transaction.commit()?;
        Ok(())
    }

    fn handle_parse_failure(&self, job: &ParseJob, error: &str) -> ServiceResult<()> {
        let config = self.current_queue_config();
        if is_non_retryable_parse_error(error) {
            self.mark_failed(job.id, error)
        } else if job.attempt_count < config.max_attempts {
            self.repositories
                .parse_jobs
                .mark_queued(job.id, error)
                .map_err(Into::into)
        } else {
            self.mark_failed(job.id, error)
        }
    }

    fn current_queue_config(&self) -> ParseQueueConfig {
        self.settings
            .as_ref()
            .and_then(|settings| settings.get().ok())
            .map(|settings| ParseQueueConfig::from_settings(&settings))
            .unwrap_or_else(|| self.config.clone())
    }

    fn current_parser_provider_config(&self) -> ParserProviderConfig {
        self.settings
            .as_ref()
            .and_then(|settings| settings.get().ok())
            .map(|settings| ParserProviderConfig::from_settings(&settings))
            .unwrap_or_else(|| self.parser_provider_config.clone())
    }
}

fn is_non_retryable_parse_error(error: &str) -> bool {
    error.to_ascii_lowercase().contains("timed out")
}

struct RepositoryParserResultSink<'a> {
    transaction: &'a Transaction<'a>,
    updated_at: &'a str,
}

impl ParserResultSink for RepositoryParserResultSink<'_> {
    type Error = super::ServiceError;
    type NoteId = NoteId;

    fn apply_cleaned_text(
        &mut self,
        note_id: Self::NoteId,
        title: &str,
        cleaned_text: &str,
        summary: &str,
    ) -> Result<(), Self::Error> {
        let note_id_text = note_id.to_string();
        let raw_text = self.transaction.query_row(
            "SELECT raw_text FROM notes WHERE id = ?1",
            [note_id_text.as_str()],
            |row| row.get::<_, String>(0),
        )?;
        self.transaction.execute(
            "UPDATE notes
             SET title = ?2, cleaned_text = ?3, summary = ?4, cleaned_edited = 0, updated_at = ?5
             WHERE id = ?1",
            params![
                note_id_text,
                normalize_title(title),
                cleaned_text,
                summary,
                self.updated_at
            ],
        )?;
        replace_fts(
            self.transaction,
            note_id,
            &raw_text,
            Some(cleaned_text),
            Some(summary),
        )?;
        Ok(())
    }

    fn add_tag(
        &mut self,
        note_id: Self::NoteId,
        tag: TagApplication<'_>,
    ) -> Result<(), Self::Error> {
        let kind = to_domain_tag_kind(tag.kind);
        let tag_id = upsert_tag(self.transaction, tag.name, kind)?;
        self.transaction.execute(
            "INSERT INTO note_tags (note_id, tag_id, source, confidence)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(note_id, tag_id, source)
             DO UPDATE SET confidence = excluded.confidence",
            params![
                note_id.to_string(),
                tag_id.to_string(),
                tag.source,
                tag.confidence as f64
            ],
        )?;
        Ok(())
    }

    fn add_action_item(
        &mut self,
        note_id: Self::NoteId,
        action_item: ActionItemApplication<'_>,
    ) -> Result<(), Self::Error> {
        self.transaction.execute(
            "INSERT INTO action_items (
                id, note_id, text, owner, due_date, status, source, confidence
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                ActionItemId::new().to_string(),
                note_id.to_string(),
                action_item.text,
                action_item.owner,
                action_item.due_date,
                action_item.status,
                "parser",
                action_item.confidence as f64
            ],
        )?;
        Ok(())
    }

    fn set_review_status(
        &mut self,
        note_id: Self::NoteId,
        review_status: &'static str,
    ) -> Result<(), Self::Error> {
        self.transaction.execute(
            "UPDATE notes SET review_status = ?2, updated_at = ?3 WHERE id = ?1",
            params![
                note_id.to_string(),
                to_review_status(review_status)?.as_str(),
                self.updated_at
            ],
        )?;
        Ok(())
    }
}

fn normalize_title(title: &str) -> String {
    let title = title.trim();
    let title = if title.is_empty() {
        "Untitled note"
    } else {
        title
    };

    if title.chars().count() > 80 {
        format!("{}...", title.chars().take(77).collect::<String>())
    } else {
        title.to_string()
    }
}

fn record_parse_run(
    transaction: &Transaction<'_>,
    note_id: NoteId,
    provider: &str,
    prompt_version: &str,
    raw_response: &str,
    parsed_json: &str,
    feedback: Option<&str>,
    created_at: &str,
) -> ServiceResult<()> {
    transaction.execute(
        "INSERT INTO parse_runs (
            id, note_id, provider, prompt_version, raw_response, parsed_json, feedback, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            ParseRunId::new().to_string(),
            note_id.to_string(),
            provider,
            prompt_version,
            raw_response,
            parsed_json,
            feedback.map(str::trim).filter(|value| !value.is_empty()),
            created_at
        ],
    )?;
    Ok(())
}

fn delete_replaceable_parser_suggestions(
    transaction: &Transaction<'_>,
    note_id: NoteId,
) -> ServiceResult<()> {
    transaction.execute(
        "DELETE FROM note_tags WHERE note_id = ?1 AND source = ?2",
        params![note_id.to_string(), "ai"],
    )?;
    transaction.execute(
        "DELETE FROM action_items
         WHERE note_id = ?1 AND source = ?2 AND status = ?3",
        params![note_id.to_string(), "parser", "suggested"],
    )?;
    Ok(())
}

fn replace_fts(
    transaction: &Transaction<'_>,
    note_id: NoteId,
    raw_text: &str,
    cleaned_text: Option<&str>,
    summary: Option<&str>,
) -> ServiceResult<()> {
    transaction.execute(
        "DELETE FROM notes_fts WHERE note_id = ?1",
        [note_id.to_string()],
    )?;
    transaction.execute(
        "INSERT INTO notes_fts (note_id, raw_text, cleaned_text, summary)
         VALUES (?1, ?2, ?3, ?4)",
        params![note_id.to_string(), raw_text, cleaned_text, summary],
    )?;
    Ok(())
}

fn upsert_tag(
    transaction: &Transaction<'_>,
    name: &str,
    kind: DomainTagKind,
) -> ServiceResult<TagId> {
    let normalized = name.trim();
    transaction.execute(
        "INSERT INTO tags (id, name, kind, created_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(name, kind) DO NOTHING",
        params![
            TagId::new().to_string(),
            normalized,
            kind.as_str(),
            Utc::now().to_rfc3339()
        ],
    )?;
    let tag_id = transaction.query_row(
        "SELECT id FROM tags WHERE name = ?1 AND kind = ?2",
        params![normalized, kind.as_str()],
        |row| row.get::<_, String>(0),
    )?;
    TagId::parse(&tag_id).map_err(|_| super::ServiceError::InvalidInput("invalid tag id"))
}

fn mark_job_parsed(
    transaction: &Transaction<'_>,
    job_id: ParseJobId,
    note_id: NoteId,
    finished_at: &str,
) -> ServiceResult<()> {
    transaction.execute(
        "UPDATE parse_jobs
         SET status = ?2, last_error = NULL, finished_at = ?3
         WHERE id = ?1",
        params![job_id.to_string(), "parsed", finished_at],
    )?;
    transaction.execute(
        "UPDATE notes SET parse_status = ?2, updated_at = ?3 WHERE id = ?1",
        params![note_id.to_string(), "parsed", finished_at],
    )?;
    Ok(())
}

fn to_domain_tag_kind(kind: &ParserTagKind) -> DomainTagKind {
    match kind {
        ParserTagKind::Person => DomainTagKind::Person,
        ParserTagKind::Project => DomainTagKind::Project,
        ParserTagKind::Topic => DomainTagKind::Topic,
        ParserTagKind::Urgency => DomainTagKind::Urgency,
        ParserTagKind::Category => DomainTagKind::Category,
        ParserTagKind::Custom => DomainTagKind::Custom,
    }
}

fn to_review_status(value: &'static str) -> ServiceResult<ReviewStatus> {
    match value {
        "none" => Ok(ReviewStatus::None),
        "needs_review" => Ok(ReviewStatus::NeedsReview),
        "reviewed" => Ok(ReviewStatus::Reviewed),
        _ => Err(super::ServiceError::InvalidInput("invalid review status")),
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::AppRepositories;
    use crate::db::Database;
    use crate::domain::{ActionStatus, ParseStatus, ReviewStatus, TagKind};
    use crate::parser::{
        ParsedActionItem, ParsedTag, ParserError, ParserOutput, ParserProvider, ParserResult,
        TagKind as ParserTagKind,
    };
    use crate::services::parse_queue::{ParseQueue, ParseQueueConfig, ParserProviderConfig};
    use crate::services::settings::AppSettings;

    fn test_repositories() -> AppRepositories {
        AppRepositories::new(Database::in_memory().unwrap())
    }

    #[test]
    fn parser_provider_config_includes_linked_workspaces_from_settings() {
        let mut settings = AppSettings::default();
        settings.codex_command_path = "codex-test".to_string();
        settings.parser_timeout_seconds = 123;
        settings.linked_workspace_paths = vec![
            "C:\\code\\product".to_string(),
            "D:\\scratch\\other".to_string(),
        ];

        let config = ParserProviderConfig::from_settings(&settings);

        assert_eq!(config.codex_command_path, "codex-test");
        assert_eq!(config.timeout_seconds, 123);
        assert_eq!(
            config.linked_workspace_paths,
            settings.linked_workspace_paths
        );
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

    #[test]
    fn process_next_with_provider_applies_successful_parse() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("sam said fix qa flag")
            .unwrap();
        repositories.parse_jobs.enqueue(note.id).unwrap();

        let processed = ParseQueue::new(repositories.clone())
            .process_next_with_provider(&StaticProvider)
            .unwrap();

        assert!(processed);
        let stored = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.raw_text, "sam said fix qa flag");
        assert_eq!(stored.title, "Fix QA Flag");
        assert_eq!(
            stored.cleaned_text,
            Some("Sam said to fix the QA flag.".to_string())
        );
        assert_eq!(stored.summary, Some("Fix QA flag.".to_string()));
        assert_eq!(stored.parse_status, ParseStatus::Parsed);
        assert_eq!(stored.review_status, ReviewStatus::NeedsReview);

        let tags = repositories.tags.list_for_note(note.id).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag.name, "QA");
        assert_eq!(tags[0].tag.kind, TagKind::Topic);
        assert_eq!(tags[0].source, "ai");

        let actions = repositories.action_items.list_for_note(note.id).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].text, "Fix the QA flag.");
        assert_eq!(actions[0].status, ActionStatus::Suggested);

        assert!(repositories.parse_jobs.next_queued().unwrap().is_none());
    }

    #[test]
    fn process_next_with_provider_marks_parser_failure_without_touching_raw_text() {
        let repositories = test_repositories();
        let note = repositories.notes.create_raw_note("raw note").unwrap();
        repositories.parse_jobs.enqueue(note.id).unwrap();

        let queue = ParseQueue::with_config(
            repositories.clone(),
            ParseQueueConfig {
                max_attempts: 1,
                ..ParseQueueConfig::default()
            },
            ParserProviderConfig::default(),
        );
        let processed = queue.process_next_with_provider(&FailingProvider).unwrap();

        assert!(processed);
        let stored = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.raw_text, "raw note");
        assert_eq!(stored.cleaned_text, None);
        assert_eq!(stored.parse_status, ParseStatus::Failed);
    }

    #[test]
    fn process_next_with_provider_marks_timeout_failed_without_requeueing() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("@codex: inspect linked context before cleaning\n\nraw note")
            .unwrap();

        let queue = ParseQueue::with_config(
            repositories.clone(),
            ParseQueueConfig {
                max_attempts: 3,
                ..ParseQueueConfig::default()
            },
            ParserProviderConfig::default(),
        );
        let processed = queue.process_next_with_provider(&TimeoutProvider).unwrap();

        assert!(processed);
        let stored = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.parse_status, ParseStatus::Failed);
        assert!(repositories.parse_jobs.next_queued().unwrap().is_none());
    }

    #[test]
    fn requeue_interrupted_jobs_recovers_jobs_left_parsing() {
        let repositories = test_repositories();
        let note = repositories.notes.create_raw_note("raw note").unwrap();
        repositories.parse_jobs.claim_next_queued().unwrap().unwrap();

        let requeued = ParseQueue::new(repositories.clone())
            .requeue_interrupted_jobs()
            .unwrap();

        assert_eq!(requeued, 1);
        let stored = repositories.notes.get(note.id).unwrap().unwrap();
        assert_eq!(stored.parse_status, ParseStatus::Queued);
        let queued = repositories.parse_jobs.next_queued().unwrap().unwrap();
        assert_eq!(queued.note_id, note.id);
        assert_eq!(
            queued.last_error.as_deref(),
            Some("parser was interrupted before completion; retrying")
        );
    }

    #[test]
    fn successful_parse_resets_cleaned_edited_flag() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("sam said fix qa flag")
            .unwrap();
        let note_id = note.id;
        repositories.parse_jobs.enqueue(note_id).unwrap();

        repositories
            .notes
            .update_cleaned_by_user(note_id, "User Title", "user body", "user summary")
            .expect("mark edited");
        assert!(
            repositories
                .notes
                .get(note_id)
                .unwrap()
                .unwrap()
                .cleaned_edited,
            "precondition: note is user-edited"
        );

        ParseQueue::new(repositories.clone())
            .process_next_with_provider(&StaticProvider)
            .expect("process parse");

        let stored = repositories.notes.get(note_id).unwrap().unwrap();
        assert!(!stored.cleaned_edited, "parser write reclaims ownership");
    }

    #[test]
    fn process_next_replaces_unreviewed_parser_suggestions_on_retry() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("sam said fix qa flag")
            .unwrap();
        let queue = ParseQueue::new(repositories.clone());

        repositories.parse_jobs.enqueue(note.id).unwrap();
        queue.process_next_with_provider(&StaticProvider).unwrap();
        repositories.parse_jobs.enqueue(note.id).unwrap();
        queue.process_next_with_provider(&StaticProvider).unwrap();

        let actions = repositories.action_items.list_for_note(note.id).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].text, "Fix the QA flag.");
    }

    #[test]
    fn process_next_records_raw_provider_response_separately_from_parsed_json() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("sam said fix qa flag")
            .unwrap();
        repositories.parse_jobs.enqueue(note.id).unwrap();

        ParseQueue::new(repositories.clone())
            .process_next_with_provider(&RawResponseProvider)
            .unwrap();

        let connection = repositories.database.connection().unwrap();
        let (raw_response, parsed_json, feedback): (String, String, Option<String>) = connection
            .query_row(
                "SELECT raw_response, parsed_json, feedback FROM parse_runs WHERE note_id = ?1",
                [note.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(raw_response, RAW_PROVIDER_RESPONSE);
        assert_ne!(raw_response, parsed_json);
        assert_eq!(feedback, None);
        assert!(parsed_json.contains("Sam said to fix the QA flag."));
    }

    #[test]
    fn process_next_records_reparse_feedback_in_parse_run() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("sam said fix qa flag")
            .unwrap();
        repositories
            .parse_jobs
            .enqueue_with_feedback(note.id, Some("Tag this as QA follow-up"))
            .unwrap();

        ParseQueue::new(repositories.clone())
            .process_next_with_provider(&RawResponseProvider)
            .unwrap();

        let connection = repositories.database.connection().unwrap();
        let feedback: Option<String> = connection
            .query_row(
                "SELECT feedback FROM parse_runs WHERE note_id = ?1",
                [note.id.to_string()],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(feedback.as_deref(), Some("Tag this as QA follow-up"));
    }

    #[test]
    fn retry_parse_uses_updated_raw_text() {
        let repositories = test_repositories();
        let note = repositories
            .notes
            .create_raw_note("sam said fix qa flag")
            .unwrap();
        let note_id = note.id;
        repositories
            .notes
            .update_raw_text_by_user(note_id, "maya said update the calico rollout")
            .unwrap();
        repositories.parse_jobs.enqueue(note_id).unwrap();

        ParseQueue::new(repositories.clone())
            .process_next_with_provider(&EchoRawProvider)
            .unwrap();

        let stored = repositories.notes.get(note_id).unwrap().unwrap();
        assert_eq!(stored.title, "maya said update the calico rollout");
        assert_eq!(
            stored.cleaned_text.as_deref(),
            Some("Parsed from: maya said update the calico rollout")
        );
    }

    struct StaticProvider;

    impl ParserProvider for StaticProvider {
        fn parse(&self, _input: &str) -> Result<ParserResult, ParserError> {
            Ok(ParserResult {
                title: "Fix QA Flag".to_string(),
                cleaned_text: "Sam said to fix the QA flag.".to_string(),
                summary: "Fix QA flag.".to_string(),
                tags: vec![ParsedTag {
                    kind: ParserTagKind::Topic,
                    name: "QA".to_string(),
                    confidence: 0.93,
                }],
                action_items: vec![ParsedActionItem {
                    text: "Fix the QA flag.".to_string(),
                    owner: None,
                    due_date: None,
                    confidence: 0.81,
                    requires_review: true,
                }],
            })
        }
    }

    struct EchoRawProvider;

    impl ParserProvider for EchoRawProvider {
        fn parse(&self, input: &str) -> Result<ParserResult, ParserError> {
            Ok(ParserResult {
                title: input.to_string(),
                cleaned_text: format!("Parsed from: {input}"),
                summary: input.to_string(),
                tags: Vec::new(),
                action_items: Vec::new(),
            })
        }
    }

    struct FailingProvider;

    impl ParserProvider for FailingProvider {
        fn parse(&self, _input: &str) -> Result<ParserResult, ParserError> {
            Err(ParserError::Provider("codex unavailable".to_string()))
        }
    }

    struct TimeoutProvider;

    impl ParserProvider for TimeoutProvider {
        fn parse(&self, _input: &str) -> Result<ParserResult, ParserError> {
            Err(ParserError::Provider(
                "codex command timed out after 90s".to_string(),
            ))
        }
    }

    const RAW_PROVIDER_RESPONSE: &str = "{\n  \"title\": \"Fix QA Flag\",\n  \"cleanedText\": \"Sam said to fix the QA flag.\",\n  \"summary\": \"Fix QA flag.\",\n  \"tags\": [],\n  \"actionItems\": []\n}";

    struct RawResponseProvider;

    impl ParserProvider for RawResponseProvider {
        fn parse(&self, _input: &str) -> Result<ParserResult, ParserError> {
            StaticProvider.parse("")
        }

        fn parse_output(&self, _input: &str) -> Result<ParserOutput, ParserError> {
            Ok(ParserOutput {
                raw_response: RAW_PROVIDER_RESPONSE.to_string(),
                result: StaticProvider.parse("")?,
            })
        }
    }
}
