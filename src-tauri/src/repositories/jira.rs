use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use crate::db::Database;
use crate::domain::NoteId;
use crate::jira::chat::{ChatMessage, ChatMessageStatus, ChatRole, ChatThread, NewChatMessage};
use crate::jira::IssueKey;

use super::{now_db_string, parse_db_datetime, RepositoryError, RepositoryResult};

/// Stores Jira agent conversations.
#[derive(Clone)]
pub struct JiraChatRepository {
    db: Database,
}

impl JiraChatRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn create_thread(&self, title: &str) -> RepositoryResult<ChatThread> {
        let id = Uuid::new_v4().to_string();
        let now = now_db_string();
        self.db.connection()?.execute(
            "INSERT INTO jira_chat_threads (id, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)",
            params![&id, title, &now],
        )?;

        let created_at = parse_db_datetime("created_at", now)?;
        Ok(ChatThread {
            id,
            title: title.to_string(),
            created_at,
            updated_at: created_at,
        })
    }

    /// Lists threads with the most recently active first.
    pub fn list_threads(&self) -> RepositoryResult<Vec<ChatThread>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, title, created_at, updated_at
             FROM jira_chat_threads
             ORDER BY updated_at DESC, rowid DESC",
        )?;
        let records = statement
            .query_map([], ThreadRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        records.into_iter().map(ThreadRecord::into_thread).collect()
    }

    pub fn get_thread(&self, thread_id: &str) -> RepositoryResult<Option<ChatThread>> {
        self.db
            .connection()?
            .query_row(
                "SELECT id, title, created_at, updated_at FROM jira_chat_threads WHERE id = ?1",
                [thread_id],
                ThreadRecord::from_row,
            )
            .optional()?
            .map(ThreadRecord::into_thread)
            .transpose()
    }

    pub fn rename_thread(&self, thread_id: &str, title: &str) -> RepositoryResult<()> {
        let changed = self.db.connection()?.execute(
            "UPDATE jira_chat_threads SET title = ?2 WHERE id = ?1",
            params![thread_id, title],
        )?;
        thread_must_exist(changed, thread_id)
    }

    /// Deletes a thread and its messages.
    pub fn delete_thread(&self, thread_id: &str) -> RepositoryResult<()> {
        let changed = self
            .db
            .connection()?
            .execute("DELETE FROM jira_chat_threads WHERE id = ?1", [thread_id])?;
        thread_must_exist(changed, thread_id)
    }

    /// Appends a message and marks the thread as recently active.
    pub fn add_message(
        &self,
        thread_id: &str,
        message: NewChatMessage,
    ) -> RepositoryResult<ChatMessage> {
        let id = Uuid::new_v4().to_string();
        let now = now_db_string();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE jira_chat_threads SET updated_at = ?2 WHERE id = ?1",
            params![thread_id, &now],
        )?;
        thread_must_exist(changed, thread_id)?;
        transaction.execute(
            "INSERT INTO jira_chat_messages
               (id, thread_id, role, content, payload, status, result, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &id,
                thread_id,
                message.role.as_str(),
                &message.content,
                &message.payload,
                message.status.map(ChatMessageStatus::as_str),
                &message.result,
                &now,
            ],
        )?;
        transaction.commit()?;

        Ok(ChatMessage {
            id,
            thread_id: thread_id.to_string(),
            role: message.role,
            content: message.content,
            payload: message.payload,
            status: message.status,
            result: message.result,
            created_at: parse_db_datetime("created_at", now)?,
        })
    }

    /// Lists a thread's messages oldest first.
    pub fn list_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(&format!(
            "{MESSAGE_SELECT} WHERE thread_id = ?1 ORDER BY created_at ASC, rowid ASC"
        ))?;
        let records = statement
            .query_map([thread_id], MessageRecord::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        records
            .into_iter()
            .map(MessageRecord::into_message)
            .collect()
    }

    pub fn get_message(&self, message_id: &str) -> RepositoryResult<Option<ChatMessage>> {
        self.db
            .connection()?
            .query_row(
                &format!("{MESSAGE_SELECT} WHERE id = ?1"),
                [message_id],
                MessageRecord::from_row,
            )
            .optional()?
            .map(MessageRecord::into_message)
            .transpose()
    }

    /// Moves a pending proposal to `to` and reports whether it was pending.
    ///
    /// The check and update are one statement, so a proposal can be claimed
    /// for execution only once.
    pub fn resolve_pending_proposal(
        &self,
        message_id: &str,
        to: ChatMessageStatus,
    ) -> RepositoryResult<bool> {
        let changed = self.db.connection()?.execute(
            "UPDATE jira_chat_messages SET status = ?2
             WHERE id = ?1 AND role = ?3 AND status = ?4",
            params![
                message_id,
                to.as_str(),
                ChatRole::Proposal.as_str(),
                ChatMessageStatus::Pending.as_str(),
            ],
        )?;
        Ok(changed == 1)
    }

    /// Records the outcome of an approved proposal.
    pub fn finish_proposal(
        &self,
        message_id: &str,
        status: ChatMessageStatus,
        result: &str,
    ) -> RepositoryResult<()> {
        let changed = self.db.connection()?.execute(
            "UPDATE jira_chat_messages SET status = ?2, result = ?3 WHERE id = ?1",
            params![message_id, status.as_str(), result],
        )?;
        if changed == 0 {
            return Err(RepositoryError::NotFound {
                entity: "chat message",
                id: message_id.to_string(),
            });
        }
        Ok(())
    }
}

