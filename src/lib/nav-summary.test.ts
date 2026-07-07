import { describe, expect, it } from "vitest";

import { buildNavSummary, EMPTY_NAV_SUMMARY } from "./nav-summary";
import type { ActionReviewItem, FollowupItem, NoteListItem, Tag, TagKind } from "$lib/types";

function tag(name: string, kind: TagKind = "topic"): Tag {
  return { id: `tag-${kind}-${name}`, name, kind };
}

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Note",
    rawText: "",
    summary: null,
    captureSource: "quick_capture",
    createdAt: "2026-05-20T10:00:00.000Z",
    updatedAt: "2026-05-20T10:00:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "none",
    isArchived: false,
    tags: [],
    actionItemCount: 0,
    suggestedActionItemCount: 0,
    ...overrides,
  };
}

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Note",
    text: "Do the thing",
    createdAt: "2026-05-20T10:00:00.000Z",
    ...overrides,
  };
}

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "followup-1",
    noteId: "note-1",
    noteTitle: "Note",
    text: "Chase the thing",
    status: "accepted",
    source: "parser",
    tags: [],
    createdAt: "2026-05-20T10:00:00.000Z",
    ...overrides,
  };
}

describe("buildNavSummary", () => {
  it("counts only non-archived notes as the inbox total", () => {
    const summary = buildNavSummary(
      [
        note({ id: "a", isArchived: false }),
        note({ id: "b", isArchived: false }),
        note({ id: "c", isArchived: true }),
      ],
      [],
      [],
    );

    expect(summary.inbox).toBe(2);
  });

  it("treats a missing isArchived flag as active", () => {
    const summary = buildNavSummary([note({ id: "a", isArchived: undefined })], [], []);
    expect(summary.inbox).toBe(1);
  });

  it("counts suggested actions as needs-review", () => {
    const summary = buildNavSummary([], [action({ id: "x" }), action({ id: "y" })], []);
    expect(summary.needsReview).toBe(2);
  });

  it("counts only open follow-ups (status other than done)", () => {
    const summary = buildNavSummary(
      [],
      [],
      [
        followup({ id: "f1", status: "accepted" }),
        followup({ id: "f2", status: "accepted" }),
        followup({ id: "f3", status: "done" }),
      ],
    );

    expect(summary.followups).toBe(2);
  });

  it("counts failed parses among active notes only", () => {
    const summary = buildNavSummary(
      [
        note({ id: "a", parseStatus: "failed", isArchived: false }),
        note({ id: "b", parseStatus: "failed", isArchived: true }),
        note({ id: "c", parseStatus: "parsed", isArchived: false }),
      ],
      [],
      [],
    );

    expect(summary.parseFailed).toBe(1);
  });

  it("counts queued and parsing active notes as the parser queue", () => {
    const summary = buildNavSummary(
      [
        note({ id: "a", parseStatus: "queued" }),
        note({ id: "b", parseStatus: "parsing" }),
        note({ id: "c", parseStatus: "parsed" }),
        note({ id: "d", parseStatus: "queued", isArchived: true }),
      ],
      [],
      [],
    );

    expect(summary.parseQueue).toBe(2);
  });

  it("collects distinct tag names from active notes, sorted, excluding archived notes", () => {
    const summary = buildNavSummary(
      [
        note({ id: "a", tags: [tag("Zebra"), tag("Apple")] }),
        note({ id: "b", tags: [tag("Apple"), tag("Maya", "person")] }),
        note({ id: "c", isArchived: true, tags: [tag("Hidden")] }),
      ],
      [],
      [],
    );

    expect(summary.tags).toEqual(["Apple", "Maya", "Zebra"]);
  });

  it("returns all-zero counts and no tags for empty inputs", () => {
    expect(buildNavSummary([], [], [])).toEqual(EMPTY_NAV_SUMMARY);
  });
});
