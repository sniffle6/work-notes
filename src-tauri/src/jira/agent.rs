//! Contract between the Jira chat agent and the app.
//!
//! The agent never talks to Jira. Each turn it returns JSON matching
//! `schemas/jira-agent-turn.schema.json`: a reply plus tool calls. The app
//! runs read-only calls itself and stores write calls as proposals that need
//! user approval.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::chat::{ChatMessage, ChatMessageStatus, ChatRole};
use super::types::{normalize_project_key, Assignee, IssueKey};
use crate::parser::run_structured_exec;

pub const AGENT_TURN_SCHEMA_JSON: &str =
    include_str!("../../../schemas/jira-agent-turn.schema.json");
/// Older messages are left out of the prompt to bound its size.
pub const MAX_TRANSCRIPT_MESSAGES: usize = 40;
const MAX_TOOL_RESULT_CHARS: usize = 6_000;
const DEFAULT_ISSUE_TYPE: &str = "Task";
const DEFAULT_LIST_TITLE: &str = "Agent search";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "tool",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum JiraToolCall {
    SearchIssues {
        jql: String,
    },
    GetIssue {
        issue_key: IssueKey,
    },
    CreateIssue {
        /// `None` means the project configured in Settings.
        project_key: Option<String>,
        issue_type: String,
        summary: String,
        description: String,
    },
    UpdateIssue {
        issue_key: IssueKey,
        summary: Option<String>,
        description: Option<String>,
    },
    TransitionIssue {
        issue_key: IssueKey,
        status: String,
    },
    AddComment {
        issue_key: IssueKey,
        comment: String,
    },
    AssignIssue {
        issue_key: IssueKey,
        assignee: Assignee,
    },
    /// Replaces the app's ticket list with the tickets matching `jql`.
    ShowIssues {
        title: String,
        jql: String,
    },
    /// Opens a ticket in the app's ticket pane.
    OpenIssue {
        issue_key: IssueKey,
    },
}

/// A change to what the app shows. It never changes Jira, so it needs no
/// approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ViewAction {
    ShowIssues { title: String, jql: String },
    OpenIssue { issue_key: IssueKey },
}

impl JiraToolCall {
    /// Write calls change Jira and must be approved by the user first.
    pub fn is_write(&self) -> bool {
        !matches!(self, Self::SearchIssues { .. } | Self::GetIssue { .. })
            && self.view_action().is_none()
    }

    /// The change to the app's view this call asks for, if it is a view call.
    pub fn view_action(&self) -> Option<ViewAction> {
        match self {
            Self::ShowIssues { title, jql } => Some(ViewAction::ShowIssues {
                title: title.clone(),
                jql: jql.clone(),
            }),
            Self::OpenIssue { issue_key } => Some(ViewAction::OpenIssue {
                issue_key: issue_key.clone(),
            }),
            _ => None,
        }
    }

