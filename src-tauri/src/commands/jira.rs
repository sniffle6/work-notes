//! Tauri commands for Jira tickets and the Jira chat agent.
//!
//! Jira and Codex calls block, so they run on the blocking thread pool.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::{parse_note_id, CommandError};
use crate::app_state::AppState;
use crate::jira::agent::{AgentContext, CodexAgentRunner, ViewAction};
use crate::jira::chat::{ChatMessage, ChatThread};
use crate::jira::client::DEFAULT_SEARCH_LIMIT;
use crate::jira::{
    Assignee, IssueKey, IssueUpdate, JiraError, JiraIssueDetail, JiraIssueSummary, JiraOps,
    JiraUser, NewIssue,
};
use crate::services::jira::{connect, jira_status, JiraEnvironment, JiraStatus};
use crate::services::jira_agent::{AgentProgress, JiraAgentService, NEW_THREAD_TITLE};
use crate::services::jira_attachments::{
    download_attachments, load_thumbnail, read_attachment_image, user_downloads_dir,
    AttachmentDownload, AttachmentThumbnail,
};
use crate::services::search::SearchService;

/// Emitted with [`JiraChatUpdatedPayload`] each time an agent run stores
/// new messages, so the conversation can refresh while the agent works.
pub const JIRA_CHAT_UPDATED_EVENT: &str = "jira-chat:updated";
/// Emitted with [`JiraViewActionPayload`] when the agent changes what the
/// Jira view shows: the ticket list or the open ticket.
pub const JIRA_VIEW_ACTION_EVENT: &str = "jira-view:action";
/// Agent turns reason over ticket contents, so they get more time than a
/// note parse.
const AGENT_TURN_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraChatUpdatedPayload {
    pub thread_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraViewActionPayload {
    pub thread_id: String,
    pub action: ViewAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJiraIssueDto {
    pub issue_type: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    /// Note the ticket is created from; the new key is linked to it.
    #[serde(default)]
    pub note_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJiraIssueDto {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

async fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| CommandError::new("task_error", "background task failed"))?
}

#[tauri::command]
pub async fn get_jira_status(
    state: tauri::State<'_, AppState>,
) -> Result<JiraStatus, CommandError> {
    let settings = state.settings.get()?;
    Ok(jira_status(&settings, &JiraEnvironment::from_process()))
}

#[tauri::command]
pub async fn test_jira_connection(
    state: tauri::State<'_, AppState>,
) -> Result<JiraUser, CommandError> {
    let settings = state.settings.clone();
    run_blocking(move || Ok(connect(&settings)?.myself()?)).await
}

#[tauri::command]
pub async fn list_jira_issues(
    state: tauri::State<'_, AppState>,
    jql: String,
) -> Result<Vec<JiraIssueSummary>, CommandError> {
    let settings = state.settings.clone();
    run_blocking(move || Ok(connect(&settings)?.search(&jql, DEFAULT_SEARCH_LIMIT)?)).await
}

#[tauri::command]
pub async fn get_jira_issue(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<JiraIssueDetail, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || Ok(connect(&settings)?.get_issue(&key)?)).await
}

/// Lists issue types for the configured project. Returns an empty list when
/// no project key is set.
#[tauri::command]
pub async fn list_jira_issue_types(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, CommandError> {
    let settings = state.settings.clone();
    run_blocking(move || {
        let project_key = settings.get()?.jira_project_key;
        if project_key.is_empty() {
            return Ok(Vec::new());
        }
        Ok(connect(&settings)?.issue_types(&project_key)?)
    })
    .await
}

#[tauri::command]
pub async fn create_jira_issue(
    state: tauri::State<'_, AppState>,
    input: CreateJiraIssueDto,
) -> Result<JiraIssueDetail, CommandError> {
    let note_id = input.note_id.as_deref().map(parse_note_id).transpose()?;
    let state = state.inner().clone();
    run_blocking(move || {
        // Fail before creating anything when the source note is gone.
        if let Some(note_id) = note_id {
            SearchService::new(state.repositories.clone()).get_note(note_id)?;
        }
        let project_key = state.settings.get()?.jira_project_key;
        if project_key.is_empty() {
            return Err(JiraError::NotConfigured(
                "set the Jira project key in Settings > Jira before creating tickets".to_string(),
            )
            .into());
        }

        let client = connect(&state.settings)?;
        let key = client.create_issue(&NewIssue {
            project_key,
            issue_type: input.issue_type,
            summary: input.summary,
            description: input.description,
        })?;
        if let Some(note_id) = note_id {
            // The ticket exists now; report a link failure without hiding that.
            state
                .repositories
                .jira_links
                .link(note_id, &key)
                .map_err(|_| {
                    CommandError::new(
                        "storage_error",
                        format!("{key} was created in Jira, but linking it to the note failed"),
                    )
                })?;
        }
        Ok(client.get_issue(&key)?)
    })
    .await
}

#[tauri::command]
pub async fn update_jira_issue(
    state: tauri::State<'_, AppState>,
    key: String,
    input: UpdateJiraIssueDto,
) -> Result<JiraIssueDetail, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || {
        let client = connect(&settings)?;
        client.update_issue(
            &key,
            &IssueUpdate {
                summary: input.summary,
                description: input.description,
            },
        )?;
        Ok(client.get_issue(&key)?)
    })
    .await
}

