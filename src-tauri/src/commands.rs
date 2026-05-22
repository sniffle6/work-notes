use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::domain::{
    ActionItem, ActionItemId, ActionStatus, InboxFilters, Note, NoteId, NoteListItem, ParseStatus,
    ReviewStatus, Tag, TagAssignment, TagId, TagKind,
};
use crate::repositories::RepositoryError;
use crate::services::capture::CaptureService;
use crate::services::parse_queue::ParseQueue;
use crate::services::search::{NoteDetail, SearchService};
use crate::services::settings::AppSettings;
use crate::services::ServiceError;
use crate::windowing;

pub type AppSettingsDto = AppSettings;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    fn invalid_input(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }

    fn not_found(entity: &str, id: &str) -> Self {
        Self::new("not_found", format!("{entity} not found: {id}"))
    }

    fn storage_error() -> Self {
        Self::new("storage_error", "storage error")
    }
}

impl From<ServiceError> for CommandError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::InvalidInput(message) => Self::invalid_input(message),
            ServiceError::NotFound { entity, id } => Self::not_found(entity, &id),
            ServiceError::Repository(error) => Self::from(error),
            ServiceError::Database(_) | ServiceError::Sqlite(_) => Self::storage_error(),
            ServiceError::Json(_) => Self::new("settings_error", "settings data is invalid"),
            ServiceError::Parser(_) => Self::new("parser_error", "parser failed"),
            ServiceError::StatePoisoned { name } => {
                Self::new("state_error", format!("{name} state unavailable"))
            }
        }
    }
}

