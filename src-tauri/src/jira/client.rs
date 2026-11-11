use serde_json::{json, Value};

use super::adf::{adf_round_trips_as_text, adf_to_text, text_to_adf};
use super::transitions::match_transition;
use super::transport::{JiraTransport, Method};
use super::types::{
    normalize_project_key, Assignee, IssueKey, IssueUpdate, JiraComment, JiraIssueDetail,
    JiraIssueSummary, JiraTransition, JiraUser, NewIssue, TransitionOutcome,
};
use super::JiraError;

pub const DEFAULT_SEARCH_LIMIT: usize = 200;
const SEARCH_PAGE_SIZE: usize = 100;
const COMMENT_PAGE_SIZE: usize = 100;
const MAX_COMMENTS: usize = 300;
const SEARCH_FIELDS: [&str; 7] = [
    "summary",
    "status",
    "issuetype",
    "priority",
    "assignee",
    "updated",
    "labels",
];
const DETAIL_FIELDS: &str =
    "summary,description,status,issuetype,priority,assignee,reporter,updated,labels,parent";

/// Jira operations the app and the chat agent rely on.
///
/// Every method performs network I/O and returns [`JiraError`] on transport,
/// permission, or validation failures. Nothing is retried.
pub trait JiraOps {
    fn myself(&self) -> Result<JiraUser, JiraError>;
    /// Runs a JQL search and returns at most `limit` issues.
    fn search(&self, jql: &str, limit: usize) -> Result<Vec<JiraIssueSummary>, JiraError>;
    fn get_issue(&self, key: &IssueKey) -> Result<JiraIssueDetail, JiraError>;
    /// Creates an issue reported by the current user and returns its key.
    fn create_issue(&self, issue: &NewIssue) -> Result<IssueKey, JiraError>;
    /// Replaces the provided fields. An update with no fields is rejected.
    fn update_issue(&self, key: &IssueKey, update: &IssueUpdate) -> Result<(), JiraError>;
    fn add_comment(&self, key: &IssueKey, text: &str) -> Result<(), JiraError>;
    fn apply_transition(&self, key: &IssueKey, transition_id: &str) -> Result<(), JiraError>;
    /// Moves the issue to the status or transition named by `status`.
    fn transition_to_status(
        &self,
        key: &IssueKey,
        status: &str,
    ) -> Result<TransitionOutcome, JiraError>;
    fn assign(&self, key: &IssueKey, assignee: Assignee) -> Result<(), JiraError>;
    /// Lists the non-subtask issue type names available in a project.
    fn issue_types(&self, project_key: &str) -> Result<Vec<String>, JiraError>;
    fn browse_url(&self, key: &IssueKey) -> String;
}

pub struct JiraClient<T> {
    transport: T,
    site_url: String,
}

impl<T: JiraTransport> JiraClient<T> {
    /// `site_url` is the `https://host` origin used for browser links.
    pub fn new(transport: T, site_url: impl Into<String>) -> Self {
        Self {
            transport,
            site_url: site_url.into().trim_end_matches('/').to_string(),
        }
    }
}

impl<T: JiraTransport> JiraClient<T> {
    fn get(&self, path: &str) -> Result<Value, JiraError> {
        self.transport.send(Method::Get, path, None)
    }

    fn summary_from_issue(&self, issue: &Value) -> Result<JiraIssueSummary, JiraError> {
        let key = issue
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| JiraError::InvalidResponse("issue without a key".to_string()))
            .and_then(IssueKey::parse)?;
        let fields = &issue["fields"];
        let status = &fields["status"];

        Ok(JiraIssueSummary {
            url: self.browse_url(&key),
            key,
            summary: string_at(fields, "summary").unwrap_or_default(),
            status: string_at(status, "name").unwrap_or_else(|| "Unknown".to_string()),
            status_category: string_at(&status["statusCategory"], "key")
                .unwrap_or_else(|| "unknown".to_string()),
            issue_type: string_at(&fields["issuetype"], "name").unwrap_or_default(),
            priority: string_at(&fields["priority"], "name"),
            assignee: user_name(&fields["assignee"]),
            updated: string_at(fields, "updated"),
            labels: fields["labels"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        })
    }

