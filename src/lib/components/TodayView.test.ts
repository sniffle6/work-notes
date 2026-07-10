// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem, FollowupItem } from "$lib/types";
import TodayView from "./TodayView.svelte";

afterEach(() => cleanup());

const now = new Date("2026-05-22T15:00:00");

function suggestion(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "suggested-1",
    noteId: "note-suggested",
    noteTitle: "Launch source note",
    text: "Review launch summary",
    dueDate: "2026-05-22",
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00",
    ...overrides,
  };
}

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "open-1",
    noteId: "note-open",
    noteTitle: "Launch source note",
    text: "Send EOD launch summary",
    owner: "Maya",
    dueDate: "2026-05-22",
    status: "accepted",
    source: "parser",
    tags: [],
    createdAt: "2026-05-20T13:42:00",
    completedAt: null,
    ...overrides,
  };
}

describe("TodayView calendar", () => {
  it("defaults to today and shows open and done buckets on a full month grid", () => {
    render(TodayView, {
      props: {
        actions: [suggestion()],
        followups: [
          followup(),
          followup({
            id: "done-1",
            text: "Publish launch notes",
            status: "done",
            completedAt: "2026-05-22T11:30:00",
          }),
        ],
        now,
      },
    });

    expect(screen.getByRole("heading", { name: "May 2026" })).toBeTruthy();
    const navigation = screen.getByRole("group", { name: "Calendar navigation" });
    expect(Array.from(navigation.children).map((child) => child.getAttribute("aria-label") ?? child.textContent)).toEqual([
      "Previous month",
      "May 2026",
      "Next month",
      "Today",
    ]);
    expect(screen.getByRole("heading", { name: "Friday, May 22" })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Select Friday, May 22, 2 open, 1 done/ }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Accept task: Review launch summary" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Mark done: Send EOD launch summary" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reopen task: Publish launch notes" })).toBeTruthy();
  });

  it("navigates months and returns to today", async () => {
    render(TodayView, { props: { actions: [], followups: [], now } });

    await fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("heading", { name: "June 2026" })).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Today" }));
    expect(screen.getByRole("heading", { name: "May 2026" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Friday, May 22" })).toBeTruthy();
  });

  it("dispatches review, complete, reopen, and source-note events", async () => {
    const accept = vi.fn();
    const complete = vi.fn();
    const reopen = vi.fn();
    const openNote = vi.fn();

    render(TodayView, {
      props: {
        actions: [suggestion()],
        followups: [
          followup(),
          followup({ id: "done-1", text: "Publish launch notes", status: "done", completedAt: "2026-05-22T11:30:00" }),
        ],
        now,
      },
      events: { accept, complete, reopen, openNote },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Accept task: Review launch summary" }));
    await fireEvent.click(screen.getByRole("button", { name: "Mark done: Send EOD launch summary" }));
    await fireEvent.click(screen.getByRole("button", { name: "Reopen task: Publish launch notes" }));
    await fireEvent.click(screen.getAllByRole("button", { name: "Open note: Launch source note" })[0]);

    expect(accept.mock.calls[0][0].detail).toBe("suggested-1");
    expect(complete.mock.calls[0][0].detail).toBe("open-1");
    expect(reopen.mock.calls[0][0].detail).toBe("done-1");
    expect(openNote.mock.calls[0][0].detail).toBe("note-open");
  });
});
