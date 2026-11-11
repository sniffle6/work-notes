import { isoDate, workInstructions } from "./agent-prompt";
import type { ActionItem, ActionStatus, NoteDetail } from "./types";

/**
 * Builds a prompt that hands a note to a coding agent. It carries the note's
 * text, action items, and added notes so the agent needs no access to the app.
 * Suggested action items are marked as unconfirmed. Dismissed ones are left out.
 */
export function buildNotePrompt(note: NoteDetail): string {
  const captured = isoDate(note.createdAt);
  const jiraKeys = note.jiraIssueKeys ?? [];
  const facts = [
    captured ? `Captured: ${captured}` : null,
    note.tags.length > 0 ? `Tags: ${note.tags.map((tag) => tag.name).join(", ")}` : null,
    jiraKeys.length > 0 ? `Jira: ${jiraKeys.join(", ")}` : null,
  ].filter((fact): fact is string => fact !== null);

  const sections = [`Work on this note: ${note.title}`, workInstructions("note")];
  if (facts.length > 0) {
    sections.push(facts.join("\n"));
  }

  const summary = note.summary?.trim();
  if (summary && summary !== note.title) {
    sections.push("## Summary", summary);
  }
  sections.push("## Note", (note.cleanedText?.trim() || note.rawText).trim());

  pushActions(sections, "## Action items", note, "accepted");
  pushActions(sections, "## Suggested action items (inferred, not confirmed by me)", note, "suggested");
  pushActions(sections, "## Already done", note, "done");

  const cardNotes = note.cardNotes ?? [];
  if (cardNotes.length > 0) {
    sections.push(
      "## Added notes",
      ...cardNotes.map((cardNote) => {
        const date = isoDate(cardNote.createdAt);
        return date ? `${date}:\n${cardNote.text.trim()}` : cardNote.text.trim();
      }),
    );
  }

  return sections.join("\n\n");
}

function pushActions(sections: string[], heading: string, note: NoteDetail, status: ActionStatus) {
  const actions = note.actionItems.filter((action) => action.status === status);
  if (actions.length > 0) {
    sections.push(heading, actions.map(actionLine).join("\n"));
  }
}

function actionLine(action: ActionItem): string {
  const details = [
    action.owner ? `owner: ${action.owner}` : null,
    action.dueDate ? `due: ${action.dueDate}` : null,
  ].filter((detail): detail is string => detail !== null);
  return details.length > 0 ? `- ${action.text} (${details.join(", ")})` : `- ${action.text}`;
}
