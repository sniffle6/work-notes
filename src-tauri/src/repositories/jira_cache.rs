//! Saved copies of Jira ticket lists and tickets. The app shows them while it
//! asks Jira for current ones. Each copy belongs to a scope naming the Jira
//! site and account, so switching either never shows another's tickets.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::db::Database;
use crate::jira::IssueKey;

use super::{now_db_string, parse_db_datetime, RepositoryResult};

/// Saved ticket lists kept per scope; older lists are dropped first.
const MAX_SAVED_LISTS: i64 = 50;
/// Saved tickets kept per scope; older tickets are dropped first.
const MAX_SAVED_ISSUES: i64 = 500;

/// A saved JSON copy and when it was fetched from Jira.
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

    /// Replaces the saved result of `jql` in `scope`.
    pub fn save_list(&self, scope: &str, jql: &str, content: &str) -> RepositoryResult<()> {
        self.save(SavedTable::Lists, scope, jql, content)
    }

    pub fn load_list(&self, scope: &str, jql: &str) -> RepositoryResult<Option<SavedJiraCopy>> {
        self.load(SavedTable::Lists, scope, jql)
    }

    /// Replaces the saved copy of one ticket in `scope`.
    pub fn save_issue(&self, scope: &str, key: &IssueKey, content: &str) -> RepositoryResult<()> {
        self.save(SavedTable::Issues, scope, key.as_str(), content)
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
    ) -> RepositoryResult<()> {
        let (name, limit) = table.name_and_limit();
        let mut connection = self.db.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            &format!(
                "INSERT INTO {name} (scope, id, content, saved_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope, id) DO UPDATE
                 SET content = excluded.content, saved_at = excluded.saved_at"
            ),
            params![scope, id, content, now_db_string()],
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

        cache.save_list(SCOPE, "project = AB", "[1]").unwrap();
        let first = cache.load_list(SCOPE, "project = AB").unwrap().unwrap();
        cache.save_list(SCOPE, "project = AB", "[2]").unwrap();
        let second = cache.load_list(SCOPE, "project = AB").unwrap().unwrap();

        assert_eq!(first.content, "[1]");
        assert_eq!(second.content, "[2]");
        assert!(second.saved_at >= first.saved_at);
        assert_eq!(cache.load_list(SCOPE, "project = CD").unwrap(), None);
    }

    #[test]
    fn a_saved_ticket_is_returned_by_key() {
        let cache = repository();

        cache.save_issue(SCOPE, &key("AB-7"), "{\"v\":1}").unwrap();
        cache.save_issue(SCOPE, &key("AB-7"), "{\"v\":2}").unwrap();

        assert_eq!(
            cache.load_issue(SCOPE, &key("ab-7")).unwrap().unwrap().content,
            "{\"v\":2}"
        );
        assert_eq!(cache.load_issue(SCOPE, &key("AB-8")).unwrap(), None);
    }

    #[test]
    fn copies_from_another_site_or_account_are_never_returned() {
        let cache = repository();

        cache.save_list(OTHER_SCOPE, "project = AB", "[1]").unwrap();
        cache.save_issue(OTHER_SCOPE, &key("AB-7"), "{}").unwrap();

        assert_eq!(cache.load_list(SCOPE, "project = AB").unwrap(), None);
        assert_eq!(cache.load_issue(SCOPE, &key("AB-7")).unwrap(), None);
    }

    #[test]
    fn the_oldest_copies_are_dropped_beyond_the_limits() {
        let cache = repository();

        for index in 0..=MAX_SAVED_LISTS {
            cache.save_list(SCOPE, &format!("id = {index}"), "[]").unwrap();
        }
        for index in 1..=MAX_SAVED_ISSUES + 1 {
            cache.save_issue(SCOPE, &key(&format!("AB-{index}")), "{}").unwrap();
        }
        // Each scope keeps its own copies.
        cache.save_list(OTHER_SCOPE, "id = 0", "[]").unwrap();

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
}
