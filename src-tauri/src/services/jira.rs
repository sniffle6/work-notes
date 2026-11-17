//! Jira setup: where credentials come from, which filters are shown, and how
//! a client is built from settings.

use serde::Serialize;

use crate::jira::transport::{resolve_cloud_id, GatewayTransport, JiraCredentials};
use crate::jira::{IssueUpdate, JiraClient, JiraError, JiraIssueDetail};

use super::settings::{AppSettings, JiraFilter, SettingsService};

/// Atlassian account email. `BITBUCKET_EMAIL` is accepted because existing
/// Atlassian tooling already sets it for the same account.
pub const EMAIL_ENV_VARS: [&str; 2] = ["JIRA_EMAIL", "BITBUCKET_EMAIL"];
/// Jira-scoped API token (`read:jira-work` and `write:jira-work`).
pub const API_TOKEN_ENV_VAR: &str = "JIRA_API_TOKEN";
/// Optional override for the cloud ID normally resolved from the site URL.
pub const CLOUD_ID_ENV_VAR: &str = "JIRA_CLOUD_ID";

/// Jira credentials supplied through environment variables. The API token is
/// never written to SQLite or sent to the chat agent.
#[derive(Clone, Default)]
pub struct JiraEnvironment {
    pub email: Option<String>,
    pub api_token: Option<String>,
    pub cloud_id: Option<String>,
}

impl JiraEnvironment {
    /// Reads the variables this process was started with. Changes made
    /// afterwards need an app restart.
    pub fn from_process() -> Self {
        let read = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        Self {
            email: EMAIL_ENV_VARS.iter().find_map(|name| read(name)),
            api_token: read(API_TOKEN_ENV_VAR),
            cloud_id: read(CLOUD_ID_ENV_VAR),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraStatus {
    pub configured: bool,
    /// Setup steps still missing, worded for the user.
    pub missing: Vec<String>,
    pub site_url: String,
    pub project_key: String,
    pub filters: Vec<JiraFilter>,
}

/// Reports whether Jira can be used and what setup is still missing.
/// Performs no network I/O.
pub fn jira_status(settings: &AppSettings, environment: &JiraEnvironment) -> JiraStatus {
    let mut missing = Vec::new();
    if settings.jira_site_url.is_empty() {
        missing.push("Enter your Jira site URL in Settings > Jira.".to_string());
    }
    if environment.email.is_none() {
        missing.push(format!(
            "Set the {} (or {}) environment variable to your Atlassian account email, then restart Work Notes.",
            EMAIL_ENV_VARS[0], EMAIL_ENV_VARS[1]
        ));
    }
    if environment.api_token.is_none() {
        missing.push(format!(
            "Set the {API_TOKEN_ENV_VAR} environment variable to a Jira API token, then restart Work Notes."
        ));
    }

    JiraStatus {
        configured: missing.is_empty(),
        missing,
        site_url: settings.jira_site_url.clone(),
        project_key: settings.jira_project_key.clone(),
        filters: effective_filters(settings),
    }
}

const OPEN_WORK_ORDER: &str = "statusCategory != Done ORDER BY priority DESC, updated DESC";

/// Filters shown in the ticket list: the user's own, or built-in ones.
pub fn effective_filters(settings: &AppSettings) -> Vec<JiraFilter> {
    if !settings.jira_filters.is_empty() {
        return settings.jira_filters.clone();
    }

    let filter = |name: &str, jql: String| JiraFilter {
        name: name.to_string(),
        jql,
    };
    let project_key = &settings.jira_project_key;
    if project_key.is_empty() {
        return vec![filter(
            "My open tickets",
            format!("assignee = currentUser() AND {OPEN_WORK_ORDER}"),
        )];
    }

    vec![
        filter(
            "My open tickets",
            format!("project = {project_key} AND assignee = currentUser() AND {OPEN_WORK_ORDER}"),
        ),
        filter(
            "All open",
            format!("project = {project_key} AND {OPEN_WORK_ORDER}"),
        ),
        filter(
            "Unassigned",
            format!("project = {project_key} AND assignee is EMPTY AND {OPEN_WORK_ORDER}"),
        ),
    ]
}

/// The ticket text an edit started from. `None` when it is unknown, as for an
/// edit kept from before the app recorded it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditBase {
    pub summary: Option<String>,
    pub description: Option<String>,
}

/// A field an edit would overwrite although Jira changed it since the edit
/// started, or although the start is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditConflict {
    Summary,
    Description,
}

impl EditConflict {
    pub fn field_name(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Description => "description",
        }
    }
}

