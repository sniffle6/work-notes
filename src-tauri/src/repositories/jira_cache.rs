//! Saved copies of Jira ticket lists and tickets. The app shows them while it
//! asks Jira for current ones. Each copy belongs to a scope naming the Jira
//! site and account, so switching either never shows another's tickets.

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension};

use crate::db::Database;
use crate::jira::IssueKey;

use super::{parse_db_datetime, RepositoryResult};

/// Saved ticket lists kept per scope; older lists are dropped first.
const MAX_SAVED_LISTS: i64 = 50;
/// Saved tickets kept per scope; older tickets are dropped first.
const MAX_SAVED_ISSUES: i64 = 500;

/// A saved JSON copy and when Jira was asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedJiraCopy {
    pub content: String,
    pub saved_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct JiraIssueCacheRepository {
    db: Database,
}

impl JiraIssueCacheRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Saves the result of `jql` in `scope`, asked of Jira at `fetched_at`.
    /// A copy fetched earlier than the saved one is ignored.
    pub fn save_list(
        &self,
        scope: &str,
        jql: &str,
        content: &str,
        fetched_at: DateTime<Utc>,
    ) -> RepositoryResult<()> {
        self.save(SavedTable::Lists, scope, jql, content, fetched_at)
    }

    pub fn load_list(&self, scope: &str, jql: &str) -> RepositoryResult<Option<SavedJiraCopy>> {
        self.load(SavedTable::Lists, scope, jql)
    }

    /// Saves one ticket in `scope`, asked of Jira at `fetched_at`. A copy
    /// fetched earlier than the saved one is ignored.
    pub fn save_issue(
        &self,
        scope: &str,
        key: &IssueKey,
        content: &str,
        fetched_at: DateTime<Utc>,
    ) -> RepositoryResult<()> {
        self.save(SavedTable::Issues, scope, key.as_str(), content, fetched_at)
    }

    /// Removes the saved copy of a ticket Jira no longer returns.
    pub fn forget_issue(&self, scope: &str, key: &IssueKey) -> RepositoryResult<()> {
        self.db.connection()?.execute(
            "DELETE FROM jira_saved_issues WHERE scope = ?1 AND id = ?2",
            params![scope, key.as_str()],
        )?;
        Ok(())
    }

    pub fn load_issue(
        &self,
        scope: &str,
        key: &IssueKey,
    ) -> RepositoryResult<Option<SavedJiraCopy>> {
        self.load(SavedTable::Issues, scope, key.as_str())
    }

    fn save(
        &self,
        table: SavedTable,
        scope: &str,
        id: &str,
        content: &str,
        fetched_at: DateTime<Utc>,
    ) -> RepositoryResult<()> {
        let (name, limit) = table.name_and_limit();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            &format!(
                "INSERT INTO {name} (scope, id, content, saved_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope, id) DO UPDATE
                 SET content = excluded.content, saved_at = excluded.saved_at
                 WHERE excluded.saved_at >= {name}.saved_at"
            ),
            params![scope, id, content, db_time(fetched_at)],
        )?;
        transaction.execute(
            &format!(
                "DELETE FROM {name} WHERE scope = ?1 AND rowid NOT IN (
                   SELECT rowid FROM {name} WHERE scope = ?1
                   ORDER BY saved_at DESC, rowid DESC LIMIT ?2
                 )"
            ),
            params![scope, limit],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn load(
        &self,
        table: SavedTable,
        scope: &str,
        id: &str,
    ) -> RepositoryResult<Option<SavedJiraCopy>> {
        let (name, _) = table.name_and_limit();
        let saved = self
            .db
            .connection()?
            .query_row(
                &format!("SELECT content, saved_at FROM {name} WHERE scope = ?1 AND id = ?2"),
                params![scope, id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        saved
            .map(|(content, saved_at)| {
                Ok(SavedJiraCopy {
                    content,
                    saved_at: parse_db_datetime("saved_at", saved_at)?,
                })
            })
            .transpose()
    }
}

// Fixed-width UTC timestamps, so SQLite compares them in time order.
fn db_time(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Clone, Copy)]
enum SavedTable {
    Lists,
    Issues,
}