const MESSAGE_SELECT: &str =
    "SELECT id, thread_id, role, content, payload, status, result, created_at
     FROM jira_chat_messages";

fn thread_must_exist(changed_rows: usize, thread_id: &str) -> RepositoryResult<()> {
    if changed_rows == 0 {
        return Err(RepositoryError::NotFound {
            entity: "chat thread",
            id: thread_id.to_string(),
        });
    }
    Ok(())
}

struct ThreadRecord {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
}

impl ThreadRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    }

    fn into_thread(self) -> RepositoryResult<ChatThread> {
        Ok(ChatThread {
            id: self.id,
            title: self.title,
            created_at: parse_db_datetime("created_at", self.created_at)?,
            updated_at: parse_db_datetime("updated_at", self.updated_at)?,
        })
    }
}

struct MessageRecord {
    id: String,
    thread_id: String,
    role: String,
    content: String,
    payload: Option<String>,
    status: Option<String>,
    result: Option<String>,
    created_at: String,
}

impl MessageRecord {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            thread_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            payload: row.get(4)?,
            status: row.get(5)?,
            result: row.get(6)?,
            created_at: row.get(7)?,
        })
    }

    fn into_message(self) -> RepositoryResult<ChatMessage> {
        let role = ChatRole::parse(&self.role).ok_or(RepositoryError::InvalidDatabaseValue {
            field: "role",
            value: self.role,
        })?;
        let status = self
            .status
            .map(|status| {
                ChatMessageStatus::parse(&status).ok_or(RepositoryError::InvalidDatabaseValue {
                    field: "status",
                    value: status,
                })
            })
            .transpose()?;

        Ok(ChatMessage {
            id: self.id,
            thread_id: self.thread_id,
            role,
            content: self.content,
            payload: self.payload,
            status,
            result: self.result,
            created_at: parse_db_datetime("created_at", self.created_at)?,
        })
    }
}

/// Remembers which Jira tickets were created from which notes.
#[derive(Clone)]
pub struct NoteJiraLinkRepository {
    db: Database,
}

