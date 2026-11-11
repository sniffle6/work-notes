import { describe, expect, it } from "vitest";

import { buildNotePrompt } from "./note-prompt";
import type { ActionItem, NoteDetail } from "./types";

function action(id: string, text: string, status: ActionItem["status"], extra: Partial<ActionItem> = {}): ActionItem {
  return { id, noteId: "note-1", text, status, source: "parser", ...extra };
}

function note(overrides: Partial<NoteDetail> = {}): NoteDetail {
  return {
    id: "note-1",
    title: "Design Reusable Math Engine",
    rawText: "math engine 5x3 reels, reusable",
    cleanedText: "## Requirements\n\n- Bake the math engine into the games.",
    summary: "Define and test a reusable math engine.",
    captureSource: "quick_capture",
    createdAt: "2026-11-11T09:00:00.000Z",
    updatedAt: "2026-11-11T09:00:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "reviewed",
    tags: [
      { id: "t1", name: "reel games", kind: "topic" },
      { id: "t2", name: "Godot", kind: "project" },
    ],
    actionItemCount: 4,
    suggestedActionItemCount: 1,
    actionItems: [
      action("a1", "Adjust the paylines", "accepted", { owner: "me", dueDate: "2026-11-20" }),
      action("a2", "Add tests", "accepted"),
      action("a3", "Ask Zach about RNG", "suggested"),
      action("a4", "Pick the engine name", "done"),
      action("a5", "Port to Unity", "dismissed"),
    ],
    cardNotes: [{ id: "c1", noteId: "note-1", text: "Robert wants Class 3 only.", createdAt: "2026-11-12T08:00:00.000Z" }],
    jiraIssueKeys: ["AB-7"],
    ...overrides,
  };
}

describe("buildNotePrompt", () => {
  it("builds a self-contained prompt from the note, its actions, and its added notes", () => {
    expect(buildNotePrompt(note())).toBe(
      [
        "Work on this note: Design Reusable Math Engine",
        "",
        "Read the note below and investigate the relevant code before changing anything. Then tell me what you understand the note to ask for and how you plan to do it.",
        "",
        "Before you implement, grill me on anything you are not sure about: requirements, edge cases, scope, and how the result will be verified. Ask one question at a time and do not guess. Start the work only once we both agree on what needs doing, then implement and verify it.",
        "",
        "Captured: 2026-11-11",
        "Tags: reel games, Godot",
        "Jira: AB-7",
        "",
        "## Summary",
        "",
        "Define and test a reusable math engine.",
        "",
        "## Note",
        "",
        "## Requirements",
        "",
        "- Bake the math engine into the games.",
        "",
        "## Action items",
        "",
        "- Adjust the paylines (owner: me, due: 2026-11-20)",
        "- Add tests",
        "",
        "## Suggested action items (inferred, not confirmed by me)",
        "",
        "- Ask Zach about RNG",
        "",
        "## Already done",
        "",
        "- Pick the engine name",
        "",
        "## Added notes",
        "",
        "2026-11-12:",
        "Robert wants Class 3 only.",
      ].join("\n"),
    );
  });

  it("uses the raw text when the note has no cleaned text and leaves out empty sections", () => {
    const prompt = buildNotePrompt(
      note({ cleanedText: null, summary: null, tags: [], actionItems: [], cardNotes: [], jiraIssueKeys: [] }),
    );

    expect(prompt.endsWith("Captured: 2026-11-11\n\n## Note\n\nmath engine 5x3 reels, reusable")).toBe(true);
    expect(prompt).not.toContain("Tags:");
    expect(prompt).not.toContain("Jira:");
    expect(prompt).not.toContain("## Summary");
    expect(prompt).not.toContain("## Action items");
  });

  it("does not repeat a summary that only restates the title", () => {
    const prompt = buildNotePrompt(note({ summary: "Design Reusable Math Engine" }));

    expect(prompt).not.toContain("## Summary");
  });
});
