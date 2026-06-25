use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::db::Database;
use crate::parser::DEFAULT_CODEX_PROGRAM;

use super::{ServiceError, ServiceResult};

const APP_SETTINGS_KEY: &str = "app_settings";
pub const DEFAULT_PARSER_TIMEOUT_SECONDS: u64 = 90;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub launch_at_startup: bool,
    pub minimize_to_tray: bool,
    pub global_hotkey: String,
    pub theme: String,
    pub parser_timeout_seconds: u64,
    pub parser_max_retries: u32,
    pub codex_command_path: String,
    #[serde(default)]
    pub linked_workspace_paths: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            minimize_to_tray: true,
            global_hotkey: "Ctrl+Shift+Space".to_string(),
            theme: "dark-compact".to_string(),
            parser_timeout_seconds: DEFAULT_PARSER_TIMEOUT_SECONDS,
            parser_max_retries: 3,
            codex_command_path: DEFAULT_CODEX_PROGRAM.to_string(),
            linked_workspace_paths: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;

    use super::{AppSettings, SettingsService};

    #[test]
    fn default_parser_timeout_covers_slower_codex_runs() {
        assert_eq!(AppSettings::default().parser_timeout_seconds, 90);
    }

    #[test]
    fn app_settings_accepts_missing_linked_workspaces() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
                "launchAtStartup": false,
                "minimizeToTray": true,
                "globalHotkey": "Ctrl+Shift+Space",
                "theme": "dark-compact",
                "parserTimeoutSeconds": 90,
                "parserMaxRetries": 3,
                "codexCommandPath": "codex.cmd"
            }"#,
        )
        .unwrap();

        assert!(settings.linked_workspace_paths.is_empty());
    }

    #[test]
    fn save_normalizes_linked_workspace_paths() {
        let service = SettingsService::new(Database::in_memory().unwrap());
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let mut settings = AppSettings::default();
        settings.linked_workspace_paths = vec![
            format!("  {}  ", first.path().display()),
            String::new(),
            first.path().display().to_string(),
            second.path().display().to_string(),
        ];

        let saved = service.save(settings).unwrap();

        assert_eq!(
            saved.linked_workspace_paths,
            vec![
                first.path().display().to_string(),
                second.path().display().to_string(),
            ]
        );
    }

    #[test]
    fn save_rejects_missing_linked_workspace_path() {
        let service = SettingsService::new(Database::in_memory().unwrap());
        let mut settings = AppSettings::default();
        settings.linked_workspace_paths =
            vec!["C:\\definitely\\missing\\work-notes-test".to_string()];

        let error = service.save(settings).unwrap_err();

        assert!(matches!(
            error,
            super::ServiceError::InvalidInput(
                "linked workspace path must be an existing directory"
            )
        ));
    }
}

#[derive(Clone)]
pub struct SettingsService {
    database: Database,
}

impl SettingsService {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub fn get(&self) -> ServiceResult<AppSettings> {
        let connection = self.database.connection()?;
        let value = connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [APP_SETTINGS_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        value
            .map(|value| serde_json::from_str(&value).map_err(ServiceError::from))
            .unwrap_or_else(|| Ok(AppSettings::default()))
    }

    pub fn save(&self, mut settings: AppSettings) -> ServiceResult<AppSettings> {
        if settings.codex_command_path.trim().is_empty() {
            return Err(ServiceError::InvalidInput("codex command path is required"));
        }
        if settings.parser_timeout_seconds == 0 {
            return Err(ServiceError::InvalidInput(
                "parser timeout must be positive",
            ));
        }
        settings.linked_workspace_paths =
            normalize_linked_workspace_paths(std::mem::take(&mut settings.linked_workspace_paths))?;

        let value = serde_json::to_string(&settings)?;
        let connection = self.database.connection()?;
        connection.execute(
            "INSERT INTO settings (key, value)
             VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![APP_SETTINGS_KEY, value],
        )?;
        Ok(settings)
    }
}

fn normalize_linked_workspace_paths(paths: Vec<String>) -> ServiceResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for path in paths {
        let path = path.trim();
        if path.is_empty() {
            continue;
        }

        let path_ref = Path::new(path);
        if !path_ref.is_absolute() || !path_ref.is_dir() {
            return Err(ServiceError::InvalidInput(
                "linked workspace path must be an existing directory",
            ));
        }

        if seen.insert(path.to_string()) {
            normalized.push(path.to_string());
        }
    }

    Ok(normalized)
}
