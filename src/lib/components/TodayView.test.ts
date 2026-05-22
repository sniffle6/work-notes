// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem, NoteListItem } from "$lib/types";
import TodayView from "./TodayView.svelte";

afterEach(() => cleanup());

const now = new Date("2026-05-22T15:00:00");

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Launch source note",
    text: "Send EOD launch summary",
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
    title: "Launch source note",
    rawText: "Maya needs the launch summary.",
    summary: "Launch summary is due.",
    captureSource: "quick_capture",
    createdAt: "2026-05-22T09:15:00",
    updatedAt: "2026-05-22T09:15:00",
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [
      { id: "tag-1", name: "Launch", kind: "topic", source: "ai", confidence: 0.9 },
      { id: "tag-2", name: "Maya", kind: "person", source: "ai", confidence: 0.8 },
    ],
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    ...overrides,
  };
}

describe("TodayView", () => {
  it("renders due actions, captured notes, and weekday activity", () => {
    render(TodayView, {
      props: {
        notes: [
          note({ id: "today-note", title: "Captured launch note" }),
          note({ id: "old-note", title: "Old note", createdAt: "2026-05-21T09:15:00" }),
        ],
        actions: [action()],
        now,
      },
    });

    expect(screen.getByRole("heading", { name: "Today" })).toBeTruthy();
    expect(screen.getByText("Friday, May 22")).toBeTruthy();
    expect(screen.getByText("Send EOD launch summary")).toBeTruthy();
    expect(screen.getByText(/from "Launch source note"/)).toBeTruthy();
    expect(screen.getByText("Captured launch note")).toBeTruthy();
    expect(screen.queryByText("Old note")).toBeNull();

    for (const label of ["Mon", "Tue", "Wed", "Thu", "Fri"]) {
      expect(screen.getByText(label)).toBeTruthy();
    }
  });

  it("dispatches accept without opening the source note", async () => {
    const accept = vi.fn();
    const openNote = vi.fn();

    render(TodayView, {
      props: {
        notes: [note()],
        actions: [action()],
        now,
      },
      events: { accept, openNote },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Accept action: Send EOD launch summary" }));

    expect(accept.mock.calls[0][0].detail).toBe("action-1");
    expect(openNote).not.toHaveBeenCalled();
  });

  it("opens action and note rows through openNote events", async () => {
    const openNote = vi.fn();

    render(TodayView, {
      props: {
        notes: [note({ id: "note-captured", title: "Captured launch note" })],
        actions: [action({ noteId: "note-action" })],
        now,
      },
      events: { openNote },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Send EOD launch summary" }));
    await fireEvent.click(screen.getByRole("button", { name: "Open note: Captured launch note" }));

    expect(openNote.mock.calls.map((call) => call[0].detail)).toEqual(["note-action", "note-captured"]);
  });
});