impl NoteJiraLinkRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Links a ticket to a note. Linking the same pair twice is a no-op.
    pub fn link(&self, note_id: NoteId, issue_key: &IssueKey) -> RepositoryResult<()> {
        let connection = self.db.connection()?;
        let note_exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = ?1)",
            [note_id.to_string()],
            |row| row.get(0),
        )?;
        if !note_exists {
            return Err(RepositoryError::NotFound {
                entity: "note",
                id: note_id.to_string(),
            });
        }
        connection.execute(
            "INSERT OR IGNORE INTO note_jira_links (note_id, issue_key, created_at)
             VALUES (?1, ?2, ?3)",
            params![note_id.to_string(), issue_key.as_str(), now_db_string()],
        )?;
        Ok(())
    }

    /// Lists linked ticket keys in the order they were linked.
    pub fn list_for_note(&self, note_id: NoteId) -> RepositoryResult<Vec<String>> {
        let connection = self.db.connection()?;
        let mut statement = connection.prepare(
            "SELECT issue_key FROM note_jira_links
             WHERE note_id = ?1
             ORDER BY created_at ASC, rowid ASC",
        )?;
        let keys = statement
            .query_map([note_id.to_string()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::domain::NoteId;
    use crate::jira::chat::{ChatMessageStatus, ChatRole, NewChatMessage};
    use crate::jira::IssueKey;
    use crate::repositories::{NoteRepository, RepositoryError};

    use super::{JiraChatRepository, NoteJiraLinkRepository};

    fn chat() -> JiraChatRepository {
        JiraChatRepository::new(Database::in_memory().expect("create database"))
    }

    fn proposal(content: &str) -> NewChatMessage {
        NewChatMessage {
            role: ChatRole::Proposal,
            content: content.to_string(),
            payload: Some("{\"tool\":\"add_comment\"}".to_string()),
            status: Some(ChatMessageStatus::Pending),
            result: None,
        }
    }

    #[test]
    fn threads_are_listed_by_latest_activity() {
        let chat = chat();
        let first = chat.create_thread("First").unwrap();
        let second = chat.create_thread("Second").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));

        chat.add_message(&first.id, NewChatMessage::text(ChatRole::User, "hello"))
            .unwrap();

        let threads = chat.list_threads().unwrap();
        assert_eq!(
            threads
                .iter()
                .map(|thread| thread.id.as_str())
                .collect::<Vec<_>>(),
            vec![first.id.as_str(), second.id.as_str()]
        );
        assert!(threads[0].updated_at > first.updated_at);
    }

    #[test]
    fn messages_keep_insertion_order_and_fields() {
        let chat = chat();
        let thread = chat.create_thread("Thread").unwrap();

        chat.add_message(
            &thread.id,
            NewChatMessage::text(ChatRole::User, "close AB-7"),
        )
        .unwrap();
        chat.add_message(
            &thread.id,
            NewChatMessage {
                role: ChatRole::Tool,
                content: "Look up AB-7".to_string(),
                payload: Some("{\"tool\":\"get_issue\"}".to_string()),
                status: Some(ChatMessageStatus::Ok),
                result: Some("AB-7 | To Do".to_string()),
            },
        )
        .unwrap();
        chat.add_message(&thread.id, proposal("Move AB-7 to \"Done\""))
            .unwrap();

        let messages = chat.list_messages(&thread.id).unwrap();
        assert_eq!(
            messages
                .iter()
                .map(|message| message.role)
                .collect::<Vec<_>>(),
            vec![ChatRole::User, ChatRole::Tool, ChatRole::Proposal]
        );
        assert_eq!(messages[1].status, Some(ChatMessageStatus::Ok));
        assert_eq!(messages[1].result.as_deref(), Some("AB-7 | To Do"));
        assert_eq!(
            messages[1].payload.as_deref(),
            Some("{\"tool\":\"get_issue\"}")
        );
        assert_eq!(messages[2].status, Some(ChatMessageStatus::Pending));
        assert_eq!(
            chat.get_message(&messages[2].id).unwrap().unwrap(),
            messages[2]
        );
    }

    #[test]
    fn adding_a_message_to_a_missing_thread_fails() {
        let error = chat()
            .add_message("missing", NewChatMessage::text(ChatRole::User, "hi"))
            .unwrap_err();

        assert!(matches!(
            error,
            RepositoryError::NotFound {
                entity: "chat thread",
                ..
            }
        ));
    }

    #[test]
    fn threads_can_be_renamed_and_deleted_with_their_messages() {
        let chat = chat();
        let thread = chat.create_thread("New chat").unwrap();
        let message = chat
            .add_message(&thread.id, NewChatMessage::text(ChatRole::User, "hi"))
            .unwrap();

        chat.rename_thread(&thread.id, "Printer tickets").unwrap();
        assert_eq!(
            chat.get_thread(&thread.id).unwrap().unwrap().title,
            "Printer tickets"
        );

        chat.delete_thread(&thread.id).unwrap();
        assert!(chat.get_thread(&thread.id).unwrap().is_none());
        assert!(chat.get_message(&message.id).unwrap().is_none());
        assert!(matches!(
            chat.delete_thread(&thread.id),
            Err(RepositoryError::NotFound { .. })
        ));
    }

    #[test]
    fn a_pending_proposal_can_be_resolved_only_once() {
        let chat = chat();
        let thread = chat.create_thread("Thread").unwrap();
        let message = chat
            .add_message(&thread.id, proposal("Unassign AB-7"))
            .unwrap();

        assert!(chat
            .resolve_pending_proposal(&message.id, ChatMessageStatus::Approved)
            .unwrap());
        assert!(!chat
            .resolve_pending_proposal(&message.id, ChatMessageStatus::Approved)
            .unwrap());
        assert!(!chat
            .resolve_pending_proposal(&message.id, ChatMessageStatus::Rejected)
            .unwrap());

        chat.finish_proposal(&message.id, ChatMessageStatus::Failed, "Jira said no")
            .unwrap();
        let stored = chat.get_message(&message.id).unwrap().unwrap();
        assert_eq!(stored.status, Some(ChatMessageStatus::Failed));
        assert_eq!(stored.result.as_deref(), Some("Jira said no"));
    }

    #[test]
    fn only_proposals_can_be_resolved() {
        let chat = chat();
        let thread = chat.create_thread("Thread").unwrap();
        let message = chat
            .add_message(&thread.id, NewChatMessage::text(ChatRole::User, "hi"))
            .unwrap();

        assert!(!chat
            .resolve_pending_proposal(&message.id, ChatMessageStatus::Approved)
            .unwrap());
    }

    #[test]
    fn note_links_are_idempotent_and_ordered() {
        let db = Database::in_memory().expect("create database");
        let note = NoteRepository::new(db.clone())
            .create_raw_note("Printer jams")
            .unwrap();
        let links = NoteJiraLinkRepository::new(db);

        links
            .link(note.id, &IssueKey::parse("AB-7").unwrap())
            .unwrap();
        links
            .link(note.id, &IssueKey::parse("AB-9").unwrap())
            .unwrap();
        links
            .link(note.id, &IssueKey::parse("AB-7").unwrap())
            .unwrap();

        assert_eq!(
            links.list_for_note(note.id).unwrap(),
            vec!["AB-7".to_string(), "AB-9".to_string()]
        );
    }

    #[test]
    fn linking_a_missing_note_fails() {
        let links = NoteJiraLinkRepository::new(Database::in_memory().expect("create database"));

        let error = links
            .link(NoteId::new(), &IssueKey::parse("AB-7").unwrap())
            .unwrap_err();

        assert!(matches!(
            error,
            RepositoryError::NotFound { entity: "note", .. }
        ));
    }
}
