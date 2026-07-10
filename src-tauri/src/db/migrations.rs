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
          is_archived INTEGER NOT NULL DEFAULT 0
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

        CREATE INDEX IF NOT EXISTS idx_notes_created_at
          ON notes(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_notes_parse_status
          ON notes(parse_status);
        CREATE INDEX IF NOT EXISTS idx_notes_review_status
          ON notes(review_status);
        CREATE INDEX IF NOT EXISTS idx_note_tags_tag_id
          ON note_tags(tag_id);
        CREATE INDEX IF NOT EXISTS idx_action_items_note_id
          ON action_items(note_id);
        CREATE INDEX IF NOT EXISTS idx_parse_jobs_status_created_at
          ON parse_jobs(status, created_at);
        CREATE INDEX IF NOT EXISTS idx_parse_runs_note_id
          ON parse_runs(note_id);
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