#[tauri::command]
pub async fn transition_jira_issue(
    state: tauri::State<'_, AppState>,
    key: String,
    transition_id: String,
) -> Result<JiraIssueDetail, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || {
        let client = connect(&settings)?;
        client.apply_transition(&key, &transition_id)?;
        Ok(client.get_issue(&key)?)
    })
    .await
}

#[tauri::command]
pub async fn add_jira_comment(
    state: tauri::State<'_, AppState>,
    key: String,
    text: String,
) -> Result<JiraIssueDetail, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || {
        let client = connect(&settings)?;
        client.add_comment(&key, &text)?;
        Ok(client.get_issue(&key)?)
    })
    .await
}

#[tauri::command]
pub async fn assign_jira_issue_to_me(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<JiraIssueDetail, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || {
        let client = connect(&settings)?;
        client.assign(&key, Assignee::Me)?;
        Ok(client.get_issue(&key)?)
    })
    .await
}

/// Saves a ticket's attachments to `Downloads\Work Notes\<KEY>` and returns
/// where each one is. `attachment_ids` limits the download; `None` saves all.
#[tauri::command]
pub async fn download_jira_attachments(
    state: tauri::State<'_, AppState>,
    key: String,
    attachment_ids: Option<Vec<String>>,
) -> Result<AttachmentDownload, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || {
        let downloads_dir = user_downloads_dir()?;
        let client = connect(&settings)?;
        Ok(download_attachments(
            &client,
            &key,
            attachment_ids.as_deref(),
            &downloads_dir,
        )?)
    })
    .await
}

/// Returns Jira's small preview image of an attachment. Fails when Jira has
/// no preview for it.
#[tauri::command]
pub async fn get_jira_attachment_thumbnail(
    state: tauri::State<'_, AppState>,
    attachment_id: String,
) -> Result<AttachmentThumbnail, CommandError> {
    let settings = state.settings.clone();
    run_blocking(move || Ok(load_thumbnail(&connect(&settings)?, &attachment_id)?)).await
}

