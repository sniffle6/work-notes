import type { JiraComment, JiraIssueDetail } from "./types";

const INSTRUCTIONS =
  "Read the ticket below and investigate the relevant code before changing anything. " +
  "Then do the work the ticket asks for and verify it. Ask me if the ticket leaves something unclear.";
const ISO_DATE = /^\d{4}-\d{2}-\d{2}/;

/**
 * Builds a prompt that hands a ticket to a coding agent. It carries the
 * ticket's facts, description, and comments so the agent needs no Jira access.
 */
export function buildTicketPrompt(issue: JiraIssueDetail): string {
  const facts = [
    `Type: ${issue.issueType}`,
    issue.priority ? `Priority: ${issue.priority}` : null,
    `Status: ${issue.status}`,
    `Assignee: ${issue.assignee ?? "Unassigned"}`,
    issue.reporter ? `Reporter: ${issue.reporter}` : null,
    issue.parentKey ? `Parent: ${issue.parentKey}` : null,
    issue.labels.length > 0 ? `Labels: ${issue.labels.join(", ")}` : null,
    `Link: ${issue.url}`,
  ].filter((fact): fact is string => fact !== null);

  const sections = [
    `Work on Jira ticket ${issue.key}: ${issue.summary}`,
    INSTRUCTIONS,
    facts.join("\n"),
    "## Description",
    issue.description.trim() || "The ticket has no description.",
  ];
  if (issue.comments.length > 0) {
    sections.push("## Comments", ...issue.comments.map(commentBlock));
  }

  return sections.join("\n\n");
}

function commentBlock(comment: JiraComment): string {
  const date = ISO_DATE.exec(comment.created)?.[0];
  const heading = date ? `${comment.author} (${date}):` : `${comment.author}:`;
  return `${heading}\n${comment.body.trim()}`;
}