/// Lists the fields `update` would overwrite although Jira's `current` value
/// no longer matches the edit's `base`. Jira has no conditional update, so
/// callers check this just before writing.
pub fn edit_conflicts(
    current: &JiraIssueDetail,
    update: &IssueUpdate,
    base: &EditBase,
) -> Vec<EditConflict> {
    let fields = [
        (EditConflict::Summary, &update.summary, &base.summary, &current.summary.summary),
        (EditConflict::Description, &update.description, &base.description, &current.description),
    ];
    fields
        .into_iter()
        .filter(|(_, written, started_from, now)| match (written, started_from) {
            (None, _) => false,
            // Writing what Jira already holds overwrites nothing.
            (Some(written), _) if same_text(written, now) => false,
            (Some(_), Some(started_from)) => !same_text(started_from, now),
            (Some(_), None) => true,
        })
        .map(|(conflict, ..)| conflict)
        .collect()
}

// Compares ticket text the way a reader sees it: line endings and whitespace
// at line ends do not count.
fn same_text(left: &str, right: &str) -> bool {
    let lines = |text: &str| {
        text.replace("\r\n", "\n")
            .trim_end()
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
    };
    lines(left) == lines(right)
}

#[derive(Clone)]
pub struct JiraConnection {
    pub credentials: JiraCredentials,
    pub cloud_id: String,
    pub site_url: String,
}

/// Works out how to reach Jira from settings and environment.
///
/// `lookup_cloud_id` is called with the site URL only when neither settings
/// nor environment provide a cloud ID. Returns the connection and whether the
/// cloud ID came from that lookup, so the caller can store it.
pub fn resolve_connection(
    settings: &AppSettings,
    environment: &JiraEnvironment,
    lookup_cloud_id: impl FnOnce(&str) -> Result<String, JiraError>,
) -> Result<(JiraConnection, bool), JiraError> {
    let status = jira_status(settings, environment);
    let (Some(email), Some(api_token)) = (&environment.email, &environment.api_token) else {
        return Err(JiraError::NotConfigured(status.missing.join(" ")));
    };
    if !status.configured {
        return Err(JiraError::NotConfigured(status.missing.join(" ")));
    }

    let known_cloud_id = environment
        .cloud_id
        .clone()
        .or_else(|| Some(settings.jira_cloud_id.clone()).filter(|cloud_id| !cloud_id.is_empty()));
    let newly_resolved = known_cloud_id.is_none();
    let cloud_id = match known_cloud_id {
        Some(cloud_id) => cloud_id,
        None => lookup_cloud_id(&settings.jira_site_url)?,
    };

    Ok((
        JiraConnection {
            credentials: JiraCredentials {
                email: email.clone(),
                api_token: api_token.clone(),
            },
            cloud_id,
            site_url: settings.jira_site_url.clone(),
        },
        newly_resolved,
    ))
}

/// Builds a Jira client from saved settings and the process environment.
///
/// The first call for a site looks up its cloud ID over the network and
/// saves it to settings.
pub fn connect(settings: &SettingsService) -> Result<JiraClient<GatewayTransport>, JiraError> {
    let app_settings = settings.get().map_err(|error| {
        JiraError::NotConfigured(format!("settings could not be read: {error}"))
    })?;
    let (connection, newly_resolved) = resolve_connection(
        &app_settings,
        &JiraEnvironment::from_process(),
        resolve_cloud_id,
    )?;
    if newly_resolved {
        let mut updated = app_settings;
        updated.jira_cloud_id = connection.cloud_id.clone();
        // Best effort: a failed save only means the lookup repeats next time.
        let _ = settings.save(updated);
    }
    let transport = GatewayTransport::new(connection.credentials, &connection.cloud_id)?;
    Ok(JiraClient::new(transport, connection.site_url))
}

#[cfg(test)]
mod tests {
    use super::{
        edit_conflicts, effective_filters, jira_status, resolve_connection, EditBase,
        EditConflict, JiraEnvironment,
    };
    use crate::jira::{IssueKey, IssueUpdate, JiraError, JiraIssueDetail, JiraIssueSummary};

    fn ticket(summary: &str, description: &str) -> JiraIssueDetail {
        JiraIssueDetail {
            summary: JiraIssueSummary {
                key: IssueKey::parse("AB-7").unwrap(),
                summary: summary.to_string(),
                status: "To Do".to_string(),
                status_category: "new".to_string(),
                issue_type: "Bug".to_string(),
                priority: None,
                assignee: None,
                updated: None,
                labels: vec![],
                url: String::new(),
            },
            description: description.to_string(),
            description_editable_losslessly: true,
            reporter: None,
            parent_key: None,
            subtasks: vec![],
            linked_issues: vec![],
            attachments: vec![],
            comments: vec![],
            transitions: vec![],
        }
    }

