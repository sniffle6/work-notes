//! Tauri command for the theme agent.

use std::time::Duration;

use super::CommandError;
use crate::app_state::AppState;
use crate::theme::{self, CodexThemeRunner, GeneratedTheme, ThemeAgentError};

/// A theme turn only writes a handful of colours, so it needs less time
/// than a note parse.
const THEME_TURN_TIMEOUT: Duration = Duration::from_secs(120);

impl From<ThemeAgentError> for CommandError {
    fn from(error: ThemeAgentError) -> Self {
        let code = match error {
            ThemeAgentError::EmptyRequest | ThemeAgentError::RequestTooLong => "invalid_input",
            ThemeAgentError::Run(_) | ThemeAgentError::InvalidOutput(_) => "agent_error",
        };
        Self::new(code, error.to_string())
    }
}

/// Asks the theme agent for a theme matching `request`. With `base`, the
/// agent refines that theme instead of starting over. Nothing is saved; the
/// caller keeps the result by adding it to the settings' custom themes.
#[tauri::command]
pub async fn generate_theme(
    state: tauri::State<'_, AppState>,
    request: String,
    base: Option<GeneratedTheme>,
) -> Result<GeneratedTheme, CommandError> {
    let settings = state.settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let runner = CodexThemeRunner {
            program: settings.get()?.codex_command_path,
            timeout: THEME_TURN_TIMEOUT,
        };
        Ok(theme::generate_theme(&runner, &request, base.as_ref())?)
    })
    .await
    .map_err(|_| CommandError::new("task_error", "background task failed"))?
}

#[cfg(test)]
mod tests {
    use super::CommandError;
    use crate::theme::ThemeAgentError;

    #[test]
    fn theme_errors_keep_their_message_and_separate_bad_input_from_agent_failures() {
        let empty = CommandError::from(ThemeAgentError::EmptyRequest);
        assert_eq!(empty.code, "invalid_input");
        assert_eq!(empty.message, "describe the theme you want");

        let failed = CommandError::from(ThemeAgentError::Run("codex was not found".to_string()));
        assert_eq!(failed.code, "agent_error");
        assert_eq!(
            failed.message,
            "the theme agent could not run: codex was not found"
        );
    }
}
