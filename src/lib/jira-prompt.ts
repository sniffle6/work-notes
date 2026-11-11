import { isoDate, workInstructions } from "./agent-prompt";
import type { JiraComment, JiraIssueDetail } from "./types";

/**
 * Builds a prompt that hands a ticket to a coding agent. It carries the
 * ticket's facts, description, subtasks, linked issues, and comments so the
 * agent needs no Jira access.
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
    workInstructions("ticket"),
    facts.join("\n"),
    "## Description",
    issue.description.trim() || "The ticket has no description.",
  ];
  if (issue.subtasks.length > 0) {
    sections.push(
      "## Subtasks",
      issue.subtasks.map((subtask) => `- ${subtask.key} (${subtask.status}): ${subtask.summary}`).join("\n"),
    );
  }
  if (issue.linkedIssues.length > 0) {
    sections.push(
      "## Linked issues",
      issue.linkedIssues
        .map((linked) => `- ${linked.relationship} ${linked.key} (${linked.status}): ${linked.summary}`)
        .join("\n"),
    );
  }
  if (issue.comments.length > 0) {
    sections.push("## Comments", ...issue.comments.map(commentBlock));
  }

  return sections.join("\n\n");
}

function commentBlock(comment: JiraComment): string {
  const date = isoDate(comment.created);
  const heading = date ? `${comment.author} (${date}):` : `${comment.author}:`;
  return `${heading}\n${comment.body.trim()}`;
}
