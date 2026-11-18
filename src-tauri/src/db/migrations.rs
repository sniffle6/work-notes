use rusqlite::Connection;

pub fn run(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS notes (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL DEFAULT 'Untitled note',
          raw_text TEXT NOT NULL,
          cleaned_text TEXT,
          summary TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          capture_source TEXT NOT NULL,
          parse_status TEXT NOT NULL,
          review_status TEXT NOT NULL,
          is_archived INTEGER NOT NULL DEFAULT 0,
          completed_at TEXT,
          completion_note TEXT
        );

        CREATE TABLE IF NOT EXISTS jira_writing (
          scope TEXT PRIMARY KEY,
          content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS jira_saved_lists (
          scope TEXT NOT NULL,
          id TEXT NOT NULL,
          content TEXT NOT NULL,
          saved_at TEXT NOT NULL,
          PRIMARY KEY(scope, id)
        );

        CREATE TABLE IF NOT EXISTS jira_saved_issues (
          scope TEXT NOT NULL,
          id TEXT NOT NULL,
          content TEXT NOT NULL,
          saved_at TEXT NOT NULL,
          PRIMARY KEY(scope, id)
        );

        CREATE TABLE IF NOT EXISTS card_notes (
          id TEXT PRIMARY KEY,
          note_id TEXT NOT NULL,
          text TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
          note_id UNINDEXED,
          raw_text,
          cleaned_text,
          summary
        );

        CREATE TABLE IF NOT EXISTS tags (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          kind TEXT NOT NULL,
          created_at TEXT NOT NULL,
          UNIQUE(name, kind)
        );

        CREATE TABLE IF NOT EXISTS note_tags (
          note_id TEXT NOT NULL,
          tag_id TEXT NOT NULL,
          source TEXT NOT NULL,
          confidence REAL,
          PRIMARY KEY(note_id, tag_id, source),
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
          FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS action_items (
          id TEXT PRIMARY KEY,
          note_id TEXT NOT NULL,
          text TEXT NOT NULL,
          owner TEXT,
          due_date TEXT,
          status TEXT NOT NULL,
          source TEXT NOT NULL,
          confidence REAL,
          followup_state TEXT,
          followup_lane TEXT,
          completed_at TEXT,
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS parse_jobs (
          id TEXT PRIMARY KEY,
          note_id TEXT NOT NULL,
          status TEXT NOT NULL,
          attempt_count INTEGER NOT NULL DEFAULT 0,
          last_error TEXT,
          created_at TEXT NOT NULL,
          started_at TEXT,
          finished_at TEXT,
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS parse_runs (
          id TEXT PRIMARY KEY,
          note_id TEXT NOT NULL,
          provider TEXT NOT NULL,
          prompt_version TEXT NOT NULL,
          raw_response TEXT NOT NULL,
          parsed_json TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS settings (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS jira_chat_threads (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS jira_chat_messages (
          id TEXT PRIMARY KEY,
          thread_id TEXT NOT NULL,
          role TEXT NOT NULL,
          content TEXT NOT NULL,
          payload TEXT,
          status TEXT,
          result TEXT,
          created_at TEXT NOT NULL,
          FOREIGN KEY(thread_id) REFERENCES jira_chat_threads(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS note_jira_links (
          note_id TEXT NOT NULL,
          issue_key TEXT NOT NULL,
          created_at TEXT NOT NULL,
          PRIMARY KEY(note_id, issue_key),
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_notes_created_at
          ON notes(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_notes_parse_status
          ON notes(parse_status);
        CREATE INDEX IF NOT EXISTS idx_notes_review_status
          ON notes(review_status);
        CREATE INDEX IF NOT EXISTS idx_note_tags_tag_id
          ON note_tags(tag_id);
        CREATE INDEX IF NOT EXISTS idx_card_notes_note_id_created_at
          ON card_notes(note_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_action_items_note_id
          ON action_items(note_id);
        CREATE INDEX IF NOT EXISTS idx_parse_jobs_status_created_at
          ON parse_jobs(status, created_at);
        CREATE INDEX IF NOT EXISTS idx_parse_runs_note_id
          ON parse_runs(note_id);
        CREATE INDEX IF NOT EXISTS idx_jira_chat_messages_thread_id_created_at
          ON jira_chat_messages(thread_id, created_at);
        ",
    )?;

    ensure_column(connection, "parse_jobs", "feedback", "feedback TEXT")?;
    ensure_column(connection, "parse_runs", "feedback", "feedback TEXT")?;
    ensure_column(connection, "notes", "title", "title TEXT")?;
    ensure_column(
        connection,
        "action_items",
        "followup_state",
        "followup_state TEXT",
    )?;
    ensure_column(
        connection,
        "action_items",
        "followup_lane",
        "followup_lane TEXT",
    )?;
    ensure_column(
        connection,
        "action_items",
        "completed_at",
        "completed_at TEXT",
    )?;
    ensure_column(
        connection,
        "notes",
        "cleaned_edited",
        "cleaned_edited INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(connection, "notes", "completed_at", "completed_at TEXT")?;
    ensure_column(
        connection,
        "notes",
        "completion_note",
        "completion_note TEXT",
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_action_items_status_followup_state
         ON action_items(status, followup_state)",
        [],
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_action_items_completed_at
         ON action_items(completed_at)",
        [],
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_notes_completed_at
         ON notes(completed_at)",
        [],
    )?;
    ensure_note_search_rows(connection)?;
    connection.execute(
        "UPDATE notes
         SET title = CASE
           WHEN summary IS NOT NULL AND length(trim(summary)) > 0 THEN trim(summary)
           WHEN length(trim(raw_text)) > 0 THEN substr(trim(raw_text), 1, 80)
           ELSE 'Untitled note'
         END
         WHERE title IS NULL OR length(trim(title)) = 0",
        [],
    )?;

    Ok(())
}

/// Creates `note_search_rows`, the rowid of each note's `notes_fts` row
/// (see `repositories::search_index`). Search rows written before the
/// table existed are adopted when it is created.
///
/// Versions before this table write search rows without recording them.
/// If one ran on this database since, its rows sit past the last recorded
/// rowid, and they are adopted again.
fn ensure_note_search_rows(connection: &Connection) -> rusqlite::Result<()> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'note_search_rows'
         )",
        [],
        |row| row.get(0),
    )?;
    if exists {
        if search_rows_are_unrecorded(connection)? {
            let transaction = connection.unchecked_transaction()?;
            adopt_search_rows(&transaction)?;
            transaction.commit()?;
        }
        return Ok(());
    }

    // One transaction, so a crash cannot leave the table without the rows
    // it should have adopted.
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(
        "
        CREATE TABLE note_search_rows (
          search_rowid INTEGER PRIMARY KEY,
          note_id TEXT NOT NULL UNIQUE,
          FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
        );
        ",
    )?;
    adopt_search_rows(&transaction)?;
    transaction.commit()
}

/// True when `notes_fts` holds a row past the last recorded search rowid.
/// Both lookups read one end of a rowid index, not the whole table.
fn search_rows_are_unrecorded(connection: &Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT COALESCE((SELECT rowid FROM notes_fts ORDER BY rowid DESC LIMIT 1), 0)
              > COALESCE((SELECT MAX(search_rowid) FROM note_search_rows), 0)",
        [],
        |row| row.get(0),
    )
}

/// Rebuilds `note_search_rows` from the rows in `notes_fts`. A note with
/// several search rows keeps its newest one, and rows of missing notes are
/// dropped, so every note is left with at most one search row.
fn adopt_search_rows(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        DELETE FROM note_search_rows;
        INSERT INTO note_search_rows (search_rowid, note_id)
          SELECT MAX(rowid), note_id FROM notes_fts
          WHERE note_id IN (SELECT id FROM notes)
          GROUP BY note_id;
        DELETE FROM notes_fts
          WHERE rowid NOT IN (SELECT search_rowid FROM note_search_rows);
        ",
    )
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !columns.iter().any(|existing| existing == column) {
        connection.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }

    Ok(())
}
