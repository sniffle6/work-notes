import { isoDate, workInstructions } from "./agent-prompt";
import { formatFileSize } from "./file-size";
import type { JiraAttachment, JiraAttachmentFile, JiraComment, JiraIssueDetail } from "./types";

/**
 * Builds a prompt that hands a ticket to a coding agent. It carries the
 * ticket's facts, description, subtasks, linked issues, attachments, and
 * comments so the agent needs no Jira access.
 *
 * `attachmentFiles` are the attachments already saved to disk. Those are
 * listed by local path; the rest are named as not downloaded.
 */
export function buildTicketPrompt(issue: JiraIssueDetail, attachmentFiles: JiraAttachmentFile[] = []): string {
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
  if (issue.attachments.length > 0) {
    sections.push("## Attachments", ...attachmentSections(issue.attachments, attachmentFiles));
  }
  if (issue.comments.length > 0) {
    sections.push("## Comments", ...issue.comments.map(commentBlock));
  }

  return sections.join("\n\n");
}

function attachmentSections(attachments: JiraAttachment[], files: JiraAttachmentFile[]): string[] {
  const pathById = new Map(files.map((file) => [file.id, file.path]));
  const anySaved = attachments.some((attachment) => pathById.has(attachment.id));
  if (!anySaved) {
    return [
      "The ticket has attachments that were not downloaded. Ask me for any you need.",
      attachments.map((attachment) => `- ${attachmentLabel(attachment)}`).join("\n"),
    ];
  }

  return [
    "The ticket's attachments are saved as local files. Read the ones that bear on the work.",
    attachments
      .map((attachment) => {
        const path = pathById.get(attachment.id);
        return `- ${attachmentLabel(attachment)}: ${path ?? "not downloaded; ask me for it if you need it"}`;
      })
      .join("\n"),
  ];
}

function attachmentLabel(attachment: JiraAttachment): string {
  const details = [attachment.size > 0 ? formatFileSize(attachment.size) : null, attachment.mimeType].filter(
    (detail): detail is string => Boolean(detail),
  );
  return details.length > 0 ? `${attachment.filename} (${details.join(", ")})` : attachment.filename;
}

function commentBlock(comment: JiraComment): string {
  const date = isoDate(comment.created);
  const heading = date ? `${comment.author} (${date}):` : `${comment.author}:`;
  return `${heading}\n${comment.body.trim()}`;
}
