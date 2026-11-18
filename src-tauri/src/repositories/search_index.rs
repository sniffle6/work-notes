//! The `notes_fts` search rows of notes.
//!
//! `notes_fts.note_id` is UNINDEXED, so finding a note's row by note ID
//! reads the whole index. `note_search_rows` records the rowid of each
//! note's search row, which turns that into a primary-key lookup.

use rusqlite::{params, Transaction};

/// Replaces the search text of a note, creating its search row the first
/// time. The note keeps the same search rowid for its whole life.
pub(crate) fn replace_note_text(
    transaction: &Transaction<'_>,
    note_id: &str,
    raw_text: &str,
    cleaned_text: Option<&str>,
    summary: Option<&str>,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO note_search_rows (note_id) VALUES (?1)
         ON CONFLICT(note_id) DO NOTHING",
        [note_id],
    )?;
    let search_rowid: i64 = transaction.query_row(
        "SELECT search_rowid FROM note_search_rows WHERE note_id = ?1",
        [note_id],
        |row| row.get(0),
    )?;
    transaction.execute("DELETE FROM notes_fts WHERE rowid = ?1", [search_rowid])?;
    transaction.execute(
        "INSERT INTO notes_fts (rowid, note_id, raw_text, cleaned_text, summary)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![search_rowid, note_id, raw_text, cleaned_text, summary],
    )?;
    Ok(())
}

/// Removes a note's search row and its rowid record. A note without a
/// search row is left as it is.
pub(crate) fn remove_note(transaction: &Transaction<'_>, note_id: &str) -> rusqlite::Result<()> {
    transaction.execute(
        "DELETE FROM notes_fts
         WHERE rowid IN (SELECT search_rowid FROM note_search_rows WHERE note_id = ?1)",
        [note_id],
    )?;
    transaction.execute("DELETE FROM note_search_rows WHERE note_id = ?1", [note_id])?;
    Ok(())
}
