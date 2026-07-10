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
    fn db_migration_creates_required_tables() {
        let db = Database::in_memory().unwrap();
        let connection = db.connection().unwrap();

        for table in [
            "notes",
            "notes_fts",
            "tags",
            "note_tags",
            "action_items",
            "parse_jobs",
            "parse_runs",
            "settings",
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
    }
}
