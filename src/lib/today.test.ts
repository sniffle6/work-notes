import { describe, expect, it } from "vitest";

import type { ActionReviewItem, NoteListItem } from "$lib/types";
import {
  actionsDueByToday,
  buildWorkWeekActivity,
  formatShortTime,
  formatTodayHeading,
  notesCapturedToday,
} from "$lib/today";

const now = new Date("2026-05-22T15:00:00");

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Source note",
    text: "Send the launch recap",
    owner: "Maya",
    dueDate: "2026-05-22",
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00",
    ...overrides,
  };
}

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Launch recap",
    rawText: "Capture the launch recap.",
    summary: "Launch needs a recap.",
    captureSource: "quick_capture",
    createdAt: "2026-05-22T09:15:00",
    updatedAt: "2026-05-22T09:15:00",
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [],
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    ...overrides,
  };
}

describe("today helpers", () => {
  it("includes overdue and due-today actions sorted by due date then creation", () => {
    const result = actionsDueByToday(
      [
        action({ id: "future", dueDate: "2026-05-23" }),
        action({ id: "today-later", dueDate: "2026-05-22", createdAt: "2026-05-20T10:00:00" }),
        action({ id: "no-date", dueDate: null }),
        action({ id: "invalid", dueDate: "not-a-date" }),
        action({ id: "overdue", dueDate: "2026-05-21", createdAt: "2026-05-21T08:00:00" }),
        action({ id: "today-earlier", dueDate: "2026-05-22", createdAt: "2026-05-19T10:00:00" }),
      ],
      now,
    );

    expect(result.map((item) => item.id)).toEqual(["overdue", "today-earlier", "today-later"]);
  });

  it("detects notes captured on the same local date", () => {
    const result = notesCapturedToday(
      [
        note({ id: "today-morning", createdAt: "2026-05-22T08:00:00" }),
        note({ id: "today-evening", createdAt: "2026-05-22T23:59:00" }),
        note({ id: "yesterday", createdAt: "2026-05-21T23:59:00" }),
        note({ id: "invalid", createdAt: "not-a-date" }),
      ],
      now,
    );

    expect(result.map((item) => item.id)).toEqual(["today-evening", "today-morning"]);
  });

  it("builds five weekday activity entries with capture and due-action counts", () => {
    const result = buildWorkWeekActivity(
      [
        note({ id: "mon-note", createdAt: "2026-05-18T10:00:00" }),
        note({ id: "fri-note-1", createdAt: "2026-05-22T09:00:00" }),
        note({ id: "fri-note-2", createdAt: "2026-05-22T11:00:00" }),
        note({ id: "sat-note", createdAt: "2026-05-23T10:00:00" }),
      ],
      [
        action({ id: "tue-action", dueDate: "2026-05-19" }),
        action({ id: "fri-action", dueDate: "2026-05-22" }),
        action({ id: "sat-action", dueDate: "2026-05-23" }),
        action({ id: "invalid-action", dueDate: "invalid" }),
      ],
      now,
    );

    expect(result.map((day) => day.label)).toEqual(["Mon", "Tue", "Wed", "Thu", "Fri"]);
    expect(result.map(({ label, captureCount, dueActionCount, totalCount, isToday }) => ({
      label,
      captureCount,
      dueActionCount,
      totalCount,
      isToday,
    }))).toEqual([
      { label: "Mon", captureCount: 1, dueActionCount: 0, totalCount: 1, isToday: false },
      { label: "Tue", captureCount: 0, dueActionCount: 1, totalCount: 1, isToday: false },
      { label: "Wed", captureCount: 0, dueActionCount: 0, totalCount: 0, isToday: false },
      { label: "Thu", captureCount: 0, dueActionCount: 0, totalCount: 0, isToday: false },
      { label: "Fri", captureCount: 2, dueActionCount: 1, totalCount: 3, isToday: true },
    ]);
  });

  it("formats the Today heading and compact times", () => {
    expect(formatTodayHeading(now)).toBe("Friday, May 22");
    expect(formatShortTime("2026-05-22T09:05:00")).toMatch(/9:05/);
    expect(formatShortTime("invalid")).toBe("invalid");
  });
});
