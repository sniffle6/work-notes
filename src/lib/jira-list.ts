import type { JiraIssueSummary, JiraStatusCategory } from "./types";

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

/** How strongly a Jira priority should stand out in the ticket list. */
export function priorityTone(priority: string | null): PriorityTone {
  const name = priority?.toLowerCase() ?? "";
  if (URGENT_PRIORITIES.has(name)) {
    return "urgent";
  }
  return name === "high" ? "high" : "normal";
}