    fn comments(&self, key: &IssueKey) -> Result<Vec<JiraComment>, JiraError> {
        let mut comments = Vec::new();
        loop {
            let page = self.get(&format!(
                "issue/{key}/comment?startAt={}&maxResults={COMMENT_PAGE_SIZE}",
                comments.len()
            ))?;
            let batch = page["comments"].as_array().cloned().unwrap_or_default();
            let total = page["total"].as_u64().unwrap_or(0) as usize;
            comments.extend(batch.iter().map(|comment| JiraComment {
                id: string_at(comment, "id").unwrap_or_default(),
                author: user_name(&comment["author"]).unwrap_or_else(|| "Unknown".to_string()),
                created: string_at(comment, "created").unwrap_or_default(),
                body: adf_to_text(&comment["body"]),
            }));
            if batch.is_empty() || comments.len() >= total || comments.len() >= MAX_COMMENTS {
                return Ok(comments);
            }
        }
    }

    fn transitions(&self, key: &IssueKey) -> Result<Vec<JiraTransition>, JiraError> {
        let response = self.get(&format!("issue/{key}/transitions"))?;
        Ok(response["transitions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|transition| JiraTransition {
                id: string_at(transition, "id").unwrap_or_default(),
                name: string_at(transition, "name").unwrap_or_default(),
                to_status: string_at(&transition["to"], "name").unwrap_or_default(),
            })
            .collect())
    }

    fn current_status(&self, key: &IssueKey) -> Result<String, JiraError> {
        let issue = self.get(&format!("issue/{key}?fields=status"))?;
        Ok(string_at(&issue["fields"]["status"], "name").unwrap_or_else(|| "Unknown".to_string()))
    }
}

fn string_at(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}

fn user_name(user: &Value) -> Option<String> {
    string_at(user, "displayName").or_else(|| string_at(user, "emailAddress"))
}

fn required_text(value: &str, message: &str) -> Result<String, JiraError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(JiraError::InvalidInput(message.to_string()));
    }
    Ok(trimmed.to_string())
}

impl<T: JiraTransport> JiraOps for JiraClient<T> {
    fn myself(&self) -> Result<JiraUser, JiraError> {
        let me = self.get("myself")?;
        let account_id = string_at(&me, "accountId").ok_or_else(|| {
            JiraError::InvalidResponse("Jira did not return the current account".to_string())
        })?;
        Ok(JiraUser {
            display_name: string_at(&me, "displayName").unwrap_or_default(),
            account_id,
        })
    }

    fn search(&self, jql: &str, limit: usize) -> Result<Vec<JiraIssueSummary>, JiraError> {
        let jql = required_text(jql, "a JQL query is required")?;
        let mut issues = Vec::new();
        let mut next_page_token: Option<String> = None;

        while issues.len() < limit {
            let page_size = (limit - issues.len()).min(SEARCH_PAGE_SIZE);
            let mut body = json!({
                "jql": jql,
                "maxResults": page_size,
                "fields": SEARCH_FIELDS,
            });
            if let Some(token) = &next_page_token {
                body["nextPageToken"] = json!(token);
            }

            let page = self
                .transport
                .send(Method::Post, "search/jql", Some(&body))?;
            let batch = page["issues"].as_array().cloned().unwrap_or_default();
            for issue in batch.iter().take(page_size) {
                issues.push(self.summary_from_issue(issue)?);
            }

            next_page_token = string_at(&page, "nextPageToken");
            if batch.is_empty() || next_page_token.is_none() {
                break;
            }
        }

        Ok(issues)
    }

    fn get_issue(&self, key: &IssueKey) -> Result<JiraIssueDetail, JiraError> {
        let issue = self.get(&format!("issue/{key}?fields={DETAIL_FIELDS}"))?;
        let fields = &issue["fields"];
        let description = &fields["description"];

        Ok(JiraIssueDetail {
            summary: self.summary_from_issue(&issue)?,
            description: adf_to_text(description),
            description_editable_losslessly: adf_round_trips_as_text(description),
            reporter: user_name(&fields["reporter"]),
            parent_key: string_at(&fields["parent"], "key"),
            comments: self.comments(key)?,
            transitions: self.transitions(key)?,
        })
    }

