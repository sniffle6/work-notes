import { describe, expect, it } from "vitest";

import { buildTicketPrompt } from "./jira-prompt";
import type { JiraIssueDetail } from "./types";

function issue(overrides: Partial<JiraIssueDetail> = {}): JiraIssueDetail {
  return {
    key: "AB-7",
    summary: "Printer jams on reboot",
    status: "To Do",
    statusCategory: "new",
    issueType: "Bug",
    priority: "High",
    assignee: "Maya Lopez",
    updated: "2026-11-01T10:00:00.000Z",
    labels: ["printer", "bench"],
    url: "https://example.atlassian.net/browse/AB-7",
    description: "## Context\n\nJams after reboot.",
    descriptionEditableLosslessly: true,
    reporter: "Rina Q",
    parentKey: "AB-1",
    subtasks: [{ key: "AB-8", summary: "Replace the sensor", status: "To Do" }],
    linkedIssues: [
      { relationship: "blocks", key: "AB-9", summary: "Ship firmware", status: "In Progress" },
      { relationship: "is blocked by", key: "AB-3", summary: "Order parts", status: "Done" },
    ],
    comments: [
      { id: "c1", author: "Maya Lopez", created: "2026-11-02T09:00:00.000Z", body: "Reproduced on bench 1." },
      { id: "c2", author: "Rina Q", created: "not a date", body: "Also on bench 2." },
    ],
    transitions: [],
    ...overrides,
  };
}

describe("buildTicketPrompt", () => {
  it("builds a self-contained prompt with the ticket facts, description, and comments", () => {
    expect(buildTicketPrompt(issue())).toBe(
      [
        "Work on Jira ticket AB-7: Printer jams on reboot",
        "",
        "Read the ticket below and investigate the relevant code before changing anything. Then tell me what you understand the ticket to ask for and how you plan to do it.",
        "",
        "Before you implement, grill me on anything you are not sure about: requirements, edge cases, scope, and how the result will be verified. Ask one question at a time and do not guess. Start the work only once we both agree on what needs doing, then implement and verify it.",
        "",
        "Type: Bug",
        "Priority: High",
        "Status: To Do",
        "Assignee: Maya Lopez",
        "Reporter: Rina Q",
        "Parent: AB-1",
        "Labels: printer, bench",
        "Link: https://example.atlassian.net/browse/AB-7",
        "",
        "## Description",
        "",
        "## Context",
        "",
        "Jams after reboot.",
        "",
        "## Subtasks",
        "",
        "- AB-8 (To Do): Replace the sensor",
        "",
        "## Linked issues",
        "",
        "- blocks AB-9 (In Progress): Ship firmware",
        "- is blocked by AB-3 (Done): Order parts",
        "",
        "## Comments",
        "",
        "Maya Lopez (2026-11-02):",
        "Reproduced on bench 1.",
        "",
        "Rina Q:",
        "Also on bench 2.",
      ].join("\n"),
    );
  });

  it("leaves out facts the ticket does not have", () => {
    const prompt = buildTicketPrompt(
      issue({
        priority: null,
        assignee: null,
        reporter: null,
        parentKey: null,
        labels: [],
        subtasks: [],
        linkedIssues: [],
        comments: [],
      }),
    );

    expect(prompt).not.toContain("## Subtasks");
    expect(prompt).not.toContain("## Linked issues");
    expect(prompt).toContain("Assignee: Unassigned");
    expect(prompt).not.toContain("Priority:");
    expect(prompt).not.toContain("Reporter:");
    expect(prompt).not.toContain("Parent:");
    expect(prompt).not.toContain("Labels:");
    expect(prompt).not.toContain("## Comments");
    expect(prompt.endsWith("Jams after reboot.")).toBe(true);
  });

  it("says so when the ticket has no description", () => {
    const prompt = buildTicketPrompt(issue({ description: "  \n", subtasks: [], linkedIssues: [], comments: [] }));

    expect(prompt.endsWith("## Description\n\nThe ticket has no description.")).toBe(true);
  });
});