    fn base(summary: &str, description: &str) -> EditBase {
        EditBase {
            summary: Some(summary.to_string()),
            description: Some(description.to_string()),
        }
    }

    fn update(summary: Option<&str>, description: Option<&str>) -> IssueUpdate {
        IssueUpdate {
            summary: summary.map(str::to_string),
            description: description.map(str::to_string),
        }
    }

    #[test]
    fn an_edit_of_text_jira_still_holds_has_no_conflict() {
        let current = ticket("Printer jams", "Jams after reboot.\n");

        assert!(edit_conflicts(
            &current,
            &update(Some("Printer jams on reboot"), Some("Mine")),
            // Line endings and trailing whitespace are not changes.
            &base("Printer jams", "Jams after reboot.\r\n  ")
        )
        .is_empty());
    }

    #[test]
    fn a_field_changed_in_jira_since_the_edit_started_is_a_conflict() {
        let current = ticket("Renamed by Maya", "Maya's new steps");

        assert_eq!(
            edit_conflicts(
                &current,
                &update(Some("Mine"), Some("Mine")),
                &base("Printer jams", "Old steps")
            ),
            vec![EditConflict::Summary, EditConflict::Description]
        );
    }

    #[test]
    fn fields_the_edit_leaves_alone_never_conflict() {
        let current = ticket("Renamed by Maya", "Old steps");

        assert!(edit_conflicts(
            &current,
            &update(None, Some("Mine")),
            &base("Printer jams", "Old steps")
        )
        .is_empty());
    }

    #[test]
    fn writing_what_jira_already_holds_is_not_a_conflict() {
        let current = ticket("Printer jams", "Same fix");

        assert!(edit_conflicts(
            &current,
            &update(None, Some("Same fix")),
            &base("Printer jams", "Old steps")
        )
        .is_empty());
    }

    #[test]
    fn an_unknown_start_is_a_conflict_for_the_fields_being_written() {
        let current = ticket("Printer jams", "Old steps");

        assert_eq!(
            edit_conflicts(&current, &update(None, Some("Mine")), &EditBase::default()),
            vec![EditConflict::Description]
        );
    }
    use crate::services::settings::{AppSettings, JiraFilter};

    fn configured_settings() -> AppSettings {
        AppSettings {
            jira_site_url: "https://example.atlassian.net".to_string(),
            jira_project_key: "AB".to_string(),
            ..AppSettings::default()
        }
    }

    fn environment() -> JiraEnvironment {
        JiraEnvironment {
            email: Some("dev@example.test".to_string()),
            api_token: Some("token".to_string()),
            cloud_id: None,
        }
    }

    fn no_lookup(_: &str) -> Result<String, JiraError> {
        panic!("cloud id lookup should not run")
    }

    #[test]
    fn status_is_configured_with_site_and_credentials() {
        let status = jira_status(&configured_settings(), &environment());

        assert!(status.configured);
        assert!(status.missing.is_empty());
        assert_eq!(status.site_url, "https://example.atlassian.net");
        assert_eq!(status.project_key, "AB");
        assert_eq!(status.filters.len(), 3);
    }

    #[test]
    fn status_lists_each_missing_setup_step() {
        let status = jira_status(&AppSettings::default(), &JiraEnvironment::default());

        assert!(!status.configured);
        assert_eq!(
            status.missing,
            vec![
                "Enter your Jira site URL in Settings > Jira.".to_string(),
                "Set the JIRA_EMAIL (or BITBUCKET_EMAIL) environment variable to your Atlassian account email, then restart Work Notes.".to_string(),
                "Set the JIRA_API_TOKEN environment variable to a Jira API token, then restart Work Notes.".to_string(),
            ]
        );
    }

    #[test]
    fn built_in_filters_scope_to_the_project() {
        let filters = effective_filters(&configured_settings());

        assert_eq!(
            filters,
            vec![
                JiraFilter {
                    name: "My open tickets".to_string(),
                    jql: "project = AB AND assignee = currentUser() AND statusCategory != Done ORDER BY priority DESC, updated DESC".to_string(),
                },
                JiraFilter {
                    name: "All open".to_string(),
                    jql: "project = AB AND statusCategory != Done ORDER BY priority DESC, updated DESC".to_string(),
                },
                JiraFilter {
                    name: "Unassigned".to_string(),
                    jql: "project = AB AND assignee is EMPTY AND statusCategory != Done ORDER BY priority DESC, updated DESC".to_string(),
                },
            ]
        );
    }