    fn create_issue(&self, issue: &NewIssue) -> Result<IssueKey, JiraError> {
        let project_key = normalize_project_key(&issue.project_key)?;
        let issue_type = required_text(&issue.issue_type, "an issue type is required")?;
        let summary = required_text(&issue.summary, "a summary is required")?;

        let me = self.myself()?;
        let mut fields = json!({
            "project": { "key": project_key },
            "issuetype": { "name": issue_type },
            "summary": summary,
            "reporter": { "accountId": me.account_id },
        });
        if !issue.description.trim().is_empty() {
            fields["description"] = text_to_adf(&issue.description);
        }

        let created =
            self.transport
                .send(Method::Post, "issue", Some(&json!({ "fields": fields })))?;
        string_at(&created, "key")
            .ok_or_else(|| {
                JiraError::InvalidResponse("Jira did not return the new issue key".to_string())
            })
            .and_then(|key| IssueKey::parse(&key))
    }

    fn update_issue(&self, key: &IssueKey, update: &IssueUpdate) -> Result<(), JiraError> {
        let mut fields = serde_json::Map::new();
        if let Some(summary) = &update.summary {
            fields.insert(
                "summary".to_string(),
                json!(required_text(summary, "a summary cannot be blank")?),
            );
        }
        if let Some(description) = &update.description {
            fields.insert("description".to_string(), text_to_adf(description));
        }
        if fields.is_empty() {
            return Err(JiraError::InvalidInput(
                "nothing to update: provide a summary or description".to_string(),
            ));
        }

        self.transport.send(
            Method::Put,
            &format!("issue/{key}"),
            Some(&json!({ "fields": fields })),
        )?;
        Ok(())
    }

    fn add_comment(&self, key: &IssueKey, text: &str) -> Result<(), JiraError> {
        let text = required_text(text, "comment text is required")?;
        self.transport.send(
            Method::Post,
            &format!("issue/{key}/comment"),
            Some(&json!({ "body": text_to_adf(&text) })),
        )?;
        Ok(())
    }

