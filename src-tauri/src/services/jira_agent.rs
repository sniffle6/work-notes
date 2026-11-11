//! Runs Jira chat turns: stores the conversation, executes the agent's
//! read-only lookups and view changes, and holds its requested changes to
//! Jira until the user approves.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::jira::agent::{
    build_agent_prompt, parse_agent_turn, AgentContext, AgentRunner, JiraToolCall, ViewAction,
};
use crate::jira::chat::{ChatMessage, ChatMessageStatus, ChatRole, NewChatMessage};
use crate::jira::client::DEFAULT_SEARCH_LIMIT;
use crate::jira::transitions::describe_transition;
use crate::jira::{Assignee, IssueUpdate, JiraError, JiraOps, NewIssue};
use crate::repositories::JiraChatRepository;

use super::{ServiceError, ServiceResult};

/// Agent turns allowed for one user message. Each turn after the first is
/// triggered by lookup results.
pub const MAX_AGENT_TURNS: usize = 6;
pub const NEW_THREAD_TITLE: &str = "New chat";
const MAX_THREAD_TITLE_CHARS: usize = 60;
const MAX_USER_MESSAGE_CHARS: usize = 8_000;
const AGENT_SEARCH_LIMIT: usize = 50;
const MAX_COMMENTS_IN_LOOKUP: usize = 10;

/// Tracks which conversations have an agent run in progress, so a thread
/// never has two runs interleaving messages.
#[derive(Clone, Default)]
pub struct AgentBusyThreads {
    threads: Arc<Mutex<HashSet<String>>>,
}

impl AgentBusyThreads {
    /// Marks a thread busy until the returned guard is dropped. Returns
    /// `None` when the thread is already busy.
    pub fn try_claim(&self, thread_id: &str) -> Option<AgentBusyGuard> {
        let mut threads = self
            .threads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        threads
            .insert(thread_id.to_string())
            .then(|| AgentBusyGuard {
                threads: Arc::clone(&self.threads),
                thread_id: thread_id.to_string(),
            })
    }
}

pub struct AgentBusyGuard {
    threads: Arc<Mutex<HashSet<String>>>,
    thread_id: String,
}

impl Drop for AgentBusyGuard {
    fn drop(&mut self) {
        self.threads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.thread_id);
    }
}

/// Reported while an agent run is in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentProgress {
    /// New messages were stored in the thread.
    MessagesStored,
    /// The agent changed what the app should show.
    View(ViewAction),
}

#[derive(Clone)]
pub struct JiraAgentService {
    chat: JiraChatRepository,
}

impl JiraAgentService {
    pub fn new(chat: JiraChatRepository) -> Self {
        Self { chat }
    }

    /// Stores the user's message and runs the agent until it stops asking
    /// for lookups or [`MAX_AGENT_TURNS`] is reached.
    ///
    /// Lookups and view calls run against `jira` immediately. Requested
    /// changes are stored as pending proposals and are not sent to Jira.
    /// `on_progress` is called for each view change as it happens and after
    /// each batch of stored messages. Returns the whole thread.
    ///
    /// Fails with `InvalidInput` for blank or oversized text, `NotFound` for
    /// an unknown thread, and `Agent` when the agent run or its output is
    /// unusable; messages stored before a failure are kept.
    pub fn send_message(
        &self,
        thread_id: &str,
        text: &str,
        context: &AgentContext,
        runner: &dyn AgentRunner,
        jira: &dyn JiraOps,
        on_progress: &dyn Fn(AgentProgress),
    ) -> ServiceResult<Vec<ChatMessage>> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ServiceError::InvalidInput("message text is required"));
        }
        if text.chars().count() > MAX_USER_MESSAGE_CHARS {
            return Err(ServiceError::InvalidInput("message text is too long"));
        }
        let thread = self
            .chat
            .get_thread(thread_id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "chat thread",
                id: thread_id.to_string(),
            })?;

        self.chat
            .add_message(thread_id, NewChatMessage::text(ChatRole::User, text))?;
        if thread.title == NEW_THREAD_TITLE {
            self.chat.rename_thread(thread_id, &thread_title(text))?;
        }

        let mut awaiting_agent = true;
        for _ in 0..MAX_AGENT_TURNS {
            let transcript = self.chat.list_messages(thread_id)?;
            let raw = runner
                .run_turn(&build_agent_prompt(context, &transcript))
                .map_err(ServiceError::Agent)?;
            let turn =
                parse_agent_turn(&raw).map_err(|error| ServiceError::Agent(error.to_string()))?;

            if !turn.reply.is_empty() {
                self.chat.add_message(
                    thread_id,
                    NewChatMessage::text(ChatRole::Assistant, turn.reply.as_str()),
                )?;
            } else if turn.tool_calls.is_empty() {
                self.chat.add_message(
                    thread_id,
                    NewChatMessage::text(ChatRole::Assistant, "(The agent returned no reply.)"),
                )?;
            }

            // The agent is called again only when this turn produced results
            // it has not seen: lookup output or a rejected request.
            awaiting_agent = false;
            for call in turn.tool_calls {
                let message = match call {
                    Err(reason) => {
                        awaiting_agent = true;
                        NewChatMessage {
                            role: ChatRole::Tool,
                            content: "Unusable request from the agent".to_string(),
                            payload: None,
                            status: Some(ChatMessageStatus::Error),
                            result: Some(reason),
                        }
                    }
                    Ok(call) if call.is_write() => NewChatMessage {
                        role: ChatRole::Proposal,
                        content: call.describe(),
                        payload: Some(serde_json::to_string(&call)?),
                        status: Some(ChatMessageStatus::Pending),
                        result: None,
                    },
                    Ok(call) if call.view_action().is_some() => {
                        let (status, result) = match run_view_call(&call, jira) {
                            Ok(outcome) => {
                                // Reported now so the app updates while the
                                // agent is still writing its follow-up.
                                on_progress(AgentProgress::View(outcome.action));
                                awaiting_agent |= outcome.agent_should_see_result;
                                (ChatMessageStatus::Ok, outcome.result)
                            }
                            Err(error) => {
                                awaiting_agent = true;
                                (ChatMessageStatus::Error, error.to_string())
                            }
                        };
                        NewChatMessage {
                            role: ChatRole::Tool,
                            content: call.describe(),
                            payload: Some(serde_json::to_string(&call)?),
                            status: Some(status),
                            result: Some(result),
                        }
                    }
                    Ok(call) => {
                        awaiting_agent = true;
                        let (status, result) = match run_lookup(&call, jira) {
                            Ok(result) => (ChatMessageStatus::Ok, result),
                            Err(error) => (ChatMessageStatus::Error, error.to_string()),
                        };
                        NewChatMessage {
                            role: ChatRole::Tool,
                            content: call.describe(),
                            payload: Some(serde_json::to_string(&call)?),
                            status: Some(status),
                            result: Some(result),
                        }
                    }
                };
                self.chat.add_message(thread_id, message)?;
            }
            on_progress(AgentProgress::MessagesStored);

            if !awaiting_agent {
                break;
            }
        }

        if awaiting_agent {
            self.chat.add_message(
                thread_id,
                NewChatMessage::text(
                    ChatRole::Assistant,
                    format!(
                        "I stopped after {MAX_AGENT_TURNS} rounds of lookups. Ask me to continue if you need more."
                    ),
                ),
            )?;
            on_progress(AgentProgress::MessagesStored);
        }

        Ok(self.chat.list_messages(thread_id)?)
    }

    /// Runs an approved proposal against Jira and records the outcome.
    ///
    /// A Jira failure is recorded on the proposal as `Failed` and is not an
    /// error. Fails with `InvalidInput` when the message is not a pending
    /// proposal, so a change can never run twice.
    pub fn approve_proposal(
        &self,
        message_id: &str,
        default_project_key: &str,
        jira: &dyn JiraOps,
    ) -> ServiceResult<Vec<ChatMessage>> {
        let message = self.pending_proposal(message_id)?;
        let call: JiraToolCall = serde_json::from_str(message.payload.as_deref().unwrap_or(""))
            .map_err(|_| ServiceError::Agent("the stored change could not be read".to_string()))?;

        // Claim before calling Jira so a second approval cannot run it again.
        if !self
            .chat
            .resolve_pending_proposal(message_id, ChatMessageStatus::Approved)?
        {
            return Err(ALREADY_RESOLVED);
        }
        let (status, result) = match apply_change(&call, default_project_key, jira) {
            Ok(result) => (ChatMessageStatus::Approved, result),
            Err(error) => (ChatMessageStatus::Failed, error.to_string()),
        };
        self.chat.finish_proposal(message_id, status, &result)?;

        Ok(self.chat.list_messages(&message.thread_id)?)
    }

    /// Declines a pending proposal. Nothing is sent to Jira.
    pub fn reject_proposal(&self, message_id: &str) -> ServiceResult<Vec<ChatMessage>> {
        let message = self.pending_proposal(message_id)?;
        if !self
            .chat
            .resolve_pending_proposal(message_id, ChatMessageStatus::Rejected)?
        {
            return Err(ALREADY_RESOLVED);
        }
        Ok(self.chat.list_messages(&message.thread_id)?)
    }

    fn pending_proposal(&self, message_id: &str) -> ServiceResult<ChatMessage> {
        let message = self
            .chat
            .get_message(message_id)?
            .ok_or_else(|| ServiceError::NotFound {
                entity: "chat message",
                id: message_id.to_string(),
            })?;
        if message.role != ChatRole::Proposal {
            return Err(ServiceError::InvalidInput(
                "only a proposed change can be approved or rejected",
            ));
        }
        if message.status != Some(ChatMessageStatus::Pending) {
            return Err(ALREADY_RESOLVED);
        }
        Ok(message)
    }
}