impl From<RepositoryError> for CommandError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound { entity, id } => Self::not_found(entity, &id),
            RepositoryError::Domain(_) => Self::invalid_input("invalid id"),
            RepositoryError::Database(_) | RepositoryError::Sqlite(_) => Self::storage_error(),
            RepositoryError::InvalidDatabaseValue { .. } => {
                Self::new("storage_error", "stored data is invalid")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteListItemDto {
    pub id: String,
    pub title: String,
    pub raw_text: String,
    pub cleaned_text: Option<String>,
    pub summary: Option<String>,
    pub capture_source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parse_status: ParseStatus,
    pub review_status: ReviewStatus,
    pub is_archived: bool,
    pub tags: Vec<TagAssignmentDto>,
    pub tag_count: u32,
    pub action_item_count: u32,
    pub suggested_action_item_count: u32,
}

impl NoteListItemDto {
    fn from_item(item: NoteListItem, tags: Vec<TagAssignment>) -> Self {
        Self {
            id: item.id.to_string(),
            title: item.title,
            raw_text: item.raw_text,
            cleaned_text: item.cleaned_text,
            summary: item.summary,
            capture_source: "quick_capture".to_string(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            parse_status: item.parse_status,
            review_status: item.review_status,
            is_archived: item.is_archived,
            tags: tags.into_iter().map(Into::into).collect(),
            tag_count: item.tag_count,
            action_item_count: item.action_item_count,
            suggested_action_item_count: item.suggested_action_item_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDetailDto {
    pub id: String,
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
    pub tags: Vec<TagAssignmentDto>,
    pub action_items: Vec<ActionItemDto>,
    pub parse_error: Option<String>,
}

impl From<NoteDetail> for NoteDetailDto {
    fn from(detail: NoteDetail) -> Self {
        let note = detail.note;
        Self {
            id: note.id.to_string(),
            title: note.title,
            raw_text: note.raw_text,
            cleaned_text: note.cleaned_text,
            summary: note.summary,
            created_at: note.created_at,
            updated_at: note.updated_at,
            capture_source: note.capture_source,
            parse_status: note.parse_status,
            review_status: note.review_status,
            is_archived: note.is_archived,
            tags: detail.tags.into_iter().map(Into::into).collect(),
            action_items: detail.action_items.into_iter().map(Into::into).collect(),
            parse_error: detail.parse_error,
        }
    }
}

impl From<Note> for NoteDetailDto {
    fn from(note: Note) -> Self {
        Self {
            id: note.id.to_string(),
            title: note.title,
            raw_text: note.raw_text,
            cleaned_text: note.cleaned_text,
            summary: note.summary,
            created_at: note.created_at,
            updated_at: note.updated_at,
            capture_source: note.capture_source,
            parse_status: note.parse_status,
            review_status: note.review_status,
            is_archived: note.is_archived,
            tags: Vec::new(),
            action_items: Vec::new(),
            parse_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagAssignmentDto {
    pub tag: TagDto,
    pub source: String,
    pub confidence: Option<f64>,
}

impl From<TagAssignment> for TagAssignmentDto {
    fn from(assignment: TagAssignment) -> Self {
        Self {
            tag: assignment.tag.into(),
            source: assignment.source,
            confidence: assignment.confidence,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub name: String,
    pub kind: TagKind,
    pub created_at: DateTime<Utc>,
}

impl From<Tag> for TagDto {
    fn from(tag: Tag) -> Self {
        Self {
            id: tag.id.to_string(),
            name: tag.name,
            kind: tag.kind,
            created_at: tag.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemDto {
    pub id: String,
    pub note_id: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
}

impl From<ActionItem> for ActionItemDto {
    fn from(action_item: ActionItem) -> Self {
        Self {
            id: action_item.id.to_string(),
            note_id: action_item.note_id.to_string(),
            text: action_item.text,
            owner: action_item.owner,
            due_date: action_item.due_date,
            status: action_item.status,
            source: action_item.source,
            confidence: action_item.confidence,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxFiltersDto {
    pub parse_status: Option<ParseStatus>,
    pub review_status: Option<ReviewStatus>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    pub query: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
    pub limit: Option<u32>,
}

impl TryFrom<InboxFiltersDto> for InboxFilters {
    type Error = CommandError;

    fn try_from(filters: InboxFiltersDto) -> Result<Self, Self::Error> {
        let tag_ids = filters
            .tag_ids
            .iter()
            .map(|id| TagId::parse(id).map_err(|_| CommandError::invalid_input("invalid tag id")))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            parse_status: filters.parse_status,
            review_status: filters.review_status,
            tag_ids,
            query: filters.query,
            include_archived: filters.include_archived,
            limit: filters.limit,
        })
    }
}

#[tauri::command]
pub async fn save_capture_note(
    state: tauri::State<'_, AppState>,
    raw_text: String,
) -> Result<NoteDetailDto, CommandError> {
    let capture = CaptureService::new(state.repositories.clone());
    let note = capture.capture(&raw_text)?;
    let detail = SearchService::new(state.repositories.clone()).get_note(note.id)?;
    Ok(detail.into())
}

#[tauri::command]
pub async fn list_inbox(
    state: tauri::State<'_, AppState>,
    filters: InboxFiltersDto,
) -> Result<Vec<NoteListItemDto>, CommandError> {
    let filters = InboxFilters::try_from(filters)?;
    let items = SearchService::new(state.repositories.clone()).list_inbox(filters)?;
    items
        .into_iter()
        .map(|item| {
            let tags = state.repositories.tags.list_for_note(item.id)?;
            Ok(NoteListItemDto::from_item(item, tags))
        })
        .collect()
}

#[tauri::command]
pub async fn get_note(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<NoteDetailDto, CommandError> {
    let id = parse_note_id(&id)?;
    let detail = SearchService::new(state.repositories.clone()).get_note(id)?;
    Ok(detail.into())
}

#[tauri::command]
pub async fn retry_parse(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    ParseQueue::with_config(
        state.repositories.clone(),
        state.parse_queue_config.clone(),
        state.parser_provider_config.clone(),
    )
    .retry_note(note_id)?;
    Ok(())
}

#[tauri::command]
pub async fn retry_parse_with_feedback(
    state: tauri::State<'_, AppState>,
    note_id: String,
    feedback: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    ParseQueue::with_config(
        state.repositories.clone(),
        state.parse_queue_config.clone(),
        state.parser_provider_config.clone(),
    )
    .retry_note_with_feedback(note_id, &feedback)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    state.repositories.notes.archive(note_id)?;
    Ok(())
}

#[tauri::command]
pub async fn accept_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    state
        .repositories
        .action_items
        .set_status(action_id, ActionStatus::Accepted)?;
    Ok(())
}

#[tauri::command]
pub async fn dismiss_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    state
        .repositories
        .action_items
        .set_status(action_id, ActionStatus::Dismissed)?;
    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
) -> Result<AppSettingsDto, CommandError> {
    state.settings.get().map_err(Into::into)
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: AppSettingsDto,
) -> Result<AppSettingsDto, CommandError> {
    windowing::hotkey::validate_shortcut(&settings.global_hotkey)
        .map_err(|error| CommandError::new("hotkey_error", error.to_string()))?;

    let previous_hotkey = state
        .settings
        .get()
        .map(|settings| settings.global_hotkey)
        .unwrap_or_else(|_| windowing::hotkey::DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string());

    if let Err(error) = windowing::hotkey::register_global_shortcut(&app, &settings.global_hotkey) {
        let _ = windowing::hotkey::register_global_shortcut(&app, &previous_hotkey);
        return Err(CommandError::new("hotkey_error", error.to_string()));
    }

    let saved = state.settings.save(settings).map_err(CommandError::from)?;
    Ok(saved)
}

#[tauri::command]
pub async fn hide_quick_capture(app: tauri::AppHandle) -> Result<(), CommandError> {
    windowing::quick_capture::hide_quick_capture_window(&app)
        .map_err(|error| CommandError::new("window_error", error.to_string()))
}

fn parse_note_id(id: &str) -> Result<NoteId, CommandError> {
    NoteId::parse(id).map_err(|_| CommandError::invalid_input("invalid note id"))
}

fn parse_action_item_id(id: &str) -> Result<ActionItemId, CommandError> {
    ActionItemId::parse(id).map_err(|_| CommandError::invalid_input("invalid action item id"))
}

#[cfg(test)]
mod tests {
    use crate::commands::{AppSettingsDto, NoteListItemDto};
    use crate::domain::{ParseStatus, ReviewStatus};
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn command_dtos_serialize_camel_case_fields() {
        let item = NoteListItemDto {
            id: "note-1".to_string(),
            title: "raw".to_string(),
            raw_text: "raw".to_string(),
            cleaned_text: None,
            summary: Some("summary".to_string()),
            capture_source: "quick_capture".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            parse_status: ParseStatus::Queued,
            review_status: ReviewStatus::None,
            is_archived: false,
            tags: Vec::new(),
            tag_count: 2,
            action_item_count: 3,
            suggested_action_item_count: 1,
        };
        let serialized = serde_json::to_value(item).unwrap();

        assert!(serialized.get("rawText").is_some());
        assert!(serialized.get("title").is_some());
        assert!(serialized.get("cleanedText").is_some());
        assert!(serialized.get("captureSource").is_some());
        assert!(serialized.get("parseStatus").is_some());
        assert!(serialized.get("reviewStatus").is_some());
        assert!(serialized.get("isArchived").is_some());
        assert!(serialized.get("tags").is_some());
        assert!(serialized.get("tagCount").is_some());
        assert!(serialized.get("actionItemCount").is_some());
        assert!(serialized.get("suggestedActionItemCount").is_some());
    }

    #[test]
    fn settings_dto_accepts_camel_case_fields() {
        let settings: AppSettingsDto = serde_json::from_value(json!({
            "launchAtStartup": true,
            "minimizeToTray": true,
            "globalHotkey": "Ctrl+Alt+Space",
            "theme": "dark",
            "parserTimeoutSeconds": 45,
            "parserMaxRetries": 4,
            "codexCommandPath": "codex"
        }))
        .unwrap();

        assert_eq!(settings.parser_timeout_seconds, 45);
        assert_eq!(settings.parser_max_retries, 4);
        assert_eq!(settings.codex_command_path, "codex");
    }
}
