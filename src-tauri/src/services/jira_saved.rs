//! Saved copies of the ticket lists and tickets Jira last returned. The Jira
//! view shows them at once while it asks Jira for current ones.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::jira::{IssueKey, JiraIssueDetail, JiraIssueSummary};
use crate::repositories::{JiraIssueCacheRepository, RepositoryResult};

use super::jira::JiraEnvironment;
use super::settings::AppSettings;

/// A saved copy as JSON in the shape the matching fresh command returns.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCopy {
    pub content: serde_json::Value,
    pub saved_at: DateTime<Utc>,
}

/// Names the Jira site and account a saved copy came from. Copies saved under
/// another site or account are never shown.
pub fn saved_copy_scope(settings: &AppSettings, environment: &JiraEnvironment) -> String {
    let site = settings
        .jira_site_url
        .trim()
        .trim_end_matches('/')
        .to_lowercase();
    let account = environment
        .email
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    serde_json::json!([site, account]).to_string()
}

pub struct JiraSavedCopies {
    repository: JiraIssueCacheRepository,
    scope: String,
}

impl JiraSavedCopies {
    pub fn new(repository: JiraIssueCacheRepository, scope: String) -> Self {
        Self { repository, scope }
    }

    /// Saves the tickets Jira listed for `jql`. Best effort: the caller already
    /// holds Jira's answer, so a failed save is skipped.
    pub fn remember_list(&self, jql: &str, issues: &[JiraIssueSummary]) {
        if let Ok(content) = serde_json::to_string(issues) {
            let _ = self.repository.save_list(&self.scope, jql, &content);
        }
    }

    /// Saves a ticket Jira returned. Best effort, like [`Self::remember_list`].
    pub fn remember_issue(&self, issue: &JiraIssueDetail) {
        if let Ok(content) = serde_json::to_string(issue) {
            let _ = self
                .repository
                .save_issue(&self.scope, &issue.summary.key, &content);
        }
    }

    /// The tickets last listed for `jql`. A copy that cannot be read is treated
    /// as missing.
    pub fn saved_list(&self, jql: &str) -> RepositoryResult<Option<SavedCopy>> {
        Ok(self
            .repository
            .load_list(&self.scope, jql)?
            .and_then(readable))
    }

    /// The ticket as Jira last returned it. A copy that cannot be read is
    /// treated as missing.
    pub fn saved_issue(&self, key: &IssueKey) -> RepositoryResult<Option<SavedCopy>> {
        Ok(self
            .repository
            .load_issue(&self.scope, key)?
            .and_then(readable))
    }
}

fn readable(saved: crate::repositories::jira_cache::SavedJiraCopy) -> Option<SavedCopy> {
    Some(SavedCopy {
        content: serde_json::from_str(&saved.content).ok()?,
        saved_at: saved.saved_at,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{saved_copy_scope, JiraSavedCopies};
    use crate::db::Database;
    use crate::jira::{IssueKey, JiraIssueDetail, JiraIssueSummary};
    use crate::repositories::JiraIssueCacheRepository;
    use crate::services::jira::JiraEnvironment;
    use crate::services::settings::AppSettings;

    fn summary(key: &str) -> JiraIssueSummary {
        JiraIssueSummary {
            key: IssueKey::parse(key).unwrap(),
            summary: "Printer jams".to_string(),
            status: "To Do".to_string(),
            status_category: "new".to_string(),
            issue_type: "Bug".to_string(),
            priority: None,
            assignee: None,
            updated: None,
            labels: vec![],
            url: format!("https://a.atlassian.net/browse/{key}"),
        }
    }

    fn detail(key: &str) -> JiraIssueDetail {
        JiraIssueDetail {
            summary: summary(key),
            description: "Jams after reboot.".to_string(),
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

    fn copies(repository: &JiraIssueCacheRepository, scope: &str) -> JiraSavedCopies {
        JiraSavedCopies::new(repository.clone(), scope.to_string())
    }

    #[test]
    fn saved_copies_return_what_the_fresh_commands_returned() {
        let saved = copies(
            &JiraIssueCacheRepository::new(Database::in_memory().unwrap()),
            "scope",
        );
        let issues = vec![summary("AB-7"), summary("AB-8")];
        let issue = detail("AB-7");

        saved.remember_list("project = AB", &issues);
        saved.remember_issue(&issue);

        assert_eq!(
            saved.saved_list("project = AB").unwrap().unwrap().content,
            serde_json::to_value(&issues).unwrap()
        );
        assert_eq!(
            saved
                .saved_issue(&IssueKey::parse("AB-7").unwrap())
                .unwrap()
                .unwrap()
                .content,
            serde_json::to_value(&issue).unwrap()
        );
        assert_eq!(saved.saved_list("project = CD").unwrap(), None);
    }

    #[test]
    fn an_unreadable_copy_is_treated_as_missing() {
        let repository = JiraIssueCacheRepository::new(Database::in_memory().unwrap());
        repository
            .save_list("scope", "project = AB", "not json")
            .unwrap();

        assert_eq!(
            copies(&repository, "scope")
                .saved_list("project = AB")
                .unwrap(),
            None
        );
    }

    #[test]
    fn the_scope_names_the_site_and_account_ignoring_case_and_trailing_slashes() {
        let settings = |site: &str| AppSettings {
            jira_site_url: site.to_string(),
            ..AppSettings::default()
        };
        let account = |email: &str| JiraEnvironment {
            email: Some(email.to_string()),
            ..JiraEnvironment::default()
        };

        let scope = saved_copy_scope(&settings("https://A.atlassian.net/"), &account("Dev@Example.test"));

        assert_eq!(scope, json!(["https://a.atlassian.net", "dev@example.test"]).to_string());
        assert_eq!(
            saved_copy_scope(&settings("https://a.atlassian.net"), &account("dev@example.test")),
            scope
        );
        assert_ne!(
            saved_copy_scope(&settings("https://b.atlassian.net"), &account("dev@example.test")),
            scope
        );
        assert_ne!(
            saved_copy_scope(&settings("https://a.atlassian.net"), &account("qa@example.test")),
            scope
        );
    }
}
