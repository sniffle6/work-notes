use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::Database;

use super::{ServiceError, ServiceResult};

const APP_SETTINGS_KEY: &str = "app_settings";

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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            minimize_to_tray: true,
            global_hotkey: "Ctrl+Alt+Space".to_string(),
            theme: "dark".to_string(),
            parser_timeout_seconds: 30,
            parser_max_retries: 3,
            codex_command_path: "codex".to_string(),
        }
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

    pub fn save(&self, settings: AppSettings) -> ServiceResult<AppSettings> {
        if settings.codex_command_path.trim().is_empty() {
            return Err(ServiceError::InvalidInput("codex command path is required"));
        }
        if settings.parser_timeout_seconds == 0 {
            return Err(ServiceError::InvalidInput(
                "parser timeout must be positive",
            ));
        }

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