impl SavedTable {
    fn name_and_limit(self) -> (&'static str, i64) {
        match self {
            Self::Lists => ("jira_saved_lists", MAX_SAVED_LISTS),
            Self::Issues => ("jira_saved_issues", MAX_SAVED_ISSUES),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{JiraIssueCacheRepository, MAX_SAVED_ISSUES, MAX_SAVED_LISTS};
    use crate::db::Database;
    use crate::jira::IssueKey;

    const SCOPE: &str = "[\"https://a.atlassian.net\",\"dev@example.test\"]";
    const OTHER_SCOPE: &str = "[\"https://b.atlassian.net\",\"dev@example.test\"]";

    fn repository() -> JiraIssueCacheRepository {
        JiraIssueCacheRepository::new(Database::in_memory().unwrap())
    }

    fn key(value: &str) -> IssueKey {
        IssueKey::parse(value).unwrap()
    }

    #[test]
    fn a_saved_list_is_returned_with_its_time_and_replaced_by_the_next_save() {
        let cache = repository();
        assert_eq!(cache.load_list(SCOPE, "project = AB").unwrap(), None);

        cache.save_list(SCOPE, "project = AB", "[1]", Utc::now()).unwrap();
        let first = cache.load_list(SCOPE, "project = AB").unwrap().unwrap();
        cache.save_list(SCOPE, "project = AB", "[2]", Utc::now()).unwrap();
        let second = cache.load_list(SCOPE, "project = AB").unwrap().unwrap();

        assert_eq!(first.content, "[1]");
        assert_eq!(second.content, "[2]");
        assert!(second.saved_at >= first.saved_at);
        assert_eq!(cache.load_list(SCOPE, "project = CD").unwrap(), None);
    }

    #[test]
    fn a_saved_ticket_is_returned_by_key() {
        let cache = repository();

        cache.save_issue(SCOPE, &key("AB-7"), "{\"v\":1}", Utc::now()).unwrap();
        cache.save_issue(SCOPE, &key("AB-7"), "{\"v\":2}", Utc::now()).unwrap();

        assert_eq!(
            cache.load_issue(SCOPE, &key("ab-7")).unwrap().unwrap().content,
            "{\"v\":2}"
        );
        assert_eq!(cache.load_issue(SCOPE, &key("AB-8")).unwrap(), None);
    }

    #[test]
    fn copies_from_another_site_or_account_are_never_returned() {
        let cache = repository();

        cache.save_list(OTHER_SCOPE, "project = AB", "[1]", Utc::now()).unwrap();
        cache.save_issue(OTHER_SCOPE, &key("AB-7"), "{}", Utc::now()).unwrap();

        assert_eq!(cache.load_list(SCOPE, "project = AB").unwrap(), None);
        assert_eq!(cache.load_issue(SCOPE, &key("AB-7")).unwrap(), None);
    }

    #[test]
    fn the_oldest_copies_are_dropped_beyond_the_limits() {
        let cache = repository();

        for index in 0..=MAX_SAVED_LISTS {
            cache.save_list(SCOPE, &format!("id = {index}"), "[]", Utc::now()).unwrap();
        }
        for index in 1..=MAX_SAVED_ISSUES + 1 {
            cache.save_issue(SCOPE, &key(&format!("AB-{index}")), "{}", Utc::now()).unwrap();
        }
        // Each scope keeps its own copies.
        cache.save_list(OTHER_SCOPE, "id = 0", "[]", Utc::now()).unwrap();

        assert_eq!(cache.load_list(SCOPE, "id = 0").unwrap(), None);
        assert!(cache.load_list(SCOPE, "id = 1").unwrap().is_some());
        assert!(cache
            .load_list(SCOPE, &format!("id = {MAX_SAVED_LISTS}"))
            .unwrap()
            .is_some());
        assert_eq!(cache.load_issue(SCOPE, &key("AB-1")).unwrap(), None);
        assert!(cache.load_issue(SCOPE, &key("AB-2")).unwrap().is_some());
        assert!(cache.load_list(OTHER_SCOPE, "id = 0").unwrap().is_some());
    }

    #[test]
    fn an_answer_fetched_earlier_never_replaces_a_newer_saved_copy() {
        let cache = repository();
        let newer = Utc::now();
        let older = newer - Duration::seconds(5);

        cache.save_issue(SCOPE, &key("AB-7"), "\"new\"", newer).unwrap();
        cache.save_issue(SCOPE, &key("AB-7"), "\"old\"", older).unwrap();
        cache.save_list(SCOPE, "project = AB", "\"new\"", newer).unwrap();
        cache.save_list(SCOPE, "project = AB", "\"old\"", older).unwrap();

        assert_eq!(cache.load_issue(SCOPE, &key("AB-7")).unwrap().unwrap().content, "\"new\"");
        assert_eq!(cache.load_list(SCOPE, "project = AB").unwrap().unwrap().content, "\"new\"");
    }

    #[test]
    fn a_forgotten_ticket_is_no_longer_returned() {
        let cache = repository();
        cache.save_issue(SCOPE, &key("AB-7"), "{}", Utc::now()).unwrap();
        cache.save_issue(OTHER_SCOPE, &key("AB-7"), "{}", Utc::now()).unwrap();

        cache.forget_issue(SCOPE, &key("AB-7")).unwrap();
        cache.forget_issue(SCOPE, &key("AB-9")).unwrap();

        assert_eq!(cache.load_issue(SCOPE, &key("AB-7")).unwrap(), None);
        assert!(cache.load_issue(OTHER_SCOPE, &key("AB-7")).unwrap().is_some());
    }
}