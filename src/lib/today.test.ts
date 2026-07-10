import { describe, expect, it } from "vitest";

import type { ActionReviewItem, FollowupItem } from "$lib/types";
import {
  buildCalendarMonth,
  buildCalendarTasks,
  buildTaskLifecycle,
  calendarOccurrencesForDate,
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

    expect(tasks.map(({ id, status, capturedAt, completedAt }) => ({ id, status, capturedAt, completedAt }))).toEqual([
      { id: "suggested-1", status: "suggested", capturedAt: "2026-05-20T13:42:00", completedAt: null },
      { id: "followup-1", status: "accepted", capturedAt: "2026-05-20T13:42:00", completedAt: null },
      { id: "done-1", status: "done", capturedAt: "2026-05-20T13:42:00", completedAt: "2026-05-22T11:30:00" },
    ]);
  });

  it("projects tasks onto captured, due, and completed dates", () => {
    const tasks = buildCalendarTasks(
      [suggestion()],
      [
        followup({ id: "overdue", dueDate: "2026-05-21" }),
        followup({ id: "done", status: "done", dueDate: "2026-05-19", completedAt: "2026-05-22T09:00:00" }),
      ],
    );

    expect(calendarOccurrencesForDate(tasks, new Date("2026-05-20T12:00:00"))).toHaveLength(3);
    expect(calendarOccurrencesForDate(tasks, new Date("2026-05-19T12:00:00"))).toMatchObject([
      { kinds: ["due"], task: { id: "done" } },
    ]);
    expect(calendarOccurrencesForDate(tasks, now).map(({ kinds, task }) => ({ id: task.id, kinds }))).toEqual([
      { id: "suggested-1", kinds: ["due"] },
      { id: "done", kinds: ["completed"] },
    ]);
  });

  it("builds a six-week month grid with lifecycle counts", () => {
    const tasks = buildCalendarTasks(
      [suggestion()],
      [followup({ id: "done", status: "done", dueDate: "2026-05-19", completedAt: "2026-05-22T09:00:00" })],
    );
    const month = buildCalendarMonth(tasks, now, now);
    const may22 = month.find((day) => day.key === "2026-05-22");
    const may19 = month.find((day) => day.key === "2026-05-19");
    const may20 = month.find((day) => day.key === "2026-05-20");

    expect(month).toHaveLength(42);
    expect(may22).toMatchObject({ isToday: true, capturedCount: 0, dueCount: 1, completedCount: 1 });
    expect(may19).toMatchObject({ capturedCount: 0, dueCount: 1, completedCount: 0 });
    expect(may20).toMatchObject({ capturedCount: 2, dueCount: 0, completedCount: 0 });
  });

  it("merges same-day moments and keeps undated or historically completed work on its captured date", () => {
    const tasks = buildCalendarTasks(
      [suggestion({ id: "same-day", dueDate: "2026-05-20" })],
      [
        followup({ id: "undated", dueDate: null }),
        followup({ id: "historic", status: "done", dueDate: null, completedAt: null }),
      ],
    );
    const captured = calendarOccurrencesForDate(tasks, new Date("2026-05-20T12:00:00"));

    expect(captured.map(({ task, kinds }) => ({ id: task.id, kinds }))).toEqual([
      { id: "undated", kinds: ["captured"] },
      { id: "same-day", kinds: ["captured", "due"] },
      { id: "historic", kinds: ["captured"] },
    ]);
  });

  it("builds a lifecycle with selected, future, and overdue moment semantics", () => {
    const [openTask, overdueTask] = buildCalendarTasks([], [
      followup({ id: "open", dueDate: "2026-05-25" }),
      followup({ id: "overdue", dueDate: "2026-05-21" }),
    ]);

    expect(buildTaskLifecycle(openTask, now, now)).toEqual([
      { kind: "captured", dateKey: "2026-05-20", position: "past", isOverdue: false },
      { kind: "due", dateKey: "2026-05-25", position: "future", isOverdue: false },
    ]);
    expect(buildTaskLifecycle(overdueTask, now, now)).toEqual([
      { kind: "captured", dateKey: "2026-05-20", position: "past", isOverdue: false },
      { kind: "due", dateKey: "2026-05-21", position: "past", isOverdue: true },
    ]);
  });
});