/// Returns the raw bytes of an image attachment for the in-app viewer,
/// downloading it to the ticket folder first when needed. Fails when the
/// file is not an image the app can show.
#[tauri::command]
pub async fn read_jira_attachment_image(
    state: tauri::State<'_, AppState>,
    key: String,
    attachment_id: String,
) -> Result<tauri::ipc::Response, CommandError> {
    let key = IssueKey::parse(&key)?;
    let settings = state.settings.clone();
    run_blocking(move || {
        let downloads_dir = user_downloads_dir()?;
        let client = connect(&settings)?;
        let bytes = read_attachment_image(&client, &key, &attachment_id, &downloads_dir)?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
}

#[tauri::command]
pub async fn list_jira_chat_threads(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ChatThread>, CommandError> {
    Ok(state.repositories.jira_chat.list_threads()?)
}

#[tauri::command]
pub async fn create_jira_chat_thread(
    state: tauri::State<'_, AppState>,
) -> Result<ChatThread, CommandError> {
    Ok(state
        .repositories
        .jira_chat
        .create_thread(NEW_THREAD_TITLE)?)
}

#[tauri::command]
pub async fn delete_jira_chat_thread(
    state: tauri::State<'_, AppState>,
    thread_id: String,
) -> Result<(), CommandError> {
    Ok(state.repositories.jira_chat.delete_thread(&thread_id)?)
}

#[tauri::command]
pub async fn list_jira_chat_messages(
    state: tauri::State<'_, AppState>,
    thread_id: String,
) -> Result<Vec<ChatMessage>, CommandError> {
    Ok(state.repositories.jira_chat.list_messages(&thread_id)?)
}

/// Runs the agent on a new user message and returns the whole thread.
///
/// Lookups and view changes run immediately; view changes are emitted as
/// [`JIRA_VIEW_ACTION_EVENT`]. Changes to Jira are stored as proposals for
/// [`resolve_jira_chat_proposal`]. Fails with `agent_busy` when the thread
/// already has a run in progress.
#[tauri::command]
pub async fn send_jira_chat_message(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    thread_id: String,
    text: String,
    focus_issue_key: Option<String>,
) -> Result<Vec<ChatMessage>, CommandError> {
    let state = state.inner().clone();
    run_blocking(move || {
        let _busy = state.jira_agent_busy.try_claim(&thread_id).ok_or_else(|| {
            CommandError::new(
                "agent_busy",
                "The agent is still working on this conversation.",
            )
        })?;
        let settings = state.settings.get()?;
        let client = connect(&state.settings)?;
        let context = AgentContext {
            site_url: settings.jira_site_url,
            project_key: settings.jira_project_key,
            today: chrono::Local::now().format("%Y-%m-%d").to_string(),
            focus_issue_key: focus_issue_key
                .and_then(|key| IssueKey::parse(&key).ok())
                .map(String::from),
        };
        let runner = CodexAgentRunner {
            program: settings.codex_command_path,
            timeout: AGENT_TURN_TIMEOUT,
        };
        let notify = |progress: AgentProgress| {
            let _ = match progress {
                AgentProgress::MessagesStored => app.emit(
                    JIRA_CHAT_UPDATED_EVENT,
                    JiraChatUpdatedPayload {
                        thread_id: thread_id.clone(),
                    },
                ),
                AgentProgress::View(action) => app.emit(
                    JIRA_VIEW_ACTION_EVENT,
                    JiraViewActionPayload {
                        thread_id: thread_id.clone(),
                        action,
                    },
                ),
            };
        };

        Ok(JiraAgentService::new(state.repositories.jira_chat.clone())
            .send_message(&thread_id, &text, &context, &runner, &client, &notify)?)
    })
    .await
}

/// Approves (and runs) or rejects a change the agent proposed. Returns the
/// whole thread; a Jira failure is recorded on the proposal as `failed`.
#[tauri::command]
pub async fn resolve_jira_chat_proposal(
    state: tauri::State<'_, AppState>,
    message_id: String,
    approve: bool,
) -> Result<Vec<ChatMessage>, CommandError> {
    let state = state.inner().clone();
    run_blocking(move || {
        let service = JiraAgentService::new(state.repositories.jira_chat.clone());
        if !approve {
            return Ok(service.reject_proposal(&message_id)?);
        }
        let project_key = state.settings.get()?.jira_project_key;
        let client = connect(&state.settings)?;
        Ok(service.approve_proposal(&message_id, &project_key, &client)?)
    })
    .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        CreateJiraIssueDto, JiraChatUpdatedPayload, JiraViewActionPayload, UpdateJiraIssueDto,
    };
    use crate::jira::agent::ViewAction;

    #[test]
    fn jira_input_dtos_accept_camel_case_and_optional_fields() {
        let create: CreateJiraIssueDto = serde_json::from_value(json!({
            "issueType": "Bug",
            "summary": "Printer jams",
            "description": "Observed",
            "noteId": "note-1",
        }))
        .unwrap();
        assert_eq!(create.issue_type, "Bug");
        assert_eq!(create.note_id.as_deref(), Some("note-1"));

        let minimal: CreateJiraIssueDto =
            serde_json::from_value(json!({ "issueType": "Task", "summary": "Tidy" })).unwrap();
        assert_eq!(minimal.description, "");
        assert_eq!(minimal.note_id, None);

        let update: UpdateJiraIssueDto =
            serde_json::from_value(json!({ "summary": null, "description": "Body" })).unwrap();
        assert_eq!(update.summary, None);
        assert_eq!(update.description.as_deref(), Some("Body"));
    }

    #[test]
    fn chat_updated_payload_serializes_camel_case() {
        let payload = serde_json::to_value(JiraChatUpdatedPayload {
            thread_id: "thread-1".to_string(),
        })
        .unwrap();

        assert_eq!(payload, json!({ "threadId": "thread-1" }));
    }

    #[test]
    fn view_action_payload_serializes_camel_case() {
        let payload = serde_json::to_value(JiraViewActionPayload {
            thread_id: "thread-1".to_string(),
            action: ViewAction::ShowIssues {
                title: "Important today".to_string(),
                jql: "priority = High".to_string(),
            },
        })
        .unwrap();

        assert_eq!(
            payload,
            json!({
                "threadId": "thread-1",
                "action": { "kind": "showIssues", "title": "Important today", "jql": "priority = High" },
            })
        );
    }
}
