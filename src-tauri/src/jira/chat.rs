use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A locally captured comparison for the approval UI, never supplied by the agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalReview {
    pub title: String,
    pub captured_at: String,
    pub warning: Option<String>,
    pub fields: Vec<ProposalField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalField {
    pub label: String,
    pub before: Option<String>,
    pub after: String,
}

impl ProposalReview {
    pub(crate) fn from_payload(payload: Option<&str>) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(payload?).ok()?;
        serde_json::from_value(value.get("review")?.clone()).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    User,
    Assistant,
    /// A read-only Jira lookup the agent ran, with its result.
    Tool,
    /// A Jira change the agent wants to make. It runs only after approval.
    Proposal,
}

impl ChatRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::Proposal => "proposal",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [Self::User, Self::Assistant, Self::Tool, Self::Proposal]
            .into_iter()
            .find(|role| role.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMessageStatus {
    /// Proposal waiting for the user.
    Pending,
    /// Proposal approved; its Jira change is running or has succeeded.
    Approved,
    /// Proposal declined by the user. Nothing was sent to Jira.
    Rejected,
    /// Approved proposal whose Jira change failed.
    Failed,
    /// Read-only lookup succeeded.
    Ok,
    /// Read-only lookup failed.
    Error,
}

impl ChatMessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Pending,
            Self::Approved,
            Self::Rejected,
            Self::Failed,
            Self::Ok,
            Self::Error,
        ]
        .into_iter()
        .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatThread {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub thread_id: String,
    pub role: ChatRole,
    /// Text shown in the conversation.
    pub content: String,
    /// Serialized tool call for `Tool` and `Proposal` messages.
    #[serde(skip_serializing)]
    pub payload: Option<String>,
    pub status: Option<ChatMessageStatus>,
    /// Lookup output or the outcome of an approved change.
    pub result: Option<String>,
    pub review: Option<ProposalReview>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub payload: Option<String>,
    pub status: Option<ChatMessageStatus>,
    pub result: Option<String>,
}

impl NewChatMessage {
    pub fn text(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            payload: None,
            status: None,
            result: None,
        }
    }
}
