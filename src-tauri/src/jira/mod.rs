//! Jira Cloud integration: REST client, document conversion, and the chat
//! agent's tool contract. HTTP details stay behind [`transport::JiraTransport`].

pub mod adf;
pub mod agent;
pub mod chat;
pub mod client;
pub mod transitions;
pub mod transport;
pub mod types;

pub use client::{JiraClient, JiraOps};
pub use types::{
    Assignee, IssueKey, IssueUpdate, JiraAttachment, JiraComment, JiraIssueDetail,
    JiraIssueSummary, JiraTransition, JiraUser, NewIssue, TransitionOutcome,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JiraError {
    #[error("Jira is not configured: {0}")]
    NotConfigured(String),
    #[error("Jira rejected the credentials; check the account email and JIRA_API_TOKEN")]
    Unauthorized,
    #[error(
        "Jira returned 404 for {path}; the item is missing or the API token lacks read:jira-work"
    )]
    NotFound { path: String },
    #[error("Jira {method} {path} failed with HTTP {status}: {message}")]
    Http {
        method: &'static str,
        path: String,
        status: u16,
        message: String,
    },
    #[error("could not reach Jira: {0}")]
    Network(String),
    #[error("Jira response was not understood: {0}")]
    InvalidResponse(String),
    /// An attachment's content could not be read from Jira or saved to disk.
    #[error("attachment download failed: {0}")]
    Download(String),
    #[error("{0}")]
    InvalidInput(String),
}