const ALREADY_RESOLVED: ServiceError =
    ServiceError::InvalidInput("this change was already approved or rejected");

fn thread_title(first_message: &str) -> String {
    first_message
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(MAX_THREAD_TITLE_CHARS)
        .collect()
}

fn summary_line(issue: &crate::jira::JiraIssueSummary) -> String {
    format!(
        "{} | {} | {} | {} | {} | {}",
        issue.key,
        issue.status,
        issue.issue_type,
        issue.priority.as_deref().unwrap_or("No priority"),
        issue.assignee.as_deref().unwrap_or("Unassigned"),
        issue.summary
    )
}

struct ViewCallOutcome {
    action: ViewAction,
    result: String,
    /// The result holds tickets the agent has not seen yet.
    agent_should_see_result: bool,
}

/// Checks a view call against Jira and describes what the app now shows.
///
/// A ticket search runs here first, so a query Jira rejects never reaches
/// the app's ticket list.
fn run_view_call(call: &JiraToolCall, jira: &dyn JiraOps) -> Result<ViewCallOutcome, JiraError> {
    let action = call.view_action().ok_or_else(|| {
        JiraError::InvalidInput("this request does not change the view".to_string())
    })?;
    match &action {
        ViewAction::ShowIssues { title, jql } => {
            let issues = jira.search(jql, DEFAULT_SEARCH_LIMIT)?;
            let result = if issues.is_empty() {
                "No tickets matched. The ticket list is empty.".to_string()
            } else {
                let mut lines = vec![format!(
                    "The ticket list now shows {} {} as \"{title}\":",
                    issues.len(),
                    if issues.len() == 1 {
                        "ticket"
                    } else {
                        "tickets"
                    }
                )];
                lines.extend(issues.iter().take(AGENT_SEARCH_LIMIT).map(summary_line));
                if issues.len() > AGENT_SEARCH_LIMIT {
                    lines.push(format!(
                        "(and {} more in the list)",
                        issues.len() - AGENT_SEARCH_LIMIT
                    ));
                }
                lines.join("\n")
            };
            Ok(ViewCallOutcome {
                action,
                result,
                agent_should_see_result: true,
            })
        }
        ViewAction::OpenIssue { issue_key } => Ok(ViewCallOutcome {
            result: format!("{issue_key} is open in the ticket pane."),
            action,
            agent_should_see_result: false,
        }),
    }
}

