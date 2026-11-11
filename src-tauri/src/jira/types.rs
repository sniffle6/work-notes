use std::fmt;

use serde::{Deserialize, Serialize};

use super::JiraError;

/// A validated, upper-cased Jira issue key such as `ABC-123`.
///
/// Validation keeps keys safe to place in REST paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IssueKey(String);

impl IssueKey {
    pub fn parse(value: &str) -> Result<Self, JiraError> {
        let key = value.trim().to_ascii_uppercase();
        let valid = key.split_once('-').is_some_and(|(project, number)| {
            is_project_key(project)
                && !number.is_empty()
                && number.bytes().all(|byte| byte.is_ascii_digit())
        });
        if !valid {
            return Err(JiraError::InvalidInput(format!(
                "\"{}\" is not a Jira issue key like ABC-123",
                value.trim()
            )));
        }
        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn project_key(&self) -> &str {
        self.0.split('-').next().unwrap_or_default()
    }
}

impl fmt::Display for IssueKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for IssueKey {
    type Error = JiraError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<IssueKey> for String {
    fn from(key: IssueKey) -> Self {
        key.0
    }
}

/// Upper-cases and validates a Jira project key such as `ABC`.
pub fn normalize_project_key(value: &str) -> Result<String, JiraError> {
    let key = value.trim().to_ascii_uppercase();
    if !is_project_key(&key) {
        return Err(JiraError::InvalidInput(format!(
            "\"{}\" is not a Jira project key like ABC",
            value.trim()
        )));
    }
    Ok(key)
}

fn is_project_key(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_uppercase())
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraUser {
    pub account_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraIssueSummary {
    pub key: IssueKey,
    pub summary: String,
    pub status: String,
    /// Jira status category key: `new`, `indeterminate`, `done`, or `unknown`.
    pub status_category: String,
    pub issue_type: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub updated: Option<String>,
    pub labels: Vec<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraComment {
    pub id: String,
    pub author: String,
    pub created: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraTransition {
    pub id: String,
    pub name: String,
    pub to_status: String,
}

/// The short form of an issue named by another issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraRelatedIssue {
    pub key: String,
    pub summary: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraLinkedIssue {
    /// How the open issue relates to this one, such as `blocks` or
    /// `is blocked by`.
    pub relationship: String,
    #[serde(flatten)]
    pub issue: JiraRelatedIssue,
}

/// A file attached to an issue. The content is fetched separately by `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraAttachment {
    pub id: String,
    /// The name Jira reports. It is not safe to use as a local file name.
    pub filename: String,
    pub mime_type: Option<String>,
    /// Size in bytes; 0 when Jira does not report one.
    pub size: u64,
    pub author: Option<String>,
    pub created: Option<String>,
    /// True when Jira offers a small preview image for this attachment.
    pub has_thumbnail: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraIssueDetail {
    #[serde(flatten)]
    pub summary: JiraIssueSummary,
    pub description: String,
    /// False when the Jira description uses formatting that the plain-text
    /// editor cannot write back, so saving an edit would flatten it.
    pub description_editable_losslessly: bool,
    pub reporter: Option<String>,
    pub parent_key: Option<String>,
    pub subtasks: Vec<JiraRelatedIssue>,
    pub linked_issues: Vec<JiraLinkedIssue>,
    pub attachments: Vec<JiraAttachment>,
    pub comments: Vec<JiraComment>,
    pub transitions: Vec<JiraTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIssue {
    pub project_key: String,
    pub issue_type: String,
    pub summary: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IssueUpdate {
    pub summary: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Assignee {
    Me,
    Unassigned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionOutcome {
    pub from: String,
    pub to: String,
    /// False when the issue was already in the requested status.
    pub changed: bool,
}

#[cfg(test)]
mod tests {
    use super::{normalize_project_key, IssueKey};

    #[test]
    fn issue_key_is_trimmed_and_upper_cased() {
        let key = IssueKey::parse("  abc-123 ").unwrap();

        assert_eq!(key.as_str(), "ABC-123");
        assert_eq!(key.project_key(), "ABC");
    }

    #[test]
    fn issue_key_rejects_values_that_are_not_keys() {
        for value in [
            "",
            "ABC",
            "ABC-",
            "-12",
            "ABC-12/../x",
            "AB C-1",
            "1AB-2",
            "ABC-1a",
        ] {
            assert!(
                IssueKey::parse(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn issue_key_deserialization_validates() {
        assert!(serde_json::from_str::<IssueKey>("\"abc-9\"").is_ok());
        assert!(serde_json::from_str::<IssueKey>("\"../admin\"").is_err());
    }

    #[test]
    fn project_key_is_upper_cased_and_validated() {
        assert_eq!(normalize_project_key(" abc ").unwrap(), "ABC");
        assert_eq!(normalize_project_key("AB_2").unwrap(), "AB_2");
        for value in ["", "R-B", "2AB", "AB C", "a/b"] {
            assert!(
                normalize_project_key(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }
}
