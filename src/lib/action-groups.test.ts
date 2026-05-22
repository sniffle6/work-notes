import { describe, expect, it } from "vitest";

import type { ActionReviewItem } from "$lib/types";
import { actionMatchesSearch, groupActionsByDueBucket } from "./action-groups";

const now = new Date("2026-05-22T12:00:00.000Z");

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Maria dashboard owner",
    text: "Decide whether to rebuild the dashboard",
    owner: "me",
    dueDate: null,
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

describe("groupActionsByDueBucket", () => {
  it("groups actions into due buckets in design order", () => {
    const groups = groupActionsByDueBucket(
      [
        action({ id: "no-date", dueDate: null }),
        action({ id: "overdue", dueDate: "2026-05-21" }),
        action({ id: "today", dueDate: "2026-05-22" }),
        action({ id: "week", dueDate: "2026-05-25" }),
        action({ id: "later", dueDate: "2026-06-10" }),
      ],
      now,
    );

    expect(groups.map((group) => [group.label, group.actions.map((item) => item.id)])).toEqual([
      ["Overdue", ["overdue"]],
      ["Today", ["today"]],
      ["This week", ["week"]],
      ["Later", ["later"]],
      ["No date", ["no-date"]],
    ]);
  });
});

describe("actionMatchesSearch", () => {
  it("matches text, owner, due date, and source note title", () => {
    const item = action({
      owner: "Maria",
      dueDate: "2026-05-25",
      noteTitle: "Q3 metrics migration",
      text: "Forward failing queries",
    });

    expect(actionMatchesSearch(item, "failing")).toBe(true);
    expect(actionMatchesSearch(item, "maria")).toBe(true);
    expect(actionMatchesSearch(item, "2026-05-25")).toBe(true);
    expect(actionMatchesSearch(item, "metrics")).toBe(true);
    expect(actionMatchesSearch(item, "payroll")).toBe(false);
  });

  it("matches displayed due labels", () => {
    expect(actionMatchesSearch(action({ dueDate: "2026-05-21" }), "1d overdue", now)).toBe(true);
    expect(actionMatchesSearch(action({ dueDate: "2026-05-22" }), "today", now)).toBe(true);
    expect(actionMatchesSearch(action({ dueDate: "2026-05-23" }), "tomorrow", now)).toBe(true);
  });
});
