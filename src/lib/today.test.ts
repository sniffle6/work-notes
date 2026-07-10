import { describe, expect, it } from "vitest";

import type { ActionReviewItem, FollowupItem } from "$lib/types";
import {
  buildCalendarMonth,
  buildCalendarTasks,
  tasksForCalendarDate,
  unplacedDoneTasks,
  unscheduledOpenTasks,
} from "$lib/today";

const now = new Date("2026-05-22T15:00:00");

function suggestion(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "suggested-1",
    noteId: "note-1",
    noteTitle: "Source note",
    text: "Review the launch recap",
    owner: "Maya",
    dueDate: "2026-05-22",
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00",
    ...overrides,
  };
}

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "followup-1",
    noteId: "note-1",
    noteTitle: "Source note",
    text: "Send the launch recap",
    dueDate: "2026-05-22",
    status: "accepted",
    source: "parser",
    tags: [],
    createdAt: "2026-05-20T13:42:00",
    completedAt: null,
    ...overrides,
  };
}

describe("calendar helpers", () => {
  it("combines suggested, open, and done tasks without losing lifecycle state", () => {
    const tasks = buildCalendarTasks(
      [suggestion()],
      [
        followup(),
        followup({ id: "done-1", status: "done", completedAt: "2026-05-22T11:30:00" }),
      ],
    );

    expect(tasks.map(({ id, status, completedAt }) => ({ id, status, completedAt }))).toEqual([
      { id: "suggested-1", status: "suggested", completedAt: null },
      { id: "followup-1", status: "accepted", completedAt: null },
      { id: "done-1", status: "done", completedAt: "2026-05-22T11:30:00" },
    ]);
  });

  it("builds a six-week month grid with open work on due dates and done work on completion dates", () => {
    const tasks = buildCalendarTasks(
      [suggestion()],
      [
        followup({ id: "overdue", dueDate: "2026-05-21" }),
        followup({ id: "done", status: "done", dueDate: "2026-05-19", completedAt: "2026-05-22T09:00:00" }),
      ],
    );
    const month = buildCalendarMonth(tasks, now, now);
    const may22 = month.find((day) => day.key === "2026-05-22");
    const may19 = month.find((day) => day.key === "2026-05-19");

    expect(month).toHaveLength(42);
    expect(may22).toMatchObject({ isToday: true, openCount: 1, doneCount: 1 });
    expect(may19).toMatchObject({ openCount: 0, doneCount: 0 });
  });

  it("adds overdue work to today's agenda and keeps undated tasks in explicit buckets", () => {
    const tasks = buildCalendarTasks(
      [suggestion({ id: "undated", dueDate: null })],
      [
        followup({ id: "overdue", dueDate: "2026-05-21" }),
        followup({ id: "today", dueDate: "2026-05-22" }),
        followup({ id: "historic", status: "done", completedAt: null }),
      ],
    );

    expect(tasksForCalendarDate(tasks, now, true).open.map((task) => task.id)).toEqual(["overdue", "today"]);
    expect(unscheduledOpenTasks(tasks).map((task) => task.id)).toEqual(["undated"]);
    expect(unplacedDoneTasks(tasks).map((task) => task.id)).toEqual(["historic"]);
  });
});
