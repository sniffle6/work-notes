use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn parse(value: &str) -> Result<Self, DomainError> {
                Uuid::parse_str(value)
                    .map(Self)
                    .map_err(|source| DomainError::InvalidId {
                        id_type: stringify!($name),
                        value: value.to_string(),
                        source,
                    })
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = DomainError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

id_type!(NoteId);
id_type!(TagId);
id_type!(ActionItemId);
id_type!(ParseJobId);
id_type!(ParseRunId);

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid {id_type} `{value}`")]
    InvalidId {
        id_type: &'static str,
        value: String,
        source: uuid::Error,
    },
    #[error("invalid {field} `{value}`")]
    InvalidEnum { field: &'static str, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    Queued,
    Parsing,
    Parsed,
    Failed,
}

impl ParseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Parsing => "parsing",
            Self::Parsed => "parsed",
            Self::Failed => "failed",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "queued" => Ok(Self::Queued),
            "parsing" => Ok(Self::Parsing),
            "parsed" => Ok(Self::Parsed),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::InvalidEnum {
                field: "parse_status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    None,
    NeedsReview,
    Reviewed,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NeedsReview => "needs_review",
            Self::Reviewed => "reviewed",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "none" => Ok(Self::None),
            "needs_review" => Ok(Self::NeedsReview),
            "reviewed" => Ok(Self::Reviewed),
            _ => Err(DomainError::InvalidEnum {
                field: "review_status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    Person,
    Project,
    Topic,
    Urgency,
    Category,
    Custom,
}

impl TagKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Project => "project",
            Self::Topic => "topic",
            Self::Urgency => "urgency",
            Self::Category => "category",
            Self::Custom => "custom",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "person" => Ok(Self::Person),
            "project" => Ok(Self::Project),
            "topic" => Ok(Self::Topic),
            "urgency" => Ok(Self::Urgency),
            "category" => Ok(Self::Category),
            "custom" => Ok(Self::Custom),
            _ => Err(DomainError::InvalidEnum {
                field: "tag_kind",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Suggested,
    Accepted,
    Dismissed,
    Done,
}

impl ActionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Suggested => "suggested",
            Self::Accepted => "accepted",
            Self::Dismissed => "dismissed",
            Self::Done => "done",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "suggested" => Ok(Self::Suggested),
            "accepted" => Ok(Self::Accepted),
            "dismissed" => Ok(Self::Dismissed),
            "done" => Ok(Self::Done),
            _ => Err(DomainError::InvalidEnum {
                field: "action_status",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowupState {
    Open,
    Waiting,
    Blocked,
}

impl FollowupState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Waiting => "waiting",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "open" => Ok(Self::Open),
            "waiting" => Ok(Self::Waiting),
            "blocked" => Ok(Self::Blocked),
            _ => Err(DomainError::InvalidEnum {
                field: "followup_state",
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub raw_text: String,
    pub cleaned_text: Option<String>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub capture_source: String,
    pub parse_status: ParseStatus,
    pub review_status: ReviewStatus,
    pub is_archived: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub cleaned_edited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteListItem {
    pub id: NoteId,
    pub title: String,
    pub raw_text: String,
    pub cleaned_text: Option<String>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parse_status: ParseStatus,
    pub review_status: ReviewStatus,
    pub is_archived: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub tag_count: u32,
    pub action_item_count: u32,
    pub suggested_action_item_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxFilters {
    pub parse_status: Option<ParseStatus>,
    pub review_status: Option<ReviewStatus>,
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
    pub query: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub include_completed: bool,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub kind: TagKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagAssignment {
    pub tag: Tag,
    pub source: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: ActionItemId,
    pub note_id: NoteId,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReviewItem {
    pub id: ActionItemId,
    pub note_id: NoteId,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FollowupItem {
    pub id: ActionItemId,
    pub note_id: NoteId,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
    pub tags: Vec<TagAssignment>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseJob {
    pub id: ParseJobId,
    pub note_id: NoteId,
    pub status: ParseStatus,
    pub attempt_count: u32,
    pub last_error: Option<String>,
    pub feedback: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseRun {
    pub id: ParseRunId,
    pub note_id: NoteId,
    pub provider: String,
    pub prompt_version: String,
    pub raw_response: String,
    pub parsed_json: String,
    pub feedback: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::{DomainError, FollowupState};

    #[test]
    fn followup_state_round_trips_db_values() {
        assert_eq!(FollowupState::from_db("open").unwrap(), FollowupState::Open);
        assert_eq!(
            FollowupState::from_db("waiting").unwrap(),
            FollowupState::Waiting
        );
        assert_eq!(
            FollowupState::from_db("blocked").unwrap(),
            FollowupState::Blocked
        );
        assert!(matches!(
            FollowupState::from_db("done"),
            Err(DomainError::InvalidEnum {
                field: "followup_state",
                value
            }) if value == "done"
        ));
    }
}
