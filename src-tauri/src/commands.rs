use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::domain::{
    ActionItem, ActionItemId, ActionReviewItem, ActionStatus, CardNote, FollowupItem,
    FollowupState, InboxFilters, Note, NoteId, NoteListItem, ParseStatus, ReviewStatus, Tag,
    TagAssignment, TagId, TagKind,
};
use crate::repositories::RepositoryError;
use crate::services::actions::ActionItemService;
use crate::services::archive::ArchiveService;
use crate::services::capture::CaptureService;
use crate::services::note_edits::NoteEditService;
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
    pub completed_at: Option<DateTime<Utc>>,
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
            completed_at: item.completed_at,
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
    pub completed_at: Option<DateTime<Utc>>,
    pub completion_note: Option<String>,
    pub cleaned_edited: bool,
    pub tags: Vec<TagAssignmentDto>,
    pub action_items: Vec<ActionItemDto>,
    pub card_notes: Vec<CardNoteDto>,
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
            completed_at: note.completed_at,
            completion_note: note.completion_note,
            cleaned_edited: note.cleaned_edited,
            tags: detail.tags.into_iter().map(Into::into).collect(),
            action_items: detail.action_items.into_iter().map(Into::into).collect(),
            card_notes: detail.card_notes.into_iter().map(Into::into).collect(),
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
            completed_at: note.completed_at,
            completion_note: note.completion_note,
            cleaned_edited: note.cleaned_edited,
            tags: Vec::new(),
            action_items: Vec::new(),
            card_notes: Vec::new(),
            parse_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardNoteDto {
    pub id: String,
    pub note_id: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

impl From<CardNote> for CardNoteDto {
    fn from(note: CardNote) -> Self {
        Self {
            id: note.id.to_string(),
            note_id: note.note_id.to_string(),
            text: note.text,
            created_at: note.created_at,
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
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
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
            followup_state: action_item.followup_state,
            followup_lane: action_item.followup_lane,
            completed_at: action_item.completed_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowupItemDto {
    pub id: String,
    pub note_id: String,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: ActionStatus,
    pub source: String,
    pub confidence: Option<f64>,
    pub followup_state: Option<FollowupState>,
    pub followup_lane: Option<String>,
    pub tags: Vec<TagAssignmentDto>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<FollowupItem> for FollowupItemDto {
    fn from(item: FollowupItem) -> Self {
        Self {
            id: item.id.to_string(),
            note_id: item.note_id.to_string(),
            note_title: item.note_title,
            text: item.text,
            owner: item.owner,
            due_date: item.due_date,
            status: item.status,
            source: item.source,
            confidence: item.confidence,
            followup_state: item.followup_state,
            followup_lane: item.followup_lane,
            tags: item.tags.into_iter().map(Into::into).collect(),
            created_at: item.created_at,
            completed_at: item.completed_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionReviewItemDto {
    pub id: String,
    pub note_id: String,
    pub note_title: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
}

impl From<ActionReviewItem> for ActionReviewItemDto {
    fn from(action_item: ActionReviewItem) -> Self {
        Self {
            id: action_item.id.to_string(),
            note_id: action_item.note_id.to_string(),
            note_title: action_item.note_title,
            text: action_item.text,
            owner: action_item.owner,
            due_date: action_item.due_date,
            confidence: action_item.confidence,
            created_at: action_item.created_at,
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
    #[serde(default)]
    pub include_completed: bool,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualFollowupDto {
    pub note_id: String,
    pub text: String,
    pub lane_override: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFollowupStateDto {
    pub id: String,
    pub state: FollowupState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFollowupLaneDto {
    pub id: String,
    pub lane_override: Option<String>,
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
            include_completed: filters.include_completed,
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
pub async fn update_note_cleaned(
    state: tauri::State<'_, AppState>,
    note_id: String,
    title: String,
    cleaned_text: String,
    summary: String,
) -> Result<NoteDetailDto, CommandError> {
    let note_id = parse_note_id(&note_id)?;
    state
        .repositories
        .notes
        .update_cleaned_by_user(note_id, &title, &cleaned_text, &summary)?;
    let detail = SearchService::new(state.repositories.clone()).get_note(note_id)?;
    Ok(detail.into())
}

#[tauri::command]
pub async fn update_note_raw(
    state: tauri::State<'_, AppState>,
    note_id: String,
    raw_text: String,
) -> Result<NoteDetailDto, CommandError> {
    let note_id = parse_note_id(&note_id)?;
    NoteEditService::new(state.repositories.clone()).update_raw_text(note_id, &raw_text)?;
    let detail = SearchService::new(state.repositories.clone()).get_note(note_id)?;
    Ok(detail.into())
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
pub async fn complete_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    completion_note: Option<String>,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    let completion_note = optional_user_note(completion_note)?;
    state
        .repositories
        .notes
        .complete(note_id, completion_note.as_deref())?;
    Ok(())
}

#[tauri::command]
pub async fn add_card_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    text: String,
) -> Result<NoteDetailDto, CommandError> {
    let note_id = parse_note_id(&note_id)?;
    let text = required_user_note(&text)?;
    SearchService::new(state.repositories.clone()).get_note(note_id)?;
    state.repositories.card_notes.add(note_id, text)?;
    let detail = SearchService::new(state.repositories.clone()).get_note(note_id)?;
    Ok(detail.into())
}

#[tauri::command]
pub async fn reopen_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    state.repositories.notes.reopen_completed(note_id)?;
    Ok(())
}

#[tauri::command]
pub async fn restore_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    ArchiveService::new(state.repositories.clone()).restore(note_id)?;
    Ok(())
}

#[tauri::command]
pub async fn permanently_delete_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<(), CommandError> {
    let note_id = parse_note_id(&note_id)?;
    ArchiveService::new(state.repositories.clone()).permanently_delete(note_id)?;
    Ok(())
}

#[tauri::command]
pub async fn accept_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).accept(action_id)?;
    Ok(())
}

#[tauri::command]
pub async fn dismiss_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).dismiss(action_id)?;
    Ok(())
}

#[tauri::command]
pub async fn complete_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).complete(action_id)?;
    Ok(())
}

#[tauri::command]
pub async fn reopen_action_item(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&action_id)?;
    ActionItemService::new(state.repositories.clone()).reopen(action_id)?;
    Ok(())
}

#[tauri::command]
pub async fn list_suggested_actions(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<ActionReviewItemDto>, CommandError> {
    let items =
        ActionItemService::new(state.repositories.clone()).list_suggested(limit.unwrap_or(100))?;

    Ok(items.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn list_followups(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<FollowupItemDto>, CommandError> {
    let items =
        ActionItemService::new(state.repositories.clone()).list_followups(limit.unwrap_or(200))?;

    Ok(items.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn create_manual_followup(
    state: tauri::State<'_, AppState>,
    input: CreateManualFollowupDto,
) -> Result<ActionItemDto, CommandError> {
    let note_id = parse_note_id(&input.note_id)?;
    let action = ActionItemService::new(state.repositories.clone()).create_manual_followup(
        note_id,
        &input.text,
        input.lane_override.as_deref(),
    )?;

    Ok(action.into())
}

#[tauri::command]
pub async fn update_followup_state(
    state: tauri::State<'_, AppState>,
    input: UpdateFollowupStateDto,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&input.id)?;
    ActionItemService::new(state.repositories.clone())
        .update_followup_state(action_id, input.state)?;
    Ok(())
}

#[tauri::command]
pub async fn update_followup_lane(
    state: tauri::State<'_, AppState>,
    input: UpdateFollowupLaneDto,
) -> Result<(), CommandError> {
    let action_id = parse_action_item_id(&input.id)?;
    ActionItemService::new(state.repositories.clone())
        .update_followup_lane(action_id, input.lane_override.as_deref())?;
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
    let normalized_hotkey = windowing::hotkey::validate_shortcut(&settings.global_hotkey)
        .map_err(|error| CommandError::new("hotkey_error", error.to_string()))?;

    let previous_settings = state.settings.get().unwrap_or_default();
    let mut settings = state
        .settings
        .validate(settings)
        .map_err(CommandError::from)?;
    settings.global_hotkey = normalized_hotkey;

    let previous_hotkey = previous_settings.global_hotkey.clone();

    if let Err(error) = windowing::hotkey::register_global_shortcut(&app, &settings.global_hotkey) {
        let _ = windowing::hotkey::register_global_shortcut(&app, &previous_hotkey);
        return Err(CommandError::new("hotkey_error", error.to_string()));
    }

    if let Err(error) =
        windowing::startup::apply_launch_at_startup_setting(&app, settings.launch_at_startup)
    {
        let _ = windowing::hotkey::register_global_shortcut(&app, &previous_hotkey);
        return Err(CommandError::new("startup_error", error.to_string()));
    }

    match state.settings.save(settings).map_err(CommandError::from) {
        Ok(saved) => Ok(saved),
        Err(error) => {
            let _ = windowing::hotkey::register_global_shortcut(&app, &previous_hotkey);
            let _ = windowing::startup::apply_launch_at_startup_setting(
                &app,
                previous_settings.launch_at_startup,
            );
            Err(error)
        }
    }
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

const MAX_USER_NOTE_CHARS: usize = 4_000;

fn required_user_note(value: &str) -> Result<&str, CommandError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CommandError::invalid_input("note text is required"));
    }
    if value.chars().count() > MAX_USER_NOTE_CHARS {
        return Err(CommandError::invalid_input("note text is too long"));
    }
    Ok(value)
}

fn optional_user_note(value: Option<String>) -> Result<Option<String>, CommandError> {
    value
        .map(|value| required_user_note(&value).map(str::to_string))
        .transpose()
}

#[cfg(test)]
mod tests {
    use crate::commands::{
        optional_user_note, required_user_note, ActionReviewItemDto, AppSettingsDto, CommandError,
        FollowupItemDto, NoteListItemDto,
    };
    use crate::domain::{
        ActionItemId, ActionReviewItem, ActionStatus, FollowupItem, FollowupState, NoteId,
        ParseStatus, ReviewStatus,
    };
    use crate::services::ServiceError;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn user_note_validation_trims_and_limits_text() {
        assert_eq!(required_user_note("  resolved  ").unwrap(), "resolved");
        assert_eq!(
            optional_user_note(Some("  shipped  ".to_string())).unwrap(),
            Some("shipped".to_string())
        );
        assert!(required_user_note("   ").is_err());
        assert!(required_user_note(&"x".repeat(4_001)).is_err());
    }

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
            completed_at: None,
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
        assert!(serialized.get("completedAt").is_some());
        assert!(serialized.get("tags").is_some());
        assert!(serialized.get("tagCount").is_some());
        assert!(serialized.get("actionItemCount").is_some());
        assert!(serialized.get("suggestedActionItemCount").is_some());
    }

    #[test]
    fn action_review_item_dto_serializes_camel_case_fields() {
        let item = ActionReviewItem {
            id: ActionItemId::new(),
            note_id: NoteId::new(),
            note_title: "Kiosk 7 telemetry IDs".to_string(),
            text: "Bring serial list into the Tuesday sync.".to_string(),
            owner: Some("Maya".to_string()),
            due_date: None,
            confidence: Some(0.82),
            created_at: Utc::now(),
        };

        let serialized = serde_json::to_value(ActionReviewItemDto::from(item)).unwrap();

        assert!(serialized.get("id").is_some());
        assert!(serialized.get("noteId").is_some());
        assert!(serialized.get("noteTitle").is_some());
        assert!(serialized.get("dueDate").is_some());
        assert!(serialized.get("createdAt").is_some());
    }

    #[test]
    fn followup_item_dto_serializes_camel_case_fields() {
        let item = FollowupItem {
            id: ActionItemId::new(),
            note_id: NoteId::new(),
            note_title: "Kiosk 7 telemetry IDs".to_string(),
            text: "Bring serial list into the Tuesday sync.".to_string(),
            owner: Some("Maya".to_string()),
            due_date: None,
            status: ActionStatus::Accepted,
            source: "parser".to_string(),
            confidence: Some(0.82),
            followup_state: Some(FollowupState::Waiting),
            followup_lane: Some("Ops".to_string()),
            tags: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
        };

        let serialized = serde_json::to_value(FollowupItemDto::from(item)).unwrap();

        assert!(serialized.get("id").is_some());
        assert!(serialized.get("noteId").is_some());
        assert!(serialized.get("noteTitle").is_some());
        assert!(serialized.get("dueDate").is_some());
        assert!(serialized.get("followupState").is_some());
        assert!(serialized.get("followupLane").is_some());
        assert!(serialized.get("createdAt").is_some());
        assert!(serialized.get("completedAt").is_some());
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
            "codexCommandPath": "codex",
            "linkedWorkspacePaths": ["C:\\code\\work-notes", "D:\\scratch\\demo"]
        }))
        .unwrap();

        assert_eq!(settings.parser_timeout_seconds, 45);
        assert_eq!(settings.parser_max_retries, 4);
        assert_eq!(settings.codex_command_path, "codex");
        assert_eq!(
            settings.linked_workspace_paths,
            vec![
                "C:\\code\\work-notes".to_string(),
                "D:\\scratch\\demo".to_string(),
            ]
        );
    }

    #[test]
    fn service_invalid_input_maps_to_command_invalid_input() {
        let error = CommandError::from(ServiceError::InvalidInput(
            "note must be archived before permanent delete",
        ));

        assert_eq!(error.code, "invalid_input");
    }

    #[test]
    fn service_not_found_maps_to_command_not_found() {
        let error = CommandError::from(ServiceError::NotFound {
            entity: "note",
            id: "note-missing".to_string(),
        });

        assert_eq!(error.code, "not_found");
    }
}
