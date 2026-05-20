use rusqlite::{params, OptionalExtension, Row, Transaction};

use crate::db::Database;
use crate::domain::{NoteId, ParseJob, ParseJobId, ParseRun, ParseRunId, ParseStatus};

use super::{
    now_db_string, optional_db_datetime, parse_db_datetime, u32_from_i64, RepositoryError,
    RepositoryResult,
};

#[derive(Clone)]
pub struct ParseJobRepository {
    db: Database,
}

impl ParseJobRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn enqueue(&self, note_id: NoteId) -> RepositoryResult<ParseJob> {
        let id = ParseJobId::new();
        let id_text = id.to_string();
        let note_id_text = note_id.to_string();
        let created_at = now_db_string();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO parse_jobs (
                id, note_id, status, attempt_count, last_error, created_at, started_at, finished_at
             ) VALUES (?1, ?2, ?3, 0, NULL, ?4, NULL, NULL)",
            params![
                id_text,
                note_id_text,
                ParseStatus::Queued.as_str(),
                created_at
            ],
        )?;
        transaction.execute(
            "UPDATE notes SET parse_status = ?2, updated_at = ?3 WHERE id = ?1",
            params![note_id_text, ParseStatus::Queued.as_str(), created_at],
        )?;
        let job =
            job_by_id(&transaction, &id.to_string())?.ok_or_else(|| RepositoryError::NotFound {
                entity: "parse_job",
                id: id.to_string(),
            })?;
        transaction.commit()?;
        Ok(job)
    }

    pub fn next_queued(&self) -> RepositoryResult<Option<ParseJob>> {
        let connection = self.db.connection()?;
        let record = connection
            .query_row(
                "SELECT id, note_id, status, attempt_count, last_error,
                        created_at, started_at, finished_at
                 FROM parse_jobs
                 WHERE status = ?1
                 ORDER BY created_at, rowid
                 LIMIT 1",
                [ParseStatus::Queued.as_str()],
                ParseJobRecord::from_row,
            )
            .optional()?;
        record.map(ParseJobRecord::into_job).transpose()
    }

    pub fn claim_next_queued(&self) -> RepositoryResult<Option<ParseJob>> {
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        let job_id = transaction
            .query_row(
                "SELECT id
                 FROM parse_jobs
                 WHERE status = ?1
                 ORDER BY created_at, rowid
                 LIMIT 1",
                [ParseStatus::Queued.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        let Some(job_id) = job_id else {
            transaction.commit()?;
            return Ok(None);
        };

        let now = now_db_string();
        let note_id = transaction.query_row(
            "SELECT note_id FROM parse_jobs WHERE id = ?1",
            [job_id.as_str()],
            |row| row.get::<_, String>(0),
        )?;
        transaction.execute(
            "UPDATE parse_jobs
             SET status = ?2,
                 attempt_count = attempt_count + 1,
                 last_error = NULL,
                 started_at = ?3,
                 finished_at = NULL
             WHERE id = ?1",
            params![job_id, ParseStatus::Parsing.as_str(), now],
        )?;
        transaction.execute(
            "UPDATE notes SET parse_status = ?2, updated_at = ?3 WHERE id = ?1",
            params![note_id, ParseStatus::Parsing.as_str(), now],
        )?;
        let job = job_by_id(&transaction, &job_id)?.ok_or_else(|| RepositoryError::NotFound {
            entity: "parse_job",
            id: job_id.clone(),
        })?;
        transaction.commit()?;
        Ok(Some(job))
    }

    pub fn mark_parsed(&self, id: ParseJobId) -> RepositoryResult<()> {
        self.finish_job(id, ParseStatus::Parsed, None)
    }

    pub fn mark_failed(&self, id: ParseJobId, error: &str) -> RepositoryResult<()> {
        self.finish_job(id, ParseStatus::Failed, Some(error))
    }

    pub fn record_run(
        &self,
        note_id: NoteId,
        provider: &str,
        prompt_version: &str,
        raw_response: &str,
        parsed_json: &str,
    ) -> RepositoryResult<ParseRun> {
        let id = ParseRunId::new();
        let created_at = now_db_string();
        let connection = self.db.connection()?;
        connection.execute(
            "INSERT INTO parse_runs (
                id, note_id, provider, prompt_version, raw_response, parsed_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id.to_string(),
                note_id.to_string(),
                provider,
                prompt_version,
                raw_response,
                parsed_json,
                created_at
            ],
        )?;

        Ok(ParseRun {
            id,
            note_id,
            provider: provider.to_string(),
            prompt_version: prompt_version.to_string(),
            raw_response: raw_response.to_string(),
            parsed_json: parsed_json.to_string(),
            created_at: parse_db_datetime("created_at", created_at)?,
        })
    }

    fn finish_job(
        &self,
        id: ParseJobId,
        status: ParseStatus,
        last_error: Option<&str>,
    ) -> RepositoryResult<()> {
        let id_text = id.to_string();
        let now = now_db_string();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        let note_id = transaction
            .query_row(
                "SELECT note_id FROM parse_jobs WHERE id = ?1",
                [id_text.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| RepositoryError::NotFound {
                entity: "parse_job",
                id: id.to_string(),
            })?;

        transaction.execute(
            "UPDATE parse_jobs
             SET status = ?2, last_error = ?3, finished_at = ?4
             WHERE id = ?1",
            params![id_text, status.as_str(), last_error, now],
        )?;
        transaction.execute(
            "UPDATE notes SET parse_status = ?2, updated_at = ?3 WHERE id = ?1",
            params![note_id, status.as_str(), now],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn job_by_id(transaction: &Transaction<'_>, id: &str) -> RepositoryResult<Option<ParseJob>> {
    let record = transaction
        .query_row(
            "SELECT id, note_id, status, attempt_count, last_error,
                    created_at, started_at, finished_at
             FROM parse_jobs
             WHERE id = ?1",
            [id],
            ParseJobRecord::from_row,
        )
        .optional()?;
    record.map(ParseJobRecord::into_job).transpose()
}

struct ParseJobRecord {
    id: String,
    note_id: String,
    status: String,
    attempt_count: i64,
    last_error: Option<String>,
    created_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
}

impl ParseJobRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            note_id: row.get(1)?,
            status: row.get(2)?,
            attempt_count: row.get(3)?,
            last_error: row.get(4)?,
            created_at: row.get(5)?,
            started_at: row.get(6)?,
            finished_at: row.get(7)?,
        })
    }

    fn into_job(self) -> RepositoryResult<ParseJob> {
        Ok(ParseJob {
            id: ParseJobId::parse(&self.id)?,
            note_id: NoteId::parse(&self.note_id)?,
            status: ParseStatus::from_db(&self.status)?,
            attempt_count: u32_from_i64("attempt_count", self.attempt_count)?,
            last_error: self.last_error,
            created_at: parse_db_datetime("created_at", self.created_at)?,
            started_at: optional_db_datetime("started_at", self.started_at)?,
            finished_at: optional_db_datetime("finished_at", self.finished_at)?,
        })
    }
}
