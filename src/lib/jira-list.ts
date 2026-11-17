import type { JiraFilter, JiraIssueSummary, JiraStatusCategory } from "./types";

export type JiraIssueGroup = {
  status: string;
  statusCategory: JiraStatusCategory;
  issues: JiraIssueSummary[];
};

export type PriorityTone = "urgent" | "high" | "normal";

const UNASSIGNED = "Unassigned";
const CATEGORY_ORDER: JiraStatusCategory[] = ["new", "indeterminate", "unknown", "done"];
const URGENT_PRIORITIES = new Set(["highest", "critical", "blocker"]);

/**
 * Keeps the tickets that contain every search word in their key, summary,
 * assignee, type, status, priority, or labels. A blank search keeps them all.
 */
export function filterIssues(issues: JiraIssueSummary[], query: string): JiraIssueSummary[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) {
    return issues;
  }

  return issues.filter((issue) => {
    const haystack = [
      issue.key,
      issue.summary,
      issue.assignee ?? UNASSIGNED,
      issue.issueType,
      issue.status,
      issue.priority ?? "",
      ...issue.labels,
    ]
      .join("\n")
      .toLowerCase();
    return words.every((word) => haystack.includes(word));
  });
}

/**
 * Groups tickets by status in workflow order (to do, in progress, done).
 * Statuses in the same stage and tickets in the same status keep list order.
 */
export function groupIssuesByStatus(issues: JiraIssueSummary[]): JiraIssueGroup[] {
  const groups = new Map<string, JiraIssueGroup>();
  for (const issue of issues) {
    const group = groups.get(issue.status);
    if (group) {
      group.issues.push(issue);
    } else {
      groups.set(issue.status, { status: issue.status, statusCategory: issue.statusCategory, issues: [issue] });
    }
  }

  return [...groups.values()].sort(
    (left, right) => CATEGORY_ORDER.indexOf(left.statusCategory) - CATEGORY_ORDER.indexOf(right.statusCategory),
  );
}

// Characters Jira's text search reads as operators, plus the quote and
// backslash that would end or escape the JQL string.
// https://support.atlassian.com/jira-software-cloud/docs/search-for-special-characters-and-text/
const TEXT_SEARCH_RESERVED = /[+\-&|!(){}[\]^~*?\\:"/]/g;
const ISSUE_KEY = /^[A-Za-z][A-Za-z0-9_]*-\d+$/;

/**
 * Builds a Jira search for tickets whose text contains the query's words,
 * newest first, within the project when one is set. Returns null when no
 * searchable words remain.
 */
export function jiraTextSearch(query: string, projectKey: string): JiraFilter | null {
  const words = query.replace(TEXT_SEARCH_RESERVED, " ").split(/\s+/).filter(Boolean).join(" ");
  if (!words) {
    return null;
  }
  const scope = projectKey ? `project = ${projectKey} AND ` : "";
  return {
    name: `Search: ${query.trim().replace(/\s+/g, " ")}`,
    jql: `${scope}text ~ "${words}" ORDER BY updated DESC`,
  };
}

/** The ticket key the query consists of, upper-cased, or null. */
export function issueKeyInQuery(query: string): string | null {
  const trimmed = query.trim();
  return ISSUE_KEY.test(trimmed) ? trimmed.toUpperCase() : null;
}

/** How strongly a Jira priority should stand out in the ticket list. */
export function priorityTone(priority: string | null): PriorityTone {
  const name = priority?.toLowerCase() ?? "";
  if (URGENT_PRIORITIES.has(name)) {
    return "urgent";
  }
  return name === "high" ? "high" : "normal";
}
