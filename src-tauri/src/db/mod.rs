pub mod migrations;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database connection lock poisoned")]
    LockPoisoned,
}

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, DatabaseError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn connection(&self) -> Result<MutexGuard<'_, Connection>, DatabaseError> {
        self.connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)
    }

    fn from_connection(connection: Connection) -> Result<Self, DatabaseError> {
        configure(&connection)?;
        migrations::run(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }
}

fn configure(connection: &Connection) -> Result<(), DatabaseError> {
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    // A write-ahead log commits with one append and flush instead of
    // rewriting a rollback journal. FULL still flushes on every commit, so
    // a saved raw note survives a power cut; NORMAL could lose it. The log
    // lives beside the database as -wal and -shm files. An in-memory
    // database stays in memory mode.
    connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA temp_store = MEMORY;
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Database;

    #[test]
    fn a_database_file_logs_writes_ahead_and_syncs_every_commit() {
        let folder = tempfile::tempdir().unwrap();
        let db = Database::open(folder.path().join("notes.sqlite3")).unwrap();
        let connection = db.connection().unwrap();

        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        let synchronous: i64 = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();

        assert_eq!(journal_mode, "wal");
        // 2 is FULL. A saved raw note must survive a power cut.
        assert_eq!(synchronous, 2);
    }

    #[test]
    fn db_migration_creates_required_tables() {
        let db = Database::in_memory().unwrap();
        let connection = db.connection().unwrap();

        for table in [
            "notes",
            "notes_fts",
            "card_notes",
            "tags",
            "note_tags",
            "action_items",
            "parse_jobs",
            "parse_runs",
            "settings",
            "jira_chat_threads",
            "jira_chat_messages",
            "note_jira_links",
            "jira_saved_lists",
            "jira_saved_issues",
            "note_search_rows",
        ] {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "{table} should exist");
        }

        let mut statement = connection
            .prepare("PRAGMA table_info(action_items)")
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            columns.iter().any(|column| column == "followup_state"),
            "action_items.followup_state should exist"
        );
        assert!(
            columns.iter().any(|column| column == "followup_lane"),
            "action_items.followup_lane should exist"
        );
        assert!(
            columns.iter().any(|column| column == "completed_at"),
            "action_items.completed_at should exist"
        );

        let mut statement = connection.prepare("PRAGMA table_info(notes)").unwrap();
        let note_columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            note_columns.iter().any(|column| column == "completed_at"),
            "notes.completed_at should exist"
        );
        assert!(
            note_columns
                .iter()
                .any(|column| column == "completion_note"),
            "notes.completion_note should exist"
        );
    }
}