/// Runs a read-only tool call and renders its result as text.
pub fn run_lookup(call: &JiraToolCall, jira: &dyn JiraOps) -> Result<String, JiraError> {
    match call {
        JiraToolCall::SearchIssues { jql } => {
            let issues = jira.search(jql, AGENT_SEARCH_LIMIT)?;
            if issues.is_empty() {
                return Ok("No tickets matched.".to_string());
            }
            let mut lines = vec![format!("{} tickets:", issues.len())];
            lines.extend(issues.iter().map(summary_line));
            Ok(lines.join("\n"))
        }
        JiraToolCall::GetIssue { issue_key } => {
            let issue = jira.get_issue(issue_key)?;
            let mut lines = vec![
                summary_line(&issue.summary),
                format!("URL: {}", issue.summary.url),
            ];
            if let Some(reporter) = &issue.reporter {
                lines.push(format!("Reporter: {reporter}"));
            }
            if let Some(parent_key) = &issue.parent_key {
                lines.push(format!("Parent: {parent_key}"));
            }
            let transitions: Vec<String> =
                issue.transitions.iter().map(describe_transition).collect();
            lines.push(format!(
                "Available transitions: {}",
                if transitions.is_empty() {
                    "(none)".to_string()
                } else {
                    transitions.join(", ")
                }
            ));
            lines.push(String::new());
            lines.push("Description:".to_string());
            lines.push(if issue.description.trim().is_empty() {
                "(empty)".to_string()
            } else {
                issue.description.trim().to_string()
            });

            let total = issue.comments.len();
            let shown = total.min(MAX_COMMENTS_IN_LOOKUP);
            if total > 0 {
                lines.push(String::new());
                lines.push(if shown < total {
                    format!("Comments (showing last {shown} of {total}):")
                } else {
                    format!("Comments ({total}):")
                });
                for comment in &issue.comments[total - shown..] {
                    lines.push(format!("--- {}, {} ---", comment.author, comment.created));
                    lines.push(comment.body.trim().to_string());
                }
            }
            Ok(lines.join("\n"))
        }
        _ => Err(JiraError::InvalidInput(
            "this request changes Jira and needs approval".to_string(),
        )),
    }
}

