use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::db::Database;
use crate::jira::transport::{normalize_site_url, validate_cloud_id};
use crate::jira::types::normalize_project_key;
use crate::parser::DEFAULT_CODEX_PROGRAM;
use crate::theme::{ThemeMode, ThemeTokens, MAX_THEME_NAME_CHARS};

use super::{ServiceError, ServiceResult};

const APP_SETTINGS_KEY: &str = "app_settings";
pub const DEFAULT_PARSER_TIMEOUT_SECONDS: u64 = 90;
const CUSTOM_THEME_ID_PREFIX: &str = "custom-";
const MAX_CUSTOM_THEME_SLUG_CHARS: usize = 48;
const MAX_CUSTOM_THEMES: usize = 30;

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
    /// `https://host` origin of the Jira Cloud site; empty when Jira is unused.
    #[serde(default)]
    pub jira_site_url: String,
    /// Cloud ID for `jira_site_url`. Resolved by the app; cleared when the
    /// site changes.
    #[serde(default)]
    pub jira_cloud_id: String,
    #[serde(default)]
    pub jira_project_key: String,
    /// Custom ticket filters. Empty means the built-in filters are used.
    #[serde(default)]
    pub jira_filters: Vec<JiraFilter>,
    /// Themes the user built with the theme agent. `theme` may name one.
    #[serde(default)]
    pub custom_themes: Vec<CustomTheme>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraFilter {
    pub name: String,
    pub jql: String,
}

/// A saved theme. `id` starts with `custom-` so it cannot shadow a built-in
/// theme, and `mode` always follows the background colour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTheme {
    pub id: String,
    pub label: String,
    pub mode: ThemeMode,
    pub tokens: ThemeTokens,
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
            jira_site_url: String::new(),
            jira_cloud_id: String::new(),
            jira_project_key: String::new(),
            jira_filters: Vec::new(),
            custom_themes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::theme::tests::dark_tokens;
    use crate::theme::ThemeMode;

    use super::{AppSettings, JiraFilter, SettingsService};

    fn filter(name: &str, jql: &str) -> JiraFilter {
        JiraFilter {
            name: name.to_string(),
            jql: jql.to_string(),
        }
    }

    #[test]
    fn save_normalizes_jira_settings() {
        let service = SettingsService::new(Database::in_memory().unwrap());
        let mut settings = AppSettings::default();
        settings.jira_site_url = " https://Example.atlassian.net/jira/projects ".to_string();
        settings.jira_project_key = " ab ".to_string();
        settings.jira_filters = vec![
            filter("  Mine ", " assignee = currentUser() "),
            filter("", "  "),
        ];

        let saved = service.save(settings).unwrap();

        assert_eq!(saved.jira_site_url, "https://example.atlassian.net");
        assert_eq!(saved.jira_project_key, "AB");
        assert_eq!(
            saved.jira_filters,
            vec![filter("Mine", "assignee = currentUser()")]
        );
    }

    fn custom_theme(id: &str, label: &str) -> super::CustomTheme {
        super::CustomTheme {
            id: id.to_string(),
            label: label.to_string(),
            mode: ThemeMode::Light,
            tokens: dark_tokens(),
        }
    }

    #[test]
    fn save_normalizes_custom_themes() {
        let service = SettingsService::new(Database::in_memory().unwrap());
        let mut theme = custom_theme("custom-night-desk", "  Night Desk  ");
        theme.tokens.accent_hot = "#6FC7BD".to_string();
        let mut settings = AppSettings::default();
        settings.custom_themes = vec![theme];

        let saved = service.save(settings).unwrap();

        assert_eq!(saved.custom_themes.len(), 1);
        assert_eq!(saved.custom_themes[0].label, "Night Desk");
        assert_eq!(saved.custom_themes[0].tokens.accent_hot, "#6fc7bd");
        // The mode always follows the background colour.
        assert_eq!(saved.custom_themes[0].mode, ThemeMode::Dark);
        assert_eq!(service.get().unwrap().custom_themes, saved.custom_themes);
    }

    #[test]
    fn save_rejects_unusable_custom_themes() {
        let service = SettingsService::new(Database::in_memory().unwrap());
        let reject = |themes: Vec<super::CustomTheme>, message: &'static str| {
            let mut settings = AppSettings::default();
            settings.custom_themes = themes;
            match service.save(settings) {
                Err(super::ServiceError::InvalidInput(actual)) => assert_eq!(actual, message),
                other => panic!("expected {message}, got {other:?}"),
            }
        };

        reject(
            vec![custom_theme("dark-compact", "Shadow")],
            "a custom theme id must look like custom-name",
        );
        reject(
            vec![custom_theme("custom-a", "   ")],
            "a custom theme needs a name",
        );
        reject(
            vec![custom_theme("custom-a", &"x".repeat(41))],
            "a custom theme name is too long",
        );
        reject(
            vec![
                custom_theme("custom-a", "One"),
                custom_theme("custom-a", "Two"),
            ],
            "custom theme ids must be unique",
        );
        let mut bad_colour = custom_theme("custom-a", "One");
        bad_colour.tokens.app_bg = "navy".to_string();
        reject(
            vec![bad_colour],
            "a custom theme colour must be a hex colour like #1a2b3c",
        );
    }

    #[test]
    fn settings_saved_before_custom_themes_still_load() {
        let stored = serde_json::to_value(AppSettings::default()).unwrap();
        let mut stored = stored.as_object().unwrap().clone();
        stored.remove("customThemes");

        let loaded: AppSettings = serde_json::from_value(stored.into()).unwrap();

        assert!(loaded.custom_themes.is_empty());
    }

    #[test]
    fn save_rejects_invalid_jira_settings() {
        let service = SettingsService::new(Database::in_memory().unwrap());

        let mut bad_site = AppSettings::default();
        bad_site.jira_site_url = "http://example.atlassian.net".to_string();
        assert!(matches!(
            service.save(bad_site),
            Err(super::ServiceError::InvalidInput(
                "Jira site URL must look like https://your-team.atlassian.net"
            ))
        ));

        let mut bad_project = AppSettings::default();
        bad_project.jira_project_key = "A B".to_string();
        assert!(matches!(
            service.save(bad_project),
            Err(super::ServiceError::InvalidInput(
                "Jira project key must look like ABC"
            ))
        ));

        let mut half_filter = AppSettings::default();
        half_filter.jira_filters = vec![filter("Mine", " ")];
        assert!(matches!(
            service.save(half_filter),
            Err(super::ServiceError::InvalidInput(
                "each Jira filter needs a name and a JQL query"
            ))
        ));
    }

    #[test]
    fn changing_the_jira_site_clears_the_stored_cloud_id() {
        let service = SettingsService::new(Database::in_memory().unwrap());
        let mut settings = AppSettings::default();
        settings.jira_site_url = "https://one.atlassian.net".to_string();
        settings.jira_cloud_id = "cloud-one".to_string();
        let saved = service.save(settings).unwrap();
        assert_eq!(saved.jira_cloud_id, "cloud-one");

        let mut same_site = saved.clone();
        same_site.theme = "light".to_string();
        assert_eq!(service.save(same_site).unwrap().jira_cloud_id, "cloud-one");

        let mut moved = saved;
        moved.jira_site_url = "https://two.atlassian.net".to_string();
        assert_eq!(service.save(moved).unwrap().jira_cloud_id, "");
    }

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
        assert_eq!(settings.jira_site_url, "");
        assert_eq!(settings.jira_project_key, "");
        assert!(settings.jira_filters.is_empty());
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

    pub fn validate(&self, settings: AppSettings) -> ServiceResult<AppSettings> {
        validate_settings(settings)
    }

    pub fn save(&self, settings: AppSettings) -> ServiceResult<AppSettings> {
        let mut settings = validate_settings(settings)?;
        // A cloud ID belongs to one site. When the site changes and the cloud
        // ID was merely carried over, drop it so it is looked up again.
        let previous = self.get().unwrap_or_default();
        if previous.jira_site_url != settings.jira_site_url
            && previous.jira_cloud_id == settings.jira_cloud_id
        {
            settings.jira_cloud_id.clear();
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

fn validate_settings(mut settings: AppSettings) -> ServiceResult<AppSettings> {
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
    normalize_jira_settings(&mut settings)?;
    settings.custom_themes = normalize_custom_themes(std::mem::take(&mut settings.custom_themes))?;

    Ok(settings)
}

fn normalize_custom_themes(themes: Vec<CustomTheme>) -> ServiceResult<Vec<CustomTheme>> {
    if themes.len() > MAX_CUSTOM_THEMES {
        return Err(ServiceError::InvalidInput("too many custom themes"));
    }

    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(themes.len());
    for theme in themes {
        let slug = theme.id.strip_prefix(CUSTOM_THEME_ID_PREFIX).unwrap_or("");
        let slug_is_valid = !slug.is_empty()
            && slug.len() <= MAX_CUSTOM_THEME_SLUG_CHARS
            && slug.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            });
        if !slug_is_valid {
            return Err(ServiceError::InvalidInput(
                "a custom theme id must look like custom-name",
            ));
        }
        if !seen.insert(theme.id.clone()) {
            return Err(ServiceError::InvalidInput(
                "custom theme ids must be unique",
            ));
        }

        let label = theme.label.trim();
        if label.is_empty() {
            return Err(ServiceError::InvalidInput("a custom theme needs a name"));
        }
        if label.chars().count() > MAX_THEME_NAME_CHARS {
            return Err(ServiceError::InvalidInput(
                "a custom theme name is too long",
            ));
        }

        let tokens = theme.tokens.normalized().map_err(|_| {
            ServiceError::InvalidInput("a custom theme colour must be a hex colour like #1a2b3c")
        })?;
        normalized.push(CustomTheme {
            id: theme.id,
            label: label.to_string(),
            mode: tokens.mode(),
            tokens,
        });
    }

    Ok(normalized)
}