    /// Human-readable description shown in the conversation.
    pub fn describe(&self) -> String {
        match self {
            Self::SearchIssues { jql } => format!("Search: {jql}"),
            Self::GetIssue { issue_key } => format!("Look up {issue_key}"),
            Self::CreateIssue {
                project_key,
                issue_type,
                summary,
                description,
            } => {
                let target = match project_key {
                    Some(project_key) => format!("{issue_type} in {project_key}"),
                    None => issue_type.clone(),
                };
                let mut text = format!("Create {target}: {summary}");
                if !description.trim().is_empty() {
                    text.push_str("\n\n");
                    text.push_str(description.trim());
                }
                text
            }
            Self::UpdateIssue {
                issue_key,
                summary,
                description,
            } => {
                let mut text = format!("Update {issue_key}");
                if let Some(summary) = summary {
                    text.push_str(&format!("\nSummary: {summary}"));
                }
                if let Some(description) = description {
                    text.push_str(&format!(
                        "\nDescription (replaces the current one):\n{description}"
                    ));
                }
                text
            }
            Self::TransitionIssue { issue_key, status } => {
                format!("Move {issue_key} to \"{status}\"")
            }
            Self::AddComment { issue_key, comment } => {
                format!("Comment on {issue_key}:\n{comment}")
            }
            Self::AssignIssue {
                issue_key,
                assignee: Assignee::Me,
            } => format!("Assign {issue_key} to me"),
            Self::AssignIssue {
                issue_key,
                assignee: Assignee::Unassigned,
            } => format!("Unassign {issue_key}"),
            Self::ShowIssues { title, jql } => format!("Show in ticket list: {title}\n{jql}"),
            Self::OpenIssue { issue_key } => format!("Open {issue_key}"),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTurn {
    reply: String,
    tool_calls: Vec<RawToolCall>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawToolCall {
    tool: String,
    issue_key: Option<String>,
    jql: Option<String>,
    project_key: Option<String>,
    issue_type: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    status: Option<String>,
    comment: Option<String>,
    assignee: Option<String>,
    title: Option<String>,
}

fn non_blank(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

impl RawToolCall {
    fn require(&self, value: &Option<String>, field: &str) -> Result<String, String> {
        non_blank(value).ok_or_else(|| format!("{} requires {field}", self.tool))
    }

    fn issue_key(&self) -> Result<IssueKey, String> {
        let value = self.require(&self.issue_key, "issueKey")?;
        IssueKey::parse(&value).map_err(|error| format!("{}: {error}", self.tool))
    }

    fn into_call(self) -> Result<JiraToolCall, String> {
        match self.tool.as_str() {
            "search_issues" => Ok(JiraToolCall::SearchIssues {
                jql: self.require(&self.jql, "jql")?,
            }),
            "get_issue" => Ok(JiraToolCall::GetIssue {
                issue_key: self.issue_key()?,
            }),
            "create_issue" => Ok(JiraToolCall::CreateIssue {
                project_key: non_blank(&self.project_key)
                    .map(|project_key| normalize_project_key(&project_key))
                    .transpose()
                    .map_err(|error| format!("{}: {error}", self.tool))?,
                issue_type: non_blank(&self.issue_type)
                    .unwrap_or_else(|| DEFAULT_ISSUE_TYPE.to_string()),
                summary: self.require(&self.summary, "summary")?,
                description: non_blank(&self.description).unwrap_or_default(),
            }),
            "update_issue" => {
                let summary = non_blank(&self.summary);
                let description = non_blank(&self.description);
                if summary.is_none() && description.is_none() {
                    return Err("update_issue requires summary or description".to_string());
                }
                Ok(JiraToolCall::UpdateIssue {
                    issue_key: self.issue_key()?,
                    summary,
                    description,
                })
            }
            "transition_issue" => Ok(JiraToolCall::TransitionIssue {
                issue_key: self.issue_key()?,
                status: self.require(&self.status, "status")?,
            }),
            "add_comment" => Ok(JiraToolCall::AddComment {
                issue_key: self.issue_key()?,
                comment: self.require(&self.comment, "comment")?,
            }),
            "assign_issue" => {
                let assignee = self.require(&self.assignee, "assignee")?;
                let assignee = match assignee.to_ascii_lowercase().as_str() {
                    "me" => Assignee::Me,
                    "unassigned" => Assignee::Unassigned,
                    _ => {
                        return Err(format!(
                            "assign_issue assignee must be \"me\" or \"unassigned\", not \"{assignee}\""
                        ))
                    }
                };
                Ok(JiraToolCall::AssignIssue {
                    issue_key: self.issue_key()?,
                    assignee,
                })
            }
            "show_issues" => Ok(JiraToolCall::ShowIssues {
                jql: self.require(&self.jql, "jql")?,
                title: non_blank(&self.title).unwrap_or_else(|| DEFAULT_LIST_TITLE.to_string()),
            }),
            "open_issue" => Ok(JiraToolCall::OpenIssue {
                issue_key: self.issue_key()?,
            }),
            other => Err(format!("unknown tool {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurn {
    pub reply: String,
    /// Tool calls in the order requested. A call the app cannot accept is
    /// kept as `Err(reason)` so the reason can be shown to the agent.
    pub tool_calls: Vec<Result<JiraToolCall, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentTurnError {
    #[error("agent output was not valid JSON: {0}")]
    InvalidJson(String),
    #[error("agent output did not match the turn schema: {0}")]
    SchemaValidation(String),
}

/// Validates raw agent output against the turn schema and types its calls.
pub fn parse_agent_turn(raw: &str) -> Result<AgentTurn, AgentTurnError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| AgentTurnError::InvalidJson(error.to_string()))?;
    let schema: Value = serde_json::from_str(AGENT_TURN_SCHEMA_JSON)
        .map_err(|error| AgentTurnError::SchemaValidation(format!("schema is invalid: {error}")))?;
    let validator = jsonschema::validator_for(&schema).map_err(|error| {
        AgentTurnError::SchemaValidation(format!("schema could not be compiled: {error}"))
    })?;
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if !errors.is_empty() {
        return Err(AgentTurnError::SchemaValidation(errors.join("; ")));
    }

    let turn: RawTurn = serde_json::from_value(value)
        .map_err(|error| AgentTurnError::SchemaValidation(error.to_string()))?;
    Ok(AgentTurn {
        reply: turn.reply.trim().to_string(),
        tool_calls: turn
            .tool_calls
            .into_iter()
            .map(RawToolCall::into_call)
            .collect(),
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentContext {
    pub site_url: String,
    pub project_key: String,
    /// Local date as `YYYY-MM-DD`.
    pub today: String,
    /// Issue open in the app when the user sent the message.
    pub focus_issue_key: Option<String>,
}

/// Builds the full prompt for one agent turn from the stored conversation.
pub fn build_agent_prompt(context: &AgentContext, transcript: &[ChatMessage]) -> String {
    let mut prompt = String::from(AGENT_INSTRUCTIONS);

    prompt.push_str("\nContext:\n");
    prompt.push_str(&format!("- Jira site: {}\n", or_not_set(&context.site_url)));
    prompt.push_str(&format!(
        "- Default project key: {}\n",
        or_not_set(&context.project_key)
    ));
    prompt.push_str(&format!("- Today: {}\n", context.today));
    if let Some(focus_issue_key) = &context.focus_issue_key {
        prompt.push_str(&format!("- Issue open in the app: {focus_issue_key}\n"));
    }

    prompt.push_str("\nConversation so far:\n");
    let skipped = transcript.len().saturating_sub(MAX_TRANSCRIPT_MESSAGES);
    if skipped > 0 {
        prompt.push_str("[earlier messages omitted]\n\n");
    }
    let rendered: Vec<String> = transcript[skipped..].iter().map(render_message).collect();
    prompt.push_str(&rendered.join("\n"));
    prompt
}

const AGENT_INSTRUCTIONS: &str = r###"You are the Jira assistant inside Work Notes, a desktop app. You help one engineer find, create, and update Jira tickets with as little ceremony as possible.

How this works:
- You cannot reach Jira yourself. Do not run shell commands, read files, or browse. Respond only with JSON matching the output schema.
- `reply` is shown to the user as plain text. Keep it short and concrete, and name ticket keys.
- `toolCalls` asks the app to act on Jira for you. Set every field a tool does not use to null.
- Lookups (search_issues, get_issue) run immediately. Their results are added to the conversation and you are called again. Use lookups when you need facts. Never guess ticket keys, statuses, or ticket contents.
- View calls (show_issues, open_issue) change what the app shows and run immediately. They never change Jira. The user sees a ticket list beside this chat and one open ticket.
- When the user asks to see, show, find, list, filter, or pull up tickets, use show_issues so the tickets land in the list they can click through. Do not paste a long ticket list into `reply`. After show_issues you are called again with the tickets; then say in one or two sentences what is in the list and which ticket you would start with.
- If show_issues matches nothing, try one broader search before giving up, and say what you loosened.
- Use search_issues only when you need facts to answer and the user did not ask to see the tickets.
- Changes (create_issue, update_issue, transition_issue, add_comment, assign_issue) are shown to the user as proposals. Nothing changes in Jira until the user approves each proposal. Propose a change only when the user asked for it, and say in `reply` what you are proposing.
- Do not say a change happened unless the conversation shows it was approved and applied.
- Do not invent owners, dates, or commitments the user did not state.
- When you have what you need, answer with an empty `toolCalls` list.

Tools:
- show_issues: `jql` (required), `title` (a short name for the list, for example "Important today"). Puts up to 200 matching tickets in the app's ticket list.
- open_issue: `issueKey`. Opens that ticket in the app's ticket pane.
- search_issues: `jql` (required). Returns up to 50 matching tickets to you only.
- get_issue: `issueKey`. Returns fields, description, recent comments, and the transitions available now.
- create_issue: `summary` (required), `issueType` (for example Bug, Task, Story; default Task), `description`, `projectKey` (null uses the default project).
- update_issue: `issueKey` plus `summary` and/or `description`. A description replaces the whole current description.
- transition_issue: `issueKey`, `status` (the target status or transition name, for example "In Progress").
- add_comment: `issueKey`, `comment`.
- assign_issue: `issueKey`, `assignee` ("me" or "unassigned").

Descriptions and comments are plain text: blank lines separate paragraphs, "- " starts a bullet, "## " starts a heading.

JQL reminders: `assignee = currentUser()`, `statusCategory != Done`, `project = KEY`, `text ~ "words"`, `labels = name`, `ORDER BY updated DESC`.
Reading the user's words as JQL:
- "my", "mine", "I need to": `assignee = currentUser()`.
- Leave out finished work with `statusCategory != Done` unless the user asks for it.
- Keep to the default project unless the user names another or asks across projects.
- "important", "urgent": `priority in (Highest, High)`. "today", "due": `duedate <= endOfDay()`. Combine them with OR when the user asks what needs attention, and order by `priority DESC, duedate ASC`.
- "this week", "last week", "recent": `updated >= -7d` or `created >= -7d`, whichever the user means.
"###;

fn or_not_set(value: &str) -> &str {
    if value.trim().is_empty() {
        "(not set)"
    } else {
        value
    }
}

fn render_message(message: &ChatMessage) -> String {
    let label = match (message.role, message.status) {
        (ChatRole::User, _) => "[user]",
        (ChatRole::Assistant, _) => "[assistant]",
        (ChatRole::Tool, Some(ChatMessageStatus::Error)) => "[lookup error]",
        (ChatRole::Tool, _) => "[lookup ok]",
        (ChatRole::Proposal, Some(ChatMessageStatus::Approved)) => {
            "[proposed change: approved and applied]"
        }
        (ChatRole::Proposal, Some(ChatMessageStatus::Rejected)) => {
            "[proposed change: rejected by user]"
        }
        (ChatRole::Proposal, Some(ChatMessageStatus::Failed)) => {
            "[proposed change: approved but failed]"
        }
        (ChatRole::Proposal, _) => "[proposed change: waiting for user approval]",
    };

    let mut rendered = format!("{label}\n{}\n", message.content);
    if let Some(result) = &message.result {
        rendered.push_str("Result:\n");
        if result.chars().count() > MAX_TOOL_RESULT_CHARS {
            rendered.extend(result.chars().take(MAX_TOOL_RESULT_CHARS));
            rendered.push_str("\n[truncated]\n");
        } else {
            rendered.push_str(result);
            rendered.push('\n');
        }
    }
    rendered
}

/// Runs one agent turn and returns its raw JSON output.
pub trait AgentRunner {
    fn run_turn(&self, prompt: &str) -> Result<String, String>;
}

/// Runs agent turns through local `codex exec`.
pub struct CodexAgentRunner {
    pub program: String,
    pub timeout: Duration,
}

impl AgentRunner for CodexAgentRunner {
    fn run_turn(&self, prompt: &str) -> Result<String, String> {
        run_structured_exec(&self.program, AGENT_TURN_SCHEMA_JSON, prompt, self.timeout)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::{
        build_agent_prompt, parse_agent_turn, AgentContext, AgentTurnError, JiraToolCall,
        MAX_TRANSCRIPT_MESSAGES,
    };
    use crate::jira::chat::{ChatMessage, ChatMessageStatus, ChatRole};
    use crate::jira::types::{Assignee, IssueKey};

    fn raw_call(overrides: serde_json::Value) -> serde_json::Value {
        let mut call = json!({
            "tool": "get_issue",
            "issueKey": null,
            "jql": null,
            "projectKey": null,
            "issueType": null,
            "summary": null,
            "description": null,
            "status": null,
            "comment": null,
            "assignee": null,
            "title": null,
        });
        for (name, value) in overrides.as_object().unwrap() {
            call[name] = value.clone();
        }
        call
    }

    #[test]
    fn parses_view_calls_and_names_an_untitled_list() {
        let raw = turn(vec![
            raw_call(json!({
                "tool": "show_issues", "title": " Important today ", "jql": " priority = High ",
            })),
            raw_call(json!({ "tool": "show_issues", "jql": "project = AB" })),
            raw_call(json!({ "tool": "open_issue", "issueKey": "ab-7" })),
            raw_call(json!({ "tool": "show_issues", "title": "No query" })),
            raw_call(json!({ "tool": "open_issue" })),
        ]);

        let parsed = parse_agent_turn(&raw).unwrap();

        assert_eq!(
            parsed.tool_calls,
            vec![
                Ok(JiraToolCall::ShowIssues {
                    title: "Important today".to_string(),
                    jql: "priority = High".to_string(),
                }),
                Ok(JiraToolCall::ShowIssues {
                    title: "Agent search".to_string(),
                    jql: "project = AB".to_string(),
                }),
                Ok(JiraToolCall::OpenIssue {
                    issue_key: key("AB-7")
                }),
                Err("show_issues requires jql".to_string()),
                Err("open_issue requires issueKey".to_string()),
            ]
        );
    }

    #[test]
    fn view_calls_need_no_approval_and_carry_their_view_action() {
        let show = JiraToolCall::ShowIssues {
            title: "Important today".to_string(),
            jql: "priority = High".to_string(),
        };
        let open = JiraToolCall::OpenIssue {
            issue_key: key("AB-7"),
        };

        assert!(!show.is_write());
        assert!(!open.is_write());
        assert_eq!(
            serde_json::to_value(show.view_action()).unwrap(),
            json!({ "kind": "showIssues", "title": "Important today", "jql": "priority = High" })
        );
        assert_eq!(
            serde_json::to_value(open.view_action()).unwrap(),
            json!({ "kind": "openIssue", "issueKey": "AB-7" })
        );
        assert_eq!(
            JiraToolCall::GetIssue {
                issue_key: key("AB-7")
            }
            .view_action(),
            None
        );
        assert_eq!(
            show.describe(),
            "Show in ticket list: Important today\npriority = High"
        );
        assert_eq!(open.describe(), "Open AB-7");
    }

    fn turn(calls: Vec<serde_json::Value>) -> String {
        json!({ "reply": "On it.", "toolCalls": calls }).to_string()
    }

    fn key(value: &str) -> IssueKey {
        IssueKey::parse(value).unwrap()
    }

    fn message(role: ChatRole, content: &str) -> ChatMessage {
        ChatMessage {
            id: "m".to_string(),
            thread_id: "t".to_string(),
            role,
            content: content.to_string(),
            payload: None,
            status: None,
            result: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn parses_reply_and_every_tool_kind() {
        let raw = turn(vec![
            raw_call(json!({ "tool": "search_issues", "jql": " project = AB " })),
            raw_call(json!({ "tool": "get_issue", "issueKey": "ab-7" })),
            raw_call(json!({
                "tool": "create_issue", "projectKey": "ab", "issueType": "Bug",
                "summary": " Printer jams ", "description": "Observed vs expected",
            })),
            raw_call(json!({ "tool": "update_issue", "issueKey": "AB-7", "summary": "New title" })),
            raw_call(
                json!({ "tool": "transition_issue", "issueKey": "AB-7", "status": "In Progress" }),
            ),
            raw_call(json!({ "tool": "add_comment", "issueKey": "AB-7", "comment": "Verified." })),
            raw_call(json!({ "tool": "assign_issue", "issueKey": "AB-7", "assignee": "me" })),
        ]);

        let parsed = parse_agent_turn(&raw).unwrap();

        assert_eq!(parsed.reply, "On it.");
        assert_eq!(
            parsed.tool_calls,
            vec![
                Ok(JiraToolCall::SearchIssues {
                    jql: "project = AB".to_string()
                }),
                Ok(JiraToolCall::GetIssue {
                    issue_key: key("AB-7")
                }),
                Ok(JiraToolCall::CreateIssue {
                    project_key: Some("AB".to_string()),
                    issue_type: "Bug".to_string(),
                    summary: "Printer jams".to_string(),
                    description: "Observed vs expected".to_string(),
                }),
                Ok(JiraToolCall::UpdateIssue {
                    issue_key: key("AB-7"),
                    summary: Some("New title".to_string()),
                    description: None,
                }),
                Ok(JiraToolCall::TransitionIssue {
                    issue_key: key("AB-7"),
                    status: "In Progress".to_string(),
                }),
                Ok(JiraToolCall::AddComment {
                    issue_key: key("AB-7"),
                    comment: "Verified.".to_string(),
                }),
                Ok(JiraToolCall::AssignIssue {
                    issue_key: key("AB-7"),
                    assignee: Assignee::Me,
                }),
            ]
        );
    }

    #[test]
    fn create_issue_defaults_type_project_and_description() {
        let raw = turn(vec![raw_call(
            json!({ "tool": "create_issue", "summary": "Tidy logs" }),
        )]);

        let parsed = parse_agent_turn(&raw).unwrap();

        assert_eq!(
            parsed.tool_calls,
            vec![Ok(JiraToolCall::CreateIssue {
                project_key: None,
                issue_type: "Task".to_string(),
                summary: "Tidy logs".to_string(),
                description: String::new(),
            })]
        );
    }

    #[test]
    fn unusable_calls_are_kept_as_errors_beside_valid_ones() {
        let raw = turn(vec![
            raw_call(json!({ "tool": "get_issue", "issueKey": "not a key" })),
            raw_call(json!({ "tool": "search_issues", "jql": "  " })),
            raw_call(json!({ "tool": "update_issue", "issueKey": "AB-7" })),
            raw_call(json!({ "tool": "add_comment", "issueKey": "AB-7" })),
            raw_call(json!({ "tool": "assign_issue", "issueKey": "AB-7" })),
            raw_call(json!({ "tool": "create_issue", "projectKey": "A B", "summary": "x" })),
            raw_call(json!({ "tool": "get_issue", "issueKey": "AB-9" })),
        ]);

        let parsed = parse_agent_turn(&raw).unwrap();

        assert_eq!(parsed.tool_calls.len(), 7);
        for call in &parsed.tool_calls[..6] {
            assert!(call.is_err(), "{call:?} should be rejected");
        }
        assert_eq!(
            parsed.tool_calls[1],
            Err("search_issues requires jql".to_string())
        );
        assert_eq!(
            parsed.tool_calls[6],
            Ok(JiraToolCall::GetIssue {
                issue_key: key("AB-9")
            })
        );
    }

    #[test]
    fn output_outside_the_schema_is_rejected() {
        assert!(matches!(
            parse_agent_turn("I could not do that"),
            Err(AgentTurnError::InvalidJson(_))
        ));
        assert!(matches!(
            parse_agent_turn(r#"{"reply":"hi"}"#),
            Err(AgentTurnError::SchemaValidation(_))
        ));
        let unknown_tool = turn(vec![raw_call(json!({ "tool": "delete_issue" }))]);
        assert!(matches!(
            parse_agent_turn(&unknown_tool),
            Err(AgentTurnError::SchemaValidation(_))
        ));
    }

    #[test]
    fn only_calls_that_change_jira_are_writes() {
        let reads = [
            JiraToolCall::SearchIssues {
                jql: "x".to_string(),
            },
            JiraToolCall::GetIssue {
                issue_key: key("AB-1"),
            },
        ];
        let writes = [
            JiraToolCall::CreateIssue {
                project_key: None,
                issue_type: "Bug".to_string(),
                summary: "s".to_string(),
                description: String::new(),
            },
            JiraToolCall::UpdateIssue {
                issue_key: key("AB-1"),
                summary: Some("s".to_string()),
                description: None,
            },
            JiraToolCall::TransitionIssue {
                issue_key: key("AB-1"),
                status: "Done".to_string(),
            },
            JiraToolCall::AddComment {
                issue_key: key("AB-1"),
                comment: "c".to_string(),
            },
            JiraToolCall::AssignIssue {
                issue_key: key("AB-1"),
                assignee: Assignee::Unassigned,
            },
        ];

        assert!(reads.iter().all(|call| !call.is_write()));
        assert!(writes.iter().all(JiraToolCall::is_write));
    }

    #[test]
    fn describes_calls_for_the_conversation() {
        assert_eq!(
            JiraToolCall::CreateIssue {
                project_key: Some("AB".to_string()),
                issue_type: "Bug".to_string(),
                summary: "Printer jams".to_string(),
                description: "Observed vs expected".to_string(),
            }
            .describe(),
            "Create Bug in AB: Printer jams\n\nObserved vs expected"
        );
        assert_eq!(
            JiraToolCall::CreateIssue {
                project_key: None,
                issue_type: "Task".to_string(),
                summary: "Tidy logs".to_string(),
                description: String::new(),
            }
            .describe(),
            "Create Task: Tidy logs"
        );
        assert_eq!(
            JiraToolCall::UpdateIssue {
                issue_key: key("AB-7"),
                summary: Some("New title".to_string()),
                description: Some("New body".to_string()),
            }
            .describe(),
            "Update AB-7\nSummary: New title\nDescription (replaces the current one):\nNew body"
        );
        assert_eq!(
            JiraToolCall::TransitionIssue {
                issue_key: key("AB-7"),
                status: "In Progress".to_string(),
            }
            .describe(),
            "Move AB-7 to \"In Progress\""
        );
        assert_eq!(
            JiraToolCall::AddComment {
                issue_key: key("AB-7"),
                comment: "Verified.".to_string(),
            }
            .describe(),
            "Comment on AB-7:\nVerified."
        );
        assert_eq!(
            JiraToolCall::AssignIssue {
                issue_key: key("AB-7"),
                assignee: Assignee::Me,
            }
            .describe(),
            "Assign AB-7 to me"
        );
        assert_eq!(
            JiraToolCall::AssignIssue {
                issue_key: key("AB-7"),
                assignee: Assignee::Unassigned,
            }
            .describe(),
            "Unassign AB-7"
        );
        assert_eq!(
            JiraToolCall::SearchIssues {
                jql: "project = AB".to_string()
            }
            .describe(),
            "Search: project = AB"
        );
        assert_eq!(
            JiraToolCall::GetIssue {
                issue_key: key("AB-7")
            }
            .describe(),
            "Look up AB-7"
        );
    }

    #[test]
    fn tool_calls_round_trip_through_stored_payloads() {
        let call = JiraToolCall::UpdateIssue {
            issue_key: key("AB-7"),
            summary: Some("New title".to_string()),
            description: None,
        };

        let stored = serde_json::to_string(&call).unwrap();

        assert!(stored.contains("\"tool\":\"update_issue\""), "{stored}");
        assert!(stored.contains("\"issueKey\":\"AB-7\""), "{stored}");
        assert_eq!(serde_json::from_str::<JiraToolCall>(&stored).unwrap(), call);
    }

    #[test]
    fn prompt_states_context_and_approval_rule() {
        let context = AgentContext {
            site_url: "https://example.atlassian.net".to_string(),
            project_key: "AB".to_string(),
            today: "2026-11-10".to_string(),
            focus_issue_key: Some("AB-7".to_string()),
        };

        let prompt = build_agent_prompt(&context, &[message(ChatRole::User, "what is open?")]);

        assert!(prompt.contains("Default project key: AB"), "{prompt}");
        assert!(prompt.contains("https://example.atlassian.net"), "{prompt}");
        assert!(prompt.contains("Today: 2026-11-10"), "{prompt}");
        assert!(prompt.contains("Issue open in the app: AB-7"), "{prompt}");
        for tool in [
            "search_issues",
            "get_issue",
            "create_issue",
            "update_issue",
            "transition_issue",
            "add_comment",
            "assign_issue",
        ] {
            assert!(prompt.contains(tool), "prompt should document {tool}");
        }
        assert!(prompt.contains("user approves"), "{prompt}");
        assert!(prompt.ends_with("[user]\nwhat is open?\n"), "{prompt}");
    }

    #[test]
    fn prompt_omits_unset_context() {
        let prompt = build_agent_prompt(&AgentContext::default(), &[]);

        assert!(
            prompt.contains("Default project key: (not set)"),
            "{prompt}"
        );
        assert!(!prompt.contains("Issue open in the app"), "{prompt}");
    }

    #[test]
    fn prompt_renders_lookups_and_proposal_states() {
        let mut lookup = message(ChatRole::Tool, "Search: project = AB");
        lookup.status = Some(ChatMessageStatus::Ok);
        lookup.result = Some("AB-7 | In Progress | Printer jams".to_string());
        let mut failed_lookup = message(ChatRole::Tool, "Look up AB-99");
        failed_lookup.status = Some(ChatMessageStatus::Error);
        failed_lookup.result = Some("Jira returned 404".to_string());
        let mut pending = message(ChatRole::Proposal, "Move AB-7 to \"Done\"");
        pending.status = Some(ChatMessageStatus::Pending);
        let mut approved = message(ChatRole::Proposal, "Comment on AB-7:\nVerified.");
        approved.status = Some(ChatMessageStatus::Approved);
        approved.result = Some("Comment added to AB-7".to_string());
        let mut rejected = message(ChatRole::Proposal, "Unassign AB-7");
        rejected.status = Some(ChatMessageStatus::Rejected);

        let prompt = build_agent_prompt(
            &AgentContext::default(),
            &[
                message(ChatRole::User, "close it out"),
                message(ChatRole::Assistant, "Checking."),
                lookup,
                failed_lookup,
                pending,
                approved,
                rejected,
            ],
        );

        assert!(prompt.contains("[assistant]\nChecking.\n"), "{prompt}");
        assert!(
            prompt.contains(
                "[lookup ok]\nSearch: project = AB\nResult:\nAB-7 | In Progress | Printer jams\n"
            ),
            "{prompt}"
        );
        assert!(
            prompt.contains("[lookup error]\nLook up AB-99\nResult:\nJira returned 404\n"),
            "{prompt}"
        );
        assert!(
            prompt
                .contains("[proposed change: waiting for user approval]\nMove AB-7 to \"Done\"\n"),
            "{prompt}"
        );
        assert!(
            prompt.contains("[proposed change: approved and applied]\nComment on AB-7:\nVerified.\nResult:\nComment added to AB-7\n"),
            "{prompt}"
        );
        assert!(
            prompt.contains("[proposed change: rejected by user]\nUnassign AB-7\n"),
            "{prompt}"
        );
    }

    #[test]
    fn prompt_bounds_transcript_and_lookup_size() {
        let mut transcript: Vec<ChatMessage> = (0..MAX_TRANSCRIPT_MESSAGES + 5)
            .map(|index| message(ChatRole::User, &format!("message-{index}")))
            .collect();
        let mut lookup = message(ChatRole::Tool, "Look up AB-1");
        lookup.status = Some(ChatMessageStatus::Ok);
        lookup.result = Some("^".repeat(20_000));
        transcript.push(lookup);

        let prompt = build_agent_prompt(&AgentContext::default(), &transcript);

        assert!(
            !prompt.contains("message-5\n"),
            "oldest messages are dropped"
        );
        assert!(prompt.contains("message-6\n"));
        assert!(prompt.contains("[earlier messages omitted]"));
        assert!(prompt.contains("[truncated]"));
        assert_eq!(prompt.matches('^').count(), 6_000);
    }
}
