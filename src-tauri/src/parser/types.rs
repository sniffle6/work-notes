use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserResult {
    pub cleaned_text: String,
    pub summary: String,
    pub tags: Vec<ParsedTag>,
    pub action_items: Vec<ParsedActionItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTag {
    pub kind: TagKind,
    pub name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedActionItem {
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub confidence: f32,
    pub requires_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TagKind {
    Person,
    Project,
    Topic,
    Urgency,
    Category,
    Custom,
}

pub trait ParserProvider {
    fn parse(&self, input: &str) -> Result<ParserResult, ParserError>;
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("parser provider failed: {0}")]
    Provider(String),
    #[error("parser result was invalid: {0}")]
    InvalidResult(String),
}