fn normalize_jira_settings(settings: &mut AppSettings) -> ServiceResult<()> {
    settings.jira_site_url = if settings.jira_site_url.trim().is_empty() {
        String::new()
    } else {
        normalize_site_url(&settings.jira_site_url).map_err(|_| {
            ServiceError::InvalidInput(
                "Jira site URL must look like https://your-team.atlassian.net",
            )
        })?
    };

    settings.jira_project_key = if settings.jira_project_key.trim().is_empty() {
        String::new()
    } else {
        normalize_project_key(&settings.jira_project_key)
            .map_err(|_| ServiceError::InvalidInput("Jira project key must look like ABC"))?
    };

    settings.jira_cloud_id = if settings.jira_cloud_id.trim().is_empty() {
        String::new()
    } else {
        validate_cloud_id(&settings.jira_cloud_id)
            .map_err(|_| ServiceError::InvalidInput("Jira cloud ID is malformed"))?
    };

    let mut filters = Vec::new();
    for filter in std::mem::take(&mut settings.jira_filters) {
        let name = filter.name.trim();
        let jql = filter.jql.trim();
        if name.is_empty() && jql.is_empty() {
            continue;
        }
        if name.is_empty() || jql.is_empty() {
            return Err(ServiceError::InvalidInput(
                "each Jira filter needs a name and a JQL query",
            ));
        }
        filters.push(JiraFilter {
            name: name.to_string(),
            jql: jql.to_string(),
        });
    }
    settings.jira_filters = filters;

    Ok(())
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