/// Applies a write tool call to Jira and describes what changed.
pub fn apply_change(
    call: &JiraToolCall,
    default_project_key: &str,
    jira: &dyn JiraOps,
) -> Result<String, JiraError> {
    match call {
        JiraToolCall::CreateIssue {
            project_key,
            issue_type,
            summary,
            description,
        } => {
            let project_key = project_key.as_deref().unwrap_or(default_project_key).trim();
            if project_key.is_empty() {
                return Err(JiraError::NotConfigured(
                    "set the Jira project key in Settings > Jira before creating tickets"
                        .to_string(),
                ));
            }
            let key = jira.create_issue(&NewIssue {
                project_key: project_key.to_string(),
                issue_type: issue_type.clone(),
                summary: summary.clone(),
                description: description.clone(),
            })?;
            Ok(format!("Created {key}: {}", jira.browse_url(&key)))
        }
        JiraToolCall::UpdateIssue {
            issue_key,
            summary,
            description,
        } => {
            jira.update_issue(
                issue_key,
                &IssueUpdate {
                    summary: summary.clone(),
                    description: description.clone(),
                },
            )?;
            Ok(format!("Updated {issue_key}."))
        }
        JiraToolCall::TransitionIssue { issue_key, status } => {
            let outcome = jira.transition_to_status(issue_key, status)?;
            Ok(if outcome.changed {
                format!("{issue_key} moved from {} to {}.", outcome.from, outcome.to)
            } else {
                format!("{issue_key} is already {}.", outcome.to)
            })
        }
        JiraToolCall::AddComment { issue_key, comment } => {
            jira.add_comment(issue_key, comment)?;
            Ok(format!("Comment added to {issue_key}."))
        }
        JiraToolCall::AssignIssue {
            issue_key,
            assignee,
        } => {
            jira.assign(issue_key, *assignee)?;
            Ok(match assignee {
                Assignee::Me => format!("{issue_key} assigned to you."),
                Assignee::Unassigned => format!("{issue_key} unassigned."),
            })
        }
        JiraToolCall::SearchIssues { .. }
        | JiraToolCall::GetIssue { .. }
        | JiraToolCall::ShowIssues { .. }
        | JiraToolCall::OpenIssue { .. } => Err(JiraError::InvalidInput(
            "this request does not change Jira".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use serde_json::json;

    use super::{apply_change, run_lookup, AgentProgress, JiraAgentService, MAX_AGENT_TURNS};
    use crate::db::Database;
    use crate::jira::agent::{AgentContext, AgentRunner, JiraToolCall, ViewAction};
    use crate::jira::chat::{ChatMessage, ChatMessageStatus, ChatRole};
    use crate::jira::{
        Assignee, IssueKey, IssueUpdate, JiraComment, JiraError, JiraIssueDetail, JiraIssueSummary,
        JiraOps, JiraTransition, JiraUser, NewIssue, TransitionOutcome,
    };
    use crate::repositories::JiraChatRepository;
    use crate::services::ServiceError;

    struct FakeRunner {
        outputs: RefCell<VecDeque<Result<String, String>>>,
        prompts: RefCell<Vec<String>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<Result<String, String>>) -> Self {
            Self {
                outputs: RefCell::new(outputs.into()),
                prompts: RefCell::new(Vec::new()),
            }
        }

        fn prompts(&self) -> Vec<String> {
            self.prompts.borrow().clone()
        }
    }

    impl AgentRunner for FakeRunner {
        fn run_turn(&self, prompt: &str) -> Result<String, String> {
            self.prompts.borrow_mut().push(prompt.to_string());
            self.outputs
                .borrow_mut()
                .pop_front()
                .expect("no fake agent output queued")
        }
    }

    #[derive(Default)]
    struct FakeJira {
        calls: RefCell<Vec<String>>,
        fail_with: RefCell<Option<JiraError>>,
    }

    impl FakeJira {
        fn failing(error: JiraError) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail_with: RefCell::new(Some(error)),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }

        fn record(&self, call: String) -> Result<(), JiraError> {
            self.calls.borrow_mut().push(call);
            match self.fail_with.borrow().clone() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    fn key(value: &str) -> IssueKey {
        IssueKey::parse(value).unwrap()
    }

    fn summary(issue_key: &str, title: &str) -> JiraIssueSummary {
        JiraIssueSummary {
            key: key(issue_key),
            summary: title.to_string(),
            status: "In Progress".to_string(),
            status_category: "indeterminate".to_string(),
            issue_type: "Bug".to_string(),
            priority: Some("High".to_string()),
            assignee: None,
            updated: None,
            labels: Vec::new(),
            url: format!("https://example.atlassian.net/browse/{issue_key}"),
        }
    }

    impl JiraOps for FakeJira {
        fn myself(&self) -> Result<JiraUser, JiraError> {
            self.record("myself".to_string())?;
            Ok(JiraUser {
                account_id: "acc-1".to_string(),
                display_name: "Sam Dev".to_string(),
            })
        }

        fn search(&self, jql: &str, limit: usize) -> Result<Vec<JiraIssueSummary>, JiraError> {
            self.record(format!("search {jql} limit={limit}"))?;
            if jql.contains("nothing") {
                return Ok(Vec::new());
            }
            Ok(vec![
                summary("AB-7", "Printer jams"),
                JiraIssueSummary {
                    assignee: Some("Maya Lopez".to_string()),
                    priority: None,
                    ..summary("AB-8", "Bill acceptor")
                },
            ])
        }

        fn get_issue(&self, issue_key: &IssueKey) -> Result<JiraIssueDetail, JiraError> {
            self.record(format!("get {issue_key}"))?;
            Ok(JiraIssueDetail {
                summary: summary(issue_key.as_str(), "Printer jams"),
                description: "Jams after reboot.".to_string(),
                description_editable_losslessly: true,
                reporter: Some("Rina Q".to_string()),
                parent_key: Some("AB-1".to_string()),
                subtasks: Vec::new(),
                linked_issues: Vec::new(),
                comments: (1..=12)
                    .map(|index| JiraComment {
                        id: index.to_string(),
                        author: "Maya Lopez".to_string(),
                        created: format!("2026-11-{index:02}"),
                        body: format!("comment {index}"),
                    })
                    .collect(),
                transitions: vec![JiraTransition {
                    id: "21".to_string(),
                    name: "Dev Complete".to_string(),
                    to_status: "Dev Complete".to_string(),
                }],
            })
        }

        fn create_issue(&self, issue: &NewIssue) -> Result<IssueKey, JiraError> {
            self.record(format!(
                "create {} {} {} | {}",
                issue.project_key, issue.issue_type, issue.summary, issue.description
            ))?;
            Ok(key("AB-12"))
        }

        fn update_issue(
            &self,
            issue_key: &IssueKey,
            update: &IssueUpdate,
        ) -> Result<(), JiraError> {
            self.record(format!(
                "update {issue_key} summary={:?} description={:?}",
                update.summary, update.description
            ))
        }

        fn add_comment(&self, issue_key: &IssueKey, text: &str) -> Result<(), JiraError> {
            self.record(format!("comment {issue_key} {text}"))
        }

        fn apply_transition(
            &self,
            issue_key: &IssueKey,
            transition_id: &str,
        ) -> Result<(), JiraError> {
            self.record(format!("apply {issue_key} {transition_id}"))
        }

        fn transition_to_status(
            &self,
            issue_key: &IssueKey,
            status: &str,
        ) -> Result<TransitionOutcome, JiraError> {
            self.record(format!("transition {issue_key} {status}"))?;
            Ok(TransitionOutcome {
                from: "In Progress".to_string(),
                to: status.to_string(),
                changed: status != "In Progress",
            })
        }

        fn assign(&self, issue_key: &IssueKey, assignee: Assignee) -> Result<(), JiraError> {
            self.record(format!("assign {issue_key} {assignee:?}"))
        }

        fn issue_types(&self, project_key: &str) -> Result<Vec<String>, JiraError> {
            self.record(format!("types {project_key}"))?;
            Ok(vec!["Bug".to_string()])
        }

        fn browse_url(&self, issue_key: &IssueKey) -> String {
            format!("https://example.atlassian.net/browse/{issue_key}")
        }
    }

    fn raw_call(overrides: serde_json::Value) -> serde_json::Value {
        let mut call = json!({
            "tool": "get_issue", "issueKey": null, "jql": null, "projectKey": null,
            "issueType": null, "summary": null, "description": null, "status": null,
            "comment": null, "assignee": null, "title": null,
        });
        for (name, value) in overrides.as_object().unwrap() {
            call[name] = value.clone();
        }
        call
    }

    fn turn(reply: &str, calls: Vec<serde_json::Value>) -> Result<String, String> {
        Ok(json!({ "reply": reply, "toolCalls": calls }).to_string())
    }

    struct Fixture {
        service: JiraAgentService,
        chat: JiraChatRepository,
        thread_id: String,
    }

    fn fixture() -> Fixture {
        let chat = JiraChatRepository::new(Database::in_memory().expect("create database"));
        let thread_id = chat.create_thread("New chat").unwrap().id;
        Fixture {
            service: JiraAgentService::new(chat.clone()),
            chat,
            thread_id,
        }
    }

    impl Fixture {
        fn send(
            &self,
            text: &str,
            runner: &FakeRunner,
            jira: &FakeJira,
        ) -> Result<Vec<ChatMessage>, ServiceError> {
            self.service.send_message(
                &self.thread_id,
                text,
                &AgentContext {
                    project_key: "AB".to_string(),
                    ..AgentContext::default()
                },
                runner,
                jira,
                &|_| {},
            )
        }
    }

    fn roles(messages: &[ChatMessage]) -> Vec<ChatRole> {
        messages.iter().map(|message| message.role).collect()
    }

    /// Runs one message through real Codex against a real Jira site and
    /// returns the thread plus the progress events. Lookups and view calls are
    /// read-only, and proposed changes are never approved.
    fn live_agent_run(text: &str) -> (Vec<ChatMessage>, Vec<AgentProgress>) {
        use crate::jira::agent::CodexAgentRunner;
        use crate::jira::transport::{resolve_cloud_id, GatewayTransport};
        use crate::jira::JiraClient;
        use crate::parser::DEFAULT_CODEX_PROGRAM;
        use crate::services::jira::{resolve_connection, JiraEnvironment};
        use crate::services::settings::AppSettings;

        let settings = AppSettings {
            jira_site_url: std::env::var("WORK_NOTES_JIRA_SITE_URL").expect("site url"),
            ..AppSettings::default()
        };
        let (connection, _) = resolve_connection(
            &settings,
            &JiraEnvironment::from_process(),
            resolve_cloud_id,
        )
        .expect("resolve connection");
        let client = JiraClient::new(
            GatewayTransport::new(connection.credentials, &connection.cloud_id).expect("transport"),
            connection.site_url.clone(),
        );
        let runner = CodexAgentRunner {
            program: DEFAULT_CODEX_PROGRAM.to_string(),
            timeout: std::time::Duration::from_secs(180),
        };
        let fixture = fixture();
        let progress = RefCell::new(Vec::new());

        let messages = fixture
            .service
            .send_message(
                &fixture.thread_id,
                text,
                &AgentContext {
                    site_url: connection.site_url,
                    project_key: std::env::var("WORK_NOTES_JIRA_PROJECT_KEY").unwrap_or_default(),
                    today: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    ..AgentContext::default()
                },
                &runner,
                &client,
                &|event| progress.borrow_mut().push(event),
            )
            .expect("agent run");

        for message in &messages {
            println!(
                "{:?} {:?} content_chars={} result_chars={}",
                message.role,
                message.status,
                message.content.chars().count(),
                message
                    .result
                    .as_deref()
                    .map_or(0, |result| result.chars().count())
            );
        }
        (messages, progress.into_inner())
    }

    /// Manual check that the agent answers a question from lookups. Run with:
    /// `WORK_NOTES_JIRA_SITE_URL=https://team.atlassian.net cargo test live_agent -- --ignored --nocapture`
    #[test]
    #[ignore = "runs codex exec and talks to a real Jira site"]
    fn live_agent_answers_with_real_codex_and_jira() {
        let (messages, _) = live_agent_run(
            "How many open tickets are assigned to me? Answer with the count and the three most recently updated keys.",
        );

        assert!(messages.iter().any(|message| message.role == ChatRole::Tool
            && message.status == Some(ChatMessageStatus::Ok)));
        let last = messages.last().expect("messages");
        assert_eq!(last.role, ChatRole::Assistant);
        println!(
            "final reply: {}",
            last.content.chars().take(300).collect::<String>()
        );
    }

    /// Manual check that a request to see tickets drives the app's ticket
    /// list. `WORK_NOTES_JIRA_PROJECT_KEY` sets the default project.
    #[test]
    #[ignore = "runs codex exec and talks to a real Jira site"]
    fn live_agent_shows_tickets_in_the_list() {
        let (messages, progress) =
            live_agent_run("Show me any important tickets I need to take care of today.");

        let shown: Vec<&ViewAction> = progress
            .iter()
            .filter_map(|event| match event {
                AgentProgress::View(action) => Some(action),
                AgentProgress::MessagesStored => None,
            })
            .collect();
        for action in &shown {
            if let ViewAction::ShowIssues { title, jql } = action {
                println!("list: {title} | {jql}");
            }
        }
        assert!(
            shown
                .iter()
                .any(|action| matches!(action, ViewAction::ShowIssues { .. })),
            "the agent should put tickets in the list"
        );
        let last = messages.last().expect("messages");
        assert_eq!(last.role, ChatRole::Assistant);
        println!(
            "final reply: {}",
            last.content.chars().take(300).collect::<String>()
        );
    }

    #[test]
    fn a_thread_is_busy_until_its_guard_is_dropped() {
        let busy = super::AgentBusyThreads::default();

        let guard = busy.try_claim("thread-1").expect("first claim");
        assert!(busy.try_claim("thread-1").is_none());
        assert!(busy.clone().try_claim("thread-1").is_none());
        assert!(busy.try_claim("thread-2").is_some());

        drop(guard);
        assert!(busy.try_claim("thread-1").is_some());
    }

    #[test]
    fn plain_reply_is_stored_and_names_the_thread() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![turn("Nothing is assigned to you.", vec![])]);
        let jira = FakeJira::default();

        let messages = fixture
            .send("  what is on my plate today?\nthanks ", &runner, &jira)
            .unwrap();

        assert_eq!(roles(&messages), vec![ChatRole::User, ChatRole::Assistant]);
        assert_eq!(messages[0].content, "what is on my plate today?\nthanks");
        assert_eq!(messages[1].content, "Nothing is assigned to you.");
        assert!(jira.calls().is_empty());
        assert_eq!(
            fixture
                .chat
                .get_thread(&fixture.thread_id)
                .unwrap()
                .unwrap()
                .title,
            "what is on my plate today?"
        );
        assert!(runner.prompts()[0].contains("Default project key: AB"));
    }

    #[test]
    fn existing_thread_titles_are_kept() {
        let fixture = fixture();
        fixture
            .chat
            .rename_thread(&fixture.thread_id, "Printer work")
            .unwrap();
        let runner = FakeRunner::new(vec![turn("Ok.", vec![])]);

        fixture
            .send("another question", &runner, &FakeJira::default())
            .unwrap();

        assert_eq!(
            fixture
                .chat
                .get_thread(&fixture.thread_id)
                .unwrap()
                .unwrap()
                .title,
            "Printer work"
        );
    }

    #[test]
    fn lookups_run_immediately_and_feed_the_next_turn() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![
            turn(
                "Checking.",
                vec![raw_call(
                    json!({ "tool": "search_issues", "jql": "project = AB" }),
                )],
            ),
            turn("Two tickets are open: AB-7 and AB-8.", vec![]),
        ]);
        let jira = FakeJira::default();
        let progress = RefCell::new(0);

        let messages = fixture
            .service
            .send_message(
                &fixture.thread_id,
                "what is open?",
                &AgentContext::default(),
                &runner,
                &jira,
                &|_| *progress.borrow_mut() += 1,
            )
            .unwrap();

        assert_eq!(
            roles(&messages),
            vec![
                ChatRole::User,
                ChatRole::Assistant,
                ChatRole::Tool,
                ChatRole::Assistant
            ]
        );
        assert_eq!(messages[2].content, "Search: project = AB");
        assert_eq!(messages[2].status, Some(ChatMessageStatus::Ok));
        assert_eq!(
            jira.calls(),
            vec!["search project = AB limit=50".to_string()]
        );
        let prompts = runner.prompts();
        assert_eq!(prompts.len(), 2);
        assert!(prompts[1].contains("AB-7 | In Progress | Bug | High | Unassigned | Printer jams"));
        assert_eq!(*progress.borrow(), 2);
    }

    #[test]
    fn showing_tickets_updates_the_view_at_once_and_feeds_the_next_turn() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![
            turn(
                "Pulling those up.",
                vec![raw_call(json!({
                    "tool": "show_issues", "title": "Important today", "jql": "priority = High",
                }))],
            ),
            turn("Two tickets need you; start with AB-7.", vec![]),
        ]);
        let jira = FakeJira::default();
        let progress = RefCell::new(Vec::new());

        let messages = fixture
            .service
            .send_message(
                &fixture.thread_id,
                "what is important today?",
                &AgentContext::default(),
                &runner,
                &jira,
                &|event| progress.borrow_mut().push(event),
            )
            .unwrap();

        assert_eq!(
            roles(&messages),
            vec![
                ChatRole::User,
                ChatRole::Assistant,
                ChatRole::Tool,
                ChatRole::Assistant
            ]
        );
        assert_eq!(
            messages[2].content,
            "Show in ticket list: Important today\npriority = High"
        );
        assert_eq!(messages[2].status, Some(ChatMessageStatus::Ok));
        assert_eq!(
            messages[2].result.as_deref(),
            Some(
                "The ticket list now shows 2 tickets as \"Important today\":\n\
                 AB-7 | In Progress | Bug | High | Unassigned | Printer jams\n\
                 AB-8 | In Progress | Bug | No priority | Maya Lopez | Bill acceptor"
            )
        );
        assert_eq!(
            jira.calls(),
            vec!["search priority = High limit=200".to_string()]
        );
        // The view changes before the agent's follow-up turn is stored.
        assert_eq!(
            *progress.borrow(),
            vec![
                AgentProgress::View(ViewAction::ShowIssues {
                    title: "Important today".to_string(),
                    jql: "priority = High".to_string(),
                }),
                AgentProgress::MessagesStored,
                AgentProgress::MessagesStored,
            ]
        );
        assert!(runner.prompts()[1].contains("AB-7 | In Progress | Bug | High"));
    }

    #[test]
    fn a_search_that_jira_rejects_leaves_the_view_alone_and_tells_the_agent() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![
            turn(
                "",
                vec![raw_call(json!({
                    "tool": "show_issues", "title": "Broken", "jql": "priority = = High",
                }))],
            ),
            turn("That search did not work.", vec![]),
        ]);
        let jira = FakeJira::failing(JiraError::InvalidInput("bad JQL".to_string()));
        let progress = RefCell::new(Vec::new());

        let messages = fixture
            .service
            .send_message(
                &fixture.thread_id,
                "show me broken things",
                &AgentContext::default(),
                &runner,
                &jira,
                &|event| progress.borrow_mut().push(event),
            )
            .unwrap();

        assert_eq!(messages[1].role, ChatRole::Tool);
        assert_eq!(messages[1].status, Some(ChatMessageStatus::Error));
        assert!(messages[1].result.as_deref().unwrap().contains("bad JQL"));
        assert_eq!(
            *progress.borrow(),
            vec![AgentProgress::MessagesStored, AgentProgress::MessagesStored]
        );
        assert_eq!(runner.prompts().len(), 2);
    }

    #[test]
    fn an_empty_search_still_updates_the_view_and_says_so() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![
            turn(
                "",
                vec![raw_call(json!({
                    "tool": "show_issues", "title": "Quiet", "jql": "text ~ nothing",
                }))],
            ),
            turn("Nothing matched.", vec![]),
        ]);
        let progress = RefCell::new(Vec::new());

        let messages = fixture
            .service
            .send_message(
                &fixture.thread_id,
                "anything quiet?",
                &AgentContext::default(),
                &runner,
                &FakeJira::default(),
                &|event| progress.borrow_mut().push(event),
            )
            .unwrap();

        assert_eq!(
            messages[1].result.as_deref(),
            Some("No tickets matched. The ticket list is empty.")
        );
        assert!(matches!(
            progress.borrow()[0],
            AgentProgress::View(ViewAction::ShowIssues { .. })
        ));
    }

    #[test]
    fn opening_a_ticket_updates_the_view_without_another_agent_turn() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![turn(
            "Here it is.",
            vec![raw_call(
                json!({ "tool": "open_issue", "issueKey": "AB-7" }),
            )],
        )]);
        let jira = FakeJira::default();
        let progress = RefCell::new(Vec::new());

        let messages = fixture
            .service
            .send_message(
                &fixture.thread_id,
                "open AB-7",
                &AgentContext::default(),
                &runner,
                &jira,
                &|event| progress.borrow_mut().push(event),
            )
            .unwrap();

        assert_eq!(
            roles(&messages),
            vec![ChatRole::User, ChatRole::Assistant, ChatRole::Tool]
        );
        assert_eq!(messages[2].content, "Open AB-7");
        assert_eq!(messages[2].status, Some(ChatMessageStatus::Ok));
        assert_eq!(
            messages[2].result.as_deref(),
            Some("AB-7 is open in the ticket pane.")
        );
        assert!(jira.calls().is_empty());
        assert_eq!(
            *progress.borrow(),
            vec![
                AgentProgress::View(ViewAction::OpenIssue {
                    issue_key: key("AB-7")
                }),
                AgentProgress::MessagesStored,
            ]
        );
        assert_eq!(runner.prompts().len(), 1);
    }

    #[test]
    fn requested_changes_wait_as_proposals_without_touching_jira() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![turn(
            "I can move AB-7 and leave a note.",
            vec![
                raw_call(
                    json!({ "tool": "transition_issue", "issueKey": "AB-7", "status": "Dev Complete" }),
                ),
                raw_call(
                    json!({ "tool": "add_comment", "issueKey": "AB-7", "comment": "Fixed in build 512." }),
                ),
            ],
        )]);
        let jira = FakeJira::default();

        let messages = fixture.send("close out AB-7", &runner, &jira).unwrap();

        assert_eq!(
            roles(&messages),
            vec![
                ChatRole::User,
                ChatRole::Assistant,
                ChatRole::Proposal,
                ChatRole::Proposal
            ]
        );
        assert_eq!(messages[2].content, "Move AB-7 to \"Dev Complete\"");
        assert_eq!(messages[2].status, Some(ChatMessageStatus::Pending));
        assert_eq!(messages[3].content, "Comment on AB-7:\nFixed in build 512.");
        assert!(jira.calls().is_empty());
        assert_eq!(runner.prompts().len(), 1);
    }

    #[test]
    fn failed_lookups_and_unusable_calls_are_reported_back_to_the_agent() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![
            turn(
                "",
                vec![
                    raw_call(json!({ "tool": "get_issue", "issueKey": "AB-404" })),
                    raw_call(json!({ "tool": "search_issues" })),
                ],
            ),
            turn("I could not find AB-404.", vec![]),
        ]);
        let jira = FakeJira::failing(JiraError::NotFound {
            path: "issue/AB-404".to_string(),
        });

        let messages = fixture.send("show AB-404", &runner, &jira).unwrap();

        assert_eq!(
            roles(&messages),
            vec![
                ChatRole::User,
                ChatRole::Tool,
                ChatRole::Tool,
                ChatRole::Assistant
            ]
        );
        assert_eq!(messages[1].status, Some(ChatMessageStatus::Error));
        assert!(messages[1].result.as_deref().unwrap().contains("404"));
        assert_eq!(messages[2].status, Some(ChatMessageStatus::Error));
        assert_eq!(messages[2].content, "Unusable request from the agent");
        assert_eq!(
            messages[2].result.as_deref(),
            Some("search_issues requires jql")
        );
        assert!(runner.prompts()[1].contains("[lookup error]"));
    }

    #[test]
    fn agent_is_stopped_after_the_turn_limit() {
        let fixture = fixture();
        let looping = || {
            turn(
                "",
                vec![raw_call(json!({ "tool": "get_issue", "issueKey": "AB-7" }))],
            )
        };
        let runner = FakeRunner::new((0..MAX_AGENT_TURNS).map(|_| looping()).collect());
        let jira = FakeJira::default();

        let messages = fixture.send("loop", &runner, &jira).unwrap();

        assert_eq!(runner.prompts().len(), MAX_AGENT_TURNS);
        assert_eq!(jira.calls().len(), MAX_AGENT_TURNS);
        let last = messages.last().unwrap();
        assert_eq!(last.role, ChatRole::Assistant);
        assert!(last.content.contains("stopped"), "{}", last.content);
    }

    #[test]
    fn empty_turn_leaves_a_visible_reply() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![turn("  ", vec![])]);

        let messages = fixture
            .send("hello", &runner, &FakeJira::default())
            .unwrap();

        assert_eq!(roles(&messages), vec![ChatRole::User, ChatRole::Assistant]);
        assert_eq!(messages[1].content, "(The agent returned no reply.)");
    }

    #[test]
    fn agent_failures_keep_the_user_message() {
        let fixture = fixture();
        let missing = FakeRunner::new(vec![Err(
            "codex command was not found: codex.cmd".to_string()
        )]);
        let garbled = FakeRunner::new(vec![Ok("not json".to_string())]);
        let jira = FakeJira::default();

        let error = fixture.send("first", &missing, &jira).unwrap_err();
        assert!(
            matches!(&error, ServiceError::Agent(message) if message.contains("codex command was not found")),
            "{error}"
        );
        let error = fixture.send("second", &garbled, &jira).unwrap_err();
        assert!(matches!(error, ServiceError::Agent(_)));

        let stored = fixture.chat.list_messages(&fixture.thread_id).unwrap();
        assert_eq!(roles(&stored), vec![ChatRole::User, ChatRole::User]);
    }

    #[test]
    fn blank_oversized_and_orphan_messages_are_rejected_before_the_agent_runs() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![]);
        let jira = FakeJira::default();

        assert!(matches!(
            fixture.send("   ", &runner, &jira),
            Err(ServiceError::InvalidInput("message text is required"))
        ));
        assert!(matches!(
            fixture.send(&"x".repeat(8_001), &runner, &jira),
            Err(ServiceError::InvalidInput("message text is too long"))
        ));
        assert!(matches!(
            fixture.service.send_message(
                "missing-thread",
                "hi",
                &AgentContext::default(),
                &runner,
                &jira,
                &|_| {},
            ),
            Err(ServiceError::NotFound {
                entity: "chat thread",
                ..
            })
        ));
        assert!(runner.prompts().is_empty());
        assert!(fixture
            .chat
            .list_messages(&fixture.thread_id)
            .unwrap()
            .is_empty());
    }

    fn pending_proposal(fixture: &Fixture, call: serde_json::Value) -> String {
        let runner = FakeRunner::new(vec![turn("Proposed.", vec![raw_call(call)])]);
        let messages = fixture
            .send("do it", &runner, &FakeJira::default())
            .unwrap();
        messages
            .iter()
            .rev()
            .find(|message| message.role == ChatRole::Proposal)
            .expect("proposal stored")
            .id
            .clone()
    }

    #[test]
    fn approving_a_proposal_runs_it_once() {
        let fixture = fixture();
        let proposal_id = pending_proposal(
            &fixture,
            json!({ "tool": "add_comment", "issueKey": "AB-7", "comment": "Fixed in build 512." }),
        );
        let jira = FakeJira::default();

        let messages = fixture
            .service
            .approve_proposal(&proposal_id, "AB", &jira)
            .unwrap();

        let proposal = messages
            .iter()
            .find(|message| message.id == proposal_id)
            .unwrap();
        assert_eq!(proposal.status, Some(ChatMessageStatus::Approved));
        assert_eq!(proposal.result.as_deref(), Some("Comment added to AB-7."));
        assert_eq!(
            jira.calls(),
            vec!["comment AB-7 Fixed in build 512.".to_string()]
        );

        assert!(matches!(
            fixture.service.approve_proposal(&proposal_id, "AB", &jira),
            Err(ServiceError::InvalidInput(
                "this change was already approved or rejected"
            ))
        ));
        assert_eq!(jira.calls().len(), 1);
    }

    #[test]
    fn a_jira_failure_is_recorded_on_the_approved_proposal() {
        let fixture = fixture();
        let proposal_id = pending_proposal(
            &fixture,
            json!({ "tool": "transition_issue", "issueKey": "AB-7", "status": "Done" }),
        );
        let jira = FakeJira::failing(JiraError::Unauthorized);

        let messages = fixture
            .service
            .approve_proposal(&proposal_id, "AB", &jira)
            .unwrap();

        let proposal = messages
            .iter()
            .find(|message| message.id == proposal_id)
            .unwrap();
        assert_eq!(proposal.status, Some(ChatMessageStatus::Failed));
        assert!(proposal
            .result
            .as_deref()
            .unwrap()
            .contains("rejected the credentials"));
    }

    #[test]
    fn rejecting_a_proposal_never_reaches_jira() {
        let fixture = fixture();
        let proposal_id = pending_proposal(
            &fixture,
            json!({ "tool": "assign_issue", "issueKey": "AB-7", "assignee": "me" }),
        );
        let jira = FakeJira::default();

        let messages = fixture.service.reject_proposal(&proposal_id).unwrap();

        let proposal = messages
            .iter()
            .find(|message| message.id == proposal_id)
            .unwrap();
        assert_eq!(proposal.status, Some(ChatMessageStatus::Rejected));
        assert!(matches!(
            fixture.service.approve_proposal(&proposal_id, "AB", &jira),
            Err(ServiceError::InvalidInput(_))
        ));
        assert!(matches!(
            fixture.service.reject_proposal(&proposal_id),
            Err(ServiceError::InvalidInput(_))
        ));
        assert!(jira.calls().is_empty());
    }

    #[test]
    fn only_proposals_can_be_approved() {
        let fixture = fixture();
        let runner = FakeRunner::new(vec![turn("Hello.", vec![])]);
        let messages = fixture.send("hi", &runner, &FakeJira::default()).unwrap();
        let jira = FakeJira::default();

        assert!(matches!(
            fixture
                .service
                .approve_proposal(&messages[0].id, "AB", &jira),
            Err(ServiceError::InvalidInput(_))
        ));
        assert!(matches!(
            fixture.service.approve_proposal("missing", "AB", &jira),
            Err(ServiceError::NotFound {
                entity: "chat message",
                ..
            })
        ));
        assert!(jira.calls().is_empty());
    }

    #[test]
    fn changes_describe_their_outcome() {
        let jira = FakeJira::default();
        let apply = |call: JiraToolCall| apply_change(&call, "AB", &jira).unwrap();

        assert_eq!(
            apply(JiraToolCall::CreateIssue {
                project_key: None,
                issue_type: "Bug".to_string(),
                summary: "Printer jams".to_string(),
                description: "Observed".to_string(),
            }),
            "Created AB-12: https://example.atlassian.net/browse/AB-12"
        );
        assert_eq!(
            apply(JiraToolCall::CreateIssue {
                project_key: Some("CD".to_string()),
                issue_type: "Task".to_string(),
                summary: "Other project".to_string(),
                description: String::new(),
            }),
            "Created AB-12: https://example.atlassian.net/browse/AB-12"
        );
        assert_eq!(
            apply(JiraToolCall::UpdateIssue {
                issue_key: key("AB-7"),
                summary: Some("New title".to_string()),
                description: None,
            }),
            "Updated AB-7."
        );
        assert_eq!(
            apply(JiraToolCall::TransitionIssue {
                issue_key: key("AB-7"),
                status: "Dev Complete".to_string(),
            }),
            "AB-7 moved from In Progress to Dev Complete."
        );
        assert_eq!(
            apply(JiraToolCall::TransitionIssue {
                issue_key: key("AB-7"),
                status: "In Progress".to_string(),
            }),
            "AB-7 is already In Progress."
        );
        assert_eq!(
            apply(JiraToolCall::AssignIssue {
                issue_key: key("AB-7"),
                assignee: Assignee::Me,
            }),
            "AB-7 assigned to you."
        );
        assert_eq!(
            apply(JiraToolCall::AssignIssue {
                issue_key: key("AB-7"),
                assignee: Assignee::Unassigned,
            }),
            "AB-7 unassigned."
        );
        assert_eq!(
            jira.calls(),
            vec![
                "create AB Bug Printer jams | Observed".to_string(),
                "create CD Task Other project | ".to_string(),
                "update AB-7 summary=Some(\"New title\") description=None".to_string(),
                "transition AB-7 Dev Complete".to_string(),
                "transition AB-7 In Progress".to_string(),
                "assign AB-7 Me".to_string(),
                "assign AB-7 Unassigned".to_string(),
            ]
        );
    }

    #[test]
    fn creating_without_any_project_key_is_refused() {
        let jira = FakeJira::default();

        let error = apply_change(
            &JiraToolCall::CreateIssue {
                project_key: None,
                issue_type: "Bug".to_string(),
                summary: "Printer jams".to_string(),
                description: String::new(),
            },
            "  ",
            &jira,
        )
        .unwrap_err();

        assert!(matches!(error, JiraError::NotConfigured(_)));
        assert!(jira.calls().is_empty());
    }

    #[test]
    fn lookups_are_not_changes_and_changes_are_not_lookups() {
        let jira = FakeJira::default();

        assert!(matches!(
            apply_change(
                &JiraToolCall::GetIssue {
                    issue_key: key("AB-7")
                },
                "AB",
                &jira
            ),
            Err(JiraError::InvalidInput(_))
        ));
        assert!(matches!(
            run_lookup(
                &JiraToolCall::AddComment {
                    issue_key: key("AB-7"),
                    comment: "x".to_string()
                },
                &jira
            ),
            Err(JiraError::InvalidInput(_))
        ));
        assert!(jira.calls().is_empty());
    }

    #[test]
    fn search_lookup_lists_one_ticket_per_line() {
        let jira = FakeJira::default();

        let result = run_lookup(
            &JiraToolCall::SearchIssues {
                jql: "project = AB".to_string(),
            },
            &jira,
        )
        .unwrap();

        assert_eq!(
            result,
            "2 tickets:\n\
             AB-7 | In Progress | Bug | High | Unassigned | Printer jams\n\
             AB-8 | In Progress | Bug | No priority | Maya Lopez | Bill acceptor"
        );
        assert_eq!(
            run_lookup(
                &JiraToolCall::SearchIssues {
                    jql: "nothing".to_string()
                },
                &jira
            )
            .unwrap(),
            "No tickets matched."
        );
    }

    #[test]
    fn issue_lookup_includes_description_recent_comments_and_transitions() {
        let jira = FakeJira::default();

        let result = run_lookup(
            &JiraToolCall::GetIssue {
                issue_key: key("AB-7"),
            },
            &jira,
        )
        .unwrap();

        assert!(
            result.starts_with(
                "AB-7 | In Progress | Bug | High | Unassigned | Printer jams\n\
             URL: https://example.atlassian.net/browse/AB-7\n\
             Reporter: Rina Q\n\
             Parent: AB-1\n\
             Available transitions: Dev Complete\n\
             \n\
             Description:\n\
             Jams after reboot.\n\
             \n\
             Comments (showing last 10 of 12):\n\
             --- Maya Lopez, 2026-11-03 ---\n\
             comment 3\n"
            ),
            "{result}"
        );
        assert!(!result.contains("comment 2\n"));
        assert!(result.ends_with("--- Maya Lopez, 2026-11-12 ---\ncomment 12"));
    }
}