    #[test]
    fn built_in_filter_without_a_project_covers_my_tickets_only() {
        let filters = effective_filters(&AppSettings::default());

        assert_eq!(
            filters,
            vec![JiraFilter {
                name: "My open tickets".to_string(),
                jql: "assignee = currentUser() AND statusCategory != Done ORDER BY priority DESC, updated DESC".to_string(),
            }]
        );
    }

    #[test]
    fn custom_filters_replace_the_built_in_ones() {
        let mut settings = configured_settings();
        settings.jira_filters = vec![JiraFilter {
            name: "Cabinet bugs".to_string(),
            jql: "project = AB AND type = Bug".to_string(),
        }];

        assert_eq!(effective_filters(&settings), settings.jira_filters);
    }

    #[test]
    fn stored_cloud_id_is_used_without_a_lookup() {
        let mut settings = configured_settings();
        settings.jira_cloud_id = "cloud-stored".to_string();

        let (connection, newly_resolved) =
            resolve_connection(&settings, &environment(), no_lookup).unwrap();

        assert_eq!(connection.cloud_id, "cloud-stored");
        assert_eq!(connection.site_url, "https://example.atlassian.net");
        assert_eq!(connection.credentials.email, "dev@example.test");
        assert_eq!(connection.credentials.api_token, "token");
        assert!(!newly_resolved);
    }

    #[test]
    fn environment_cloud_id_overrides_the_stored_one() {
        let mut settings = configured_settings();
        settings.jira_cloud_id = "cloud-stored".to_string();
        let mut environment = environment();
        environment.cloud_id = Some("cloud-env".to_string());

        let (connection, newly_resolved) =
            resolve_connection(&settings, &environment, no_lookup).unwrap();

        assert_eq!(connection.cloud_id, "cloud-env");
        assert!(!newly_resolved);
    }

    #[test]
    fn missing_cloud_id_is_looked_up_from_the_site() {
        let (connection, newly_resolved) =
            resolve_connection(&configured_settings(), &environment(), |site_url| {
                assert_eq!(site_url, "https://example.atlassian.net");
                Ok("cloud-found".to_string())
            })
            .unwrap();

        assert_eq!(connection.cloud_id, "cloud-found");
        assert!(newly_resolved);
    }

    /// Manual, read-only check against a real Jira site. Run with:
    /// `WORK_NOTES_JIRA_SITE_URL=https://team.atlassian.net cargo test live_jira -- --ignored --nocapture`
    /// plus the email and API token environment variables.
    #[test]
    #[ignore = "talks to a real Jira site"]
    fn live_jira_read_only_smoke() {
        use crate::jira::transport::{resolve_cloud_id, GatewayTransport};
        use crate::jira::{JiraClient, JiraOps};

        let settings = AppSettings {
            jira_site_url: std::env::var("WORK_NOTES_JIRA_SITE_URL").expect("site url"),
            ..AppSettings::default()
        };
        let (connection, newly_resolved) = resolve_connection(
            &settings,
            &JiraEnvironment::from_process(),
            resolve_cloud_id,
        )
        .expect("resolve connection");
        assert!(newly_resolved);
        let client = JiraClient::new(
            GatewayTransport::new(connection.credentials, &connection.cloud_id).expect("transport"),
            connection.site_url,
        );

        let me = client.myself().expect("myself");
        assert!(!me.account_id.is_empty());

        let filters = effective_filters(&settings);
        let issues = client.search(&filters[0].jql, 3).expect("search");
        println!("search returned {} issue(s)", issues.len());
        let Some(first) = issues.first() else {
            return;
        };
        assert!(!first.summary.is_empty());
        assert_ne!(first.status, "Unknown");
        assert_ne!(first.status_category, "unknown");

        let detail = client.get_issue(&first.key).expect("get issue");
        println!(
            "detail: description_chars={} comments={} transitions={} lossless={}",
            detail.description.chars().count(),
            detail.comments.len(),
            detail.transitions.len(),
            detail.description_editable_losslessly
        );
        assert!(!detail.transitions.is_empty());

        let types = client
            .issue_types(first.key.project_key())
            .expect("issue types");
        println!("issue types: {}", types.len());
        assert!(!types.is_empty());
    }

    #[test]
    fn incomplete_setup_fails_before_any_lookup() {
        let error = resolve_connection(
            &configured_settings(),
            &JiraEnvironment::default(),
            no_lookup,
        )
        .err()
        .unwrap();

        let JiraError::NotConfigured(message) = error else {
            panic!("expected not configured");
        };
        assert!(message.contains("JIRA_API_TOKEN"), "{message}");
    }
}
