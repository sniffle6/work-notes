import { describe, expect, it } from "vitest";

import { filterIssues, groupIssuesByStatus, priorityTone } from "./jira-list";
import type { JiraIssueSummary } from "./types";

function issue(overrides: Partial<JiraIssueSummary> = {}): JiraIssueSummary {
  return {
    key: "AB-7",
    summary: "Printer jams on reboot",
    status: "To Do",
    statusCategory: "new",
    issueType: "Bug",
    priority: "High",
    assignee: null,
    updated: null,
    labels: [],
    url: "https://example.atlassian.net/browse/AB-7",
    ...overrides,
  };
}

describe("filterIssues", () => {
  const issues = [
    issue(),
    issue({ key: "AB-8", summary: "Bill acceptor rejects notes", assignee: "Maya Lopez", labels: ["hardware"] }),
    issue({ key: "AB-12", summary: "Meter export is empty", issueType: "Task", status: "In QA" }),
  ];

  it("returns every ticket for a blank search", () => {
    expect(filterIssues(issues, "   ")).toEqual(issues);
  });

  it("matches key, summary, assignee, type, status, and labels without regard to case", () => {
    expect(filterIssues(issues, "ab-12").map((item) => item.key)).toEqual(["AB-12"]);
    expect(filterIssues(issues, "PRINTER").map((item) => item.key)).toEqual(["AB-7"]);
    expect(filterIssues(issues, "maya").map((item) => item.key)).toEqual(["AB-8"]);
    expect(filterIssues(issues, "task").map((item) => item.key)).toEqual(["AB-12"]);
    expect(filterIssues(issues, "in qa").map((item) => item.key)).toEqual(["AB-12"]);
    expect(filterIssues(issues, "hardware").map((item) => item.key)).toEqual(["AB-8"]);
  });

  it("requires every search word to match", () => {
    expect(filterIssues(issues, "bug notes").map((item) => item.key)).toEqual(["AB-8"]);
    expect(filterIssues(issues, "bug export")).toEqual([]);
  });

  it("finds unassigned tickets by that word", () => {
    expect(filterIssues(issues, "unassigned").map((item) => item.key)).toEqual(["AB-7", "AB-12"]);
  });
});

describe("groupIssuesByStatus", () => {
  it("groups in workflow order and keeps the list order inside each group", () => {
    const groups = groupIssuesByStatus([
      issue({ key: "AB-1", status: "Done", statusCategory: "done" }),
      issue({ key: "AB-2", status: "In QA", statusCategory: "indeterminate" }),
      issue({ key: "AB-3", status: "To Do", statusCategory: "new" }),
      issue({ key: "AB-4", status: "Blocked", statusCategory: "indeterminate" }),
      issue({ key: "AB-5", status: "To Do", statusCategory: "new" }),
      issue({ key: "AB-6", status: "In QA", statusCategory: "indeterminate" }),
    ]);

    expect(groups.map((group) => [group.status, group.issues.map((item) => item.key)])).toEqual([
      ["To Do", ["AB-3", "AB-5"]],
      ["In QA", ["AB-2", "AB-6"]],
      ["Blocked", ["AB-4"]],
      ["Done", ["AB-1"]],
    ]);
    expect(groups[1].statusCategory).toBe("indeterminate");
  });

  it("returns no groups for no tickets", () => {
    expect(groupIssuesByStatus([])).toEqual([]);
  });
});

describe("priorityTone", () => {
  it("flags the priorities that should stand out", () => {
    expect(priorityTone("Highest")).toBe("urgent");
    expect(priorityTone("Critical")).toBe("urgent");
    expect(priorityTone("Blocker")).toBe("urgent");
    expect(priorityTone("High")).toBe("high");
    expect(priorityTone("Medium")).toBe("normal");
    expect(priorityTone("Low")).toBe("normal");
    expect(priorityTone(null)).toBe("normal");
  });
});
