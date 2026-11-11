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
        "Read the ticket below and investigate the relevant code before changing anything. Then do the work the ticket asks for and verify it. Ask me if the ticket leaves something unclear.",
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
      issue({ priority: null, assignee: null, reporter: null, parentKey: null, labels: [], comments: [] }),
    );

    expect(prompt).toContain("Assignee: Unassigned");
    expect(prompt).not.toContain("Priority:");
    expect(prompt).not.toContain("Reporter:");
    expect(prompt).not.toContain("Parent:");
    expect(prompt).not.toContain("Labels:");
    expect(prompt).not.toContain("## Comments");
    expect(prompt.endsWith("Jams after reboot.")).toBe(true);
  });

  it("says so when the ticket has no description", () => {
    const prompt = buildTicketPrompt(issue({ description: "  \n", comments: [] }));

    expect(prompt.endsWith("## Description\n\nThe ticket has no description.")).toBe(true);
  });
});