    fn apply_transition(&self, key: &IssueKey, transition_id: &str) -> Result<(), JiraError> {
        let transition_id = transition_id.trim();
        if transition_id.is_empty() || !transition_id.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(JiraError::InvalidInput(
                "a transition id is required".to_string(),
            ));
        }
        self.transport.send(
            Method::Post,
            &format!("issue/{key}/transitions"),
            Some(&json!({ "transition": { "id": transition_id } })),
        )?;
        Ok(())
    }

    fn transition_to_status(
        &self,
        key: &IssueKey,
        status: &str,
    ) -> Result<TransitionOutcome, JiraError> {
        let from = self.current_status(key)?;
        let transitions = self.transitions(key)?;
        let transition = match_transition(&transitions, status)
            .map_err(|error| JiraError::InvalidInput(format!("{key}: {error}")))?;

        if from.eq_ignore_ascii_case(&transition.to_status) {
            return Ok(TransitionOutcome {
                to: from.clone(),
                from,
                changed: false,
            });
        }

        self.apply_transition(key, &transition.id)?;
        Ok(TransitionOutcome {
            from,
            to: self.current_status(key)?,
            changed: true,
        })
    }

    fn assign(&self, key: &IssueKey, assignee: Assignee) -> Result<(), JiraError> {
        let account_id = match assignee {
            Assignee::Me => Value::String(self.myself()?.account_id),
            Assignee::Unassigned => Value::Null,
        };
        self.transport.send(
            Method::Put,
            &format!("issue/{key}/assignee"),
            Some(&json!({ "accountId": account_id })),
        )?;
        Ok(())
    }

    fn issue_types(&self, project_key: &str) -> Result<Vec<String>, JiraError> {
        let project_key = normalize_project_key(project_key)?;
        let response = self.get(&format!("issue/createmeta/{project_key}/issuetypes"))?;
        // Jira Cloud names the list `issueTypes`; older deployments use `values`.
        let types = response["issueTypes"]
            .as_array()
            .or_else(|| response["values"].as_array());
        Ok(types
            .into_iter()
            .flatten()
            .filter(|issue_type| !issue_type["subtask"].as_bool().unwrap_or(false))
            .filter_map(|issue_type| string_at(issue_type, "name"))
            .collect())
    }

    fn browse_url(&self, key: &IssueKey) -> String {
        format!("{}/browse/{key}", self.site_url)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{JiraClient, JiraOps};
    use crate::jira::adf::text_to_adf;
    use crate::jira::transport::testing::FakeTransport;
    use crate::jira::transport::Method;
    use crate::jira::types::{Assignee, IssueKey, IssueUpdate, NewIssue};
    use crate::jira::JiraError;

    const SITE: &str = "https://example.atlassian.net";

    fn key(value: &str) -> IssueKey {
        IssueKey::parse(value).unwrap()
    }

    fn issue_json(key: &str, summary: &str) -> Value {
        json!({
            "key": key,
            "fields": {
                "summary": summary,
                "status": { "name": "In Progress", "statusCategory": { "key": "indeterminate" } },
                "issuetype": { "name": "Bug" },
                "priority": { "name": "High" },
                "assignee": { "displayName": "Maya Lopez" },
                "updated": "2026-11-01T10:00:00.000+0000",
                "labels": ["printer"],
            }
        })
    }

    fn transitions_json() -> Value {
        json!({ "transitions": [
            { "id": "11", "name": "Start work", "to": { "name": "In Progress" } },
            { "id": "21", "name": "Dev Complete", "to": { "name": "Dev Complete" } },
        ]})
    }

    #[test]
    fn myself_maps_account() {
        let transport = FakeTransport::with_responses(vec![Ok(
            json!({ "accountId": "acc-1", "displayName": "Sam Dev" }),
        )]);
        let client = JiraClient::new(&transport, SITE);

        let me = client.myself().unwrap();

        assert_eq!(me.account_id, "acc-1");
        assert_eq!(me.display_name, "Sam Dev");
        assert_eq!(transport.requests()[0].path, "myself");
    }

    #[test]
    fn search_posts_jql_and_maps_issue_fields() {
        let transport = FakeTransport::with_responses(vec![Ok(json!({
            "issues": [issue_json("AB-7", "Printer jams"), {
                "key": "AB-8",
                "fields": { "summary": "Sparse issue" }
            }],
        }))]);
        let client = JiraClient::new(&transport, SITE);

        let issues = client.search("project = AB", 50).unwrap();

        let requests = transport.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, Method::Post);
        assert_eq!(requests[0].path, "search/jql");
        let body = requests[0].body.as_ref().unwrap();
        assert_eq!(body["jql"], "project = AB");
        assert_eq!(body["maxResults"], 50);
        assert!(body["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("status")));
        assert!(body.get("nextPageToken").is_none());

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].key.as_str(), "AB-7");
        assert_eq!(issues[0].summary, "Printer jams");
        assert_eq!(issues[0].status, "In Progress");
        assert_eq!(issues[0].status_category, "indeterminate");
        assert_eq!(issues[0].issue_type, "Bug");
        assert_eq!(issues[0].priority.as_deref(), Some("High"));
        assert_eq!(issues[0].assignee.as_deref(), Some("Maya Lopez"));
        assert_eq!(issues[0].labels, vec!["printer".to_string()]);
        assert_eq!(issues[0].url, "https://example.atlassian.net/browse/AB-7");
        assert_eq!(issues[1].status, "Unknown");
        assert_eq!(issues[1].status_category, "unknown");
        assert_eq!(issues[1].assignee, None);
        assert_eq!(issues[1].priority, None);
    }

    #[test]
    fn search_follows_page_tokens_until_the_limit() {
        let page = |keys: &[&str], token: Option<&str>| {
            let issues: Vec<Value> = keys.iter().map(|key| issue_json(key, "x")).collect();
            match token {
                Some(token) => json!({ "issues": issues, "nextPageToken": token }),
                None => json!({ "issues": issues }),
            }
        };
        let transport = FakeTransport::with_responses(vec![
            Ok(page(&["AB-1", "AB-2"], Some("t1"))),
            Ok(page(&["AB-3", "AB-4"], Some("t2"))),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let issues = client.search("project = AB", 3).unwrap();

        assert_eq!(issues.len(), 3);
        let requests = transport.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].body.as_ref().unwrap()["nextPageToken"], "t1");
        assert_eq!(requests[1].body.as_ref().unwrap()["maxResults"], 1);
    }

    #[test]
    fn search_rejects_blank_jql() {
        let transport = FakeTransport::default();
        let client = JiraClient::new(&transport, SITE);

        assert!(matches!(
            client.search("   ", 10),
            Err(JiraError::InvalidInput(_))
        ));
        assert!(transport.requests().is_empty());
    }

    #[test]
    fn get_issue_combines_fields_comments_and_transitions() {
        let mut issue = issue_json("AB-7", "Printer jams");
        issue["fields"]["description"] = text_to_adf("Jams after reboot.");
        issue["fields"]["reporter"] = json!({ "displayName": "Rina Q" });
        issue["fields"]["parent"] = json!({ "key": "AB-1" });
        let transport = FakeTransport::with_responses(vec![
            Ok(issue),
            Ok(json!({
                "total": 1,
                "comments": [{
                    "id": "900",
                    "author": { "displayName": "Maya Lopez" },
                    "created": "2026-11-02T09:00:00.000+0000",
                    "body": text_to_adf("Reproduced on bench 1."),
                }],
            })),
            Ok(transitions_json()),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let detail = client.get_issue(&key("ab-7")).unwrap();

        let requests = transport.requests();
        assert!(requests[0].path.starts_with("issue/AB-7?fields="));
        assert_eq!(
            requests[1].path,
            "issue/AB-7/comment?startAt=0&maxResults=100"
        );
        assert_eq!(requests[2].path, "issue/AB-7/transitions");
        assert_eq!(detail.summary.key.as_str(), "AB-7");
        assert_eq!(detail.description, "Jams after reboot.");
        assert!(detail.description_editable_losslessly);
        assert_eq!(detail.reporter.as_deref(), Some("Rina Q"));
        assert_eq!(detail.parent_key.as_deref(), Some("AB-1"));
        assert_eq!(detail.comments.len(), 1);
        assert_eq!(detail.comments[0].author, "Maya Lopez");
        assert_eq!(detail.comments[0].body, "Reproduced on bench 1.");
        assert_eq!(detail.transitions.len(), 2);
        assert_eq!(detail.transitions[0].id, "11");
        assert_eq!(detail.transitions[0].to_status, "In Progress");
    }

    #[test]
    fn get_issue_flags_descriptions_the_text_editor_would_flatten() {
        let mut issue = issue_json("AB-7", "Printer jams");
        issue["fields"]["description"] = json!({
            "type": "doc", "version": 1,
            "content": [{ "type": "table", "content": [] }],
        });
        let transport = FakeTransport::with_responses(vec![
            Ok(issue),
            Ok(json!({ "total": 0, "comments": [] })),
            Ok(json!({ "transitions": [] })),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let detail = client.get_issue(&key("AB-7")).unwrap();

        assert!(!detail.description_editable_losslessly);
    }

    #[test]
    fn create_issue_reports_as_current_user_and_returns_key() {
        let transport = FakeTransport::with_responses(vec![
            Ok(json!({ "accountId": "acc-1", "displayName": "Sam Dev" })),
            Ok(json!({ "id": "10001", "key": "AB-12" })),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let created = client
            .create_issue(&NewIssue {
                project_key: "ab".to_string(),
                issue_type: " Bug ".to_string(),
                summary: "  Printer jams on reboot ".to_string(),
                description: "Observed\n\n- jam".to_string(),
            })
            .unwrap();

        assert_eq!(created.as_str(), "AB-12");
        let requests = transport.requests();
        assert_eq!(requests[1].method, Method::Post);
        assert_eq!(requests[1].path, "issue");
        let fields = &requests[1].body.as_ref().unwrap()["fields"];
        assert_eq!(fields["project"]["key"], "AB");
        assert_eq!(fields["issuetype"]["name"], "Bug");
        assert_eq!(fields["summary"], "Printer jams on reboot");
        assert_eq!(fields["reporter"]["accountId"], "acc-1");
        assert_eq!(fields["description"], text_to_adf("Observed\n\n- jam"));
    }

    #[test]
    fn create_issue_omits_blank_description_and_requires_summary() {
        let transport = FakeTransport::with_responses(vec![
            Ok(json!({ "accountId": "acc-1", "displayName": "Sam Dev" })),
            Ok(json!({ "key": "AB-13" })),
        ]);
        let client = JiraClient::new(&transport, SITE);
        let mut issue = NewIssue {
            project_key: "AB".to_string(),
            issue_type: "Task".to_string(),
            summary: "Tidy logs".to_string(),
            description: "   ".to_string(),
        };

        client.create_issue(&issue).unwrap();

        let requests = transport.requests();
        let fields = &requests[1].body.as_ref().unwrap()["fields"];
        assert!(fields.get("description").is_none());

        issue.summary = "  ".to_string();
        assert!(matches!(
            client.create_issue(&issue),
            Err(JiraError::InvalidInput(_))
        ));
        assert_eq!(transport.requests().len(), 2);
    }

    #[test]
    fn update_issue_sends_only_provided_fields() {
        let transport = FakeTransport::with_responses(vec![Ok(Value::Null)]);
        let client = JiraClient::new(&transport, SITE);

        client
            .update_issue(
                &key("AB-7"),
                &IssueUpdate {
                    summary: Some(" New title ".to_string()),
                    description: None,
                },
            )
            .unwrap();

        let requests = transport.requests();
        assert_eq!(requests[0].method, Method::Put);
        assert_eq!(requests[0].path, "issue/AB-7");
        assert_eq!(
            requests[0].body,
            Some(json!({ "fields": { "summary": "New title" } }))
        );
    }

    #[test]
    fn update_issue_converts_description_and_rejects_empty_updates() {
        let transport = FakeTransport::with_responses(vec![Ok(Value::Null)]);
        let client = JiraClient::new(&transport, SITE);

        client
            .update_issue(
                &key("AB-7"),
                &IssueUpdate {
                    summary: None,
                    description: Some("Now fixed".to_string()),
                },
            )
            .unwrap();

        assert_eq!(
            transport.requests()[0].body,
            Some(json!({ "fields": { "description": text_to_adf("Now fixed") } }))
        );
        assert!(matches!(
            client.update_issue(&key("AB-7"), &IssueUpdate::default()),
            Err(JiraError::InvalidInput(_))
        ));
        assert!(matches!(
            client.update_issue(
                &key("AB-7"),
                &IssueUpdate {
                    summary: Some("  ".to_string()),
                    description: None
                }
            ),
            Err(JiraError::InvalidInput(_))
        ));
        assert_eq!(transport.requests().len(), 1);
    }

    #[test]
    fn add_comment_posts_document_body() {
        let transport = FakeTransport::with_responses(vec![Ok(json!({ "id": "901" }))]);
        let client = JiraClient::new(&transport, SITE);

        client
            .add_comment(&key("AB-7"), " Verified on bench ")
            .unwrap();

        let requests = transport.requests();
        assert_eq!(requests[0].method, Method::Post);
        assert_eq!(requests[0].path, "issue/AB-7/comment");
        assert_eq!(
            requests[0].body,
            Some(json!({ "body": text_to_adf("Verified on bench") }))
        );
        assert!(matches!(
            client.add_comment(&key("AB-7"), "  "),
            Err(JiraError::InvalidInput(_))
        ));
    }

    #[test]
    fn apply_transition_posts_transition_id() {
        let transport = FakeTransport::with_responses(vec![Ok(Value::Null)]);
        let client = JiraClient::new(&transport, SITE);

        client.apply_transition(&key("AB-7"), "21").unwrap();

        let requests = transport.requests();
        assert_eq!(requests[0].path, "issue/AB-7/transitions");
        assert_eq!(
            requests[0].body,
            Some(json!({ "transition": { "id": "21" } }))
        );
        assert!(matches!(
            client.apply_transition(&key("AB-7"), "abc"),
            Err(JiraError::InvalidInput(_))
        ));
    }

    #[test]
    fn transition_to_status_resolves_name_and_reports_new_status() {
        let status = |name: &str| json!({ "fields": { "status": { "name": name } } });
        let transport = FakeTransport::with_responses(vec![
            Ok(status("In Progress")),
            Ok(transitions_json()),
            Ok(Value::Null),
            Ok(status("Dev Complete")),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let outcome = client
            .transition_to_status(&key("AB-7"), "dev complete")
            .unwrap();

        assert_eq!(outcome.from, "In Progress");
        assert_eq!(outcome.to, "Dev Complete");
        assert!(outcome.changed);
        let requests = transport.requests();
        assert_eq!(requests[0].path, "issue/AB-7?fields=status");
        assert_eq!(
            requests[2].body,
            Some(json!({ "transition": { "id": "21" } }))
        );
    }

    #[test]
    fn transition_to_current_status_changes_nothing() {
        let transport = FakeTransport::with_responses(vec![
            Ok(json!({ "fields": { "status": { "name": "In Progress" } } })),
            Ok(transitions_json()),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let outcome = client
            .transition_to_status(&key("AB-7"), "In Progress")
            .unwrap();

        assert!(!outcome.changed);
        assert_eq!(outcome.to, "In Progress");
        assert_eq!(transport.requests().len(), 2);
    }

    #[test]
    fn transition_to_unknown_status_lists_available_transitions() {
        let transport = FakeTransport::with_responses(vec![
            Ok(json!({ "fields": { "status": { "name": "To Do" } } })),
            Ok(transitions_json()),
        ]);
        let client = JiraClient::new(&transport, SITE);

        let error = client
            .transition_to_status(&key("AB-7"), "Closed")
            .unwrap_err();

        let JiraError::InvalidInput(message) = error else {
            panic!("expected invalid input");
        };
        assert!(message.contains("Start work -> In Progress"), "{message}");
    }

    #[test]
    fn assign_to_me_uses_current_account_and_unassign_sends_null() {
        let transport = FakeTransport::with_responses(vec![
            Ok(json!({ "accountId": "acc-1", "displayName": "Sam Dev" })),
            Ok(Value::Null),
            Ok(Value::Null),
        ]);
        let client = JiraClient::new(&transport, SITE);

        client.assign(&key("AB-7"), Assignee::Me).unwrap();
        client.assign(&key("AB-7"), Assignee::Unassigned).unwrap();

        let requests = transport.requests();
        assert_eq!(requests[1].method, Method::Put);
        assert_eq!(requests[1].path, "issue/AB-7/assignee");
        assert_eq!(requests[1].body, Some(json!({ "accountId": "acc-1" })));
        assert_eq!(requests[2].body, Some(json!({ "accountId": null })));
    }

    #[test]
    fn issue_types_skip_subtasks() {
        let transport = FakeTransport::with_responses(vec![Ok(json!({
            "issueTypes": [
                { "name": "Bug", "subtask": false },
                { "name": "Sub-task", "subtask": true },
                { "name": "Story", "subtask": false },
            ],
        }))]);
        let client = JiraClient::new(&transport, SITE);

        let types = client.issue_types("ab").unwrap();

        assert_eq!(types, vec!["Bug".to_string(), "Story".to_string()]);
        assert_eq!(
            transport.requests()[0].path,
            "issue/createmeta/AB/issuetypes"
        );
    }

    #[test]
    fn browse_url_uses_site_origin() {
        let client = JiraClient::new(FakeTransport::default(), "https://example.atlassian.net/");

        assert_eq!(
            client.browse_url(&key("AB-7")),
            "https://example.atlassian.net/browse/AB-7"
        );
    }
}
