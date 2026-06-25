// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { FollowupItem } from "$lib/types";
import FollowupsView from "./FollowupsView.svelte";

afterEach(() => cleanup());

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "followup-1",
    noteId: "note-1",
    noteTitle: "Badge printer note",
    text: "Check badge printer template alignment.",
    owner: "Rina",
    dueDate: null,
    status: "accepted",
    source: "parser",
    confidence: 0.78,
    followupState: "open",
    followupLane: null,
    createdAt: "2026-05-20T13:42:00.000Z",
    tags: [{ id: "tag-topic", name: "Badges", kind: "topic", source: "ai", confidence: 0.8 }],
    ...overrides,
  };
}

describe("FollowupsView", () => {
  it("groups follow-ups by lane and dispatches open note", async () => {
    const openNote = vi.fn();

    render(FollowupsView, {
      props: {
        followups: [
          followup({
            id: "alpha",
            noteId: "note-alpha",
            noteTitle: "Alpha planning",
            text: "Confirm the alpha date.",
            followupLane: "Alpha",
          }),
          followup({
            id: "ops",
            noteId: "note-ops",
            noteTitle: "Ops review",
            text: "Review the ops checklist.",
            tags: [{ id: "project", name: "Operations", kind: "project", source: "ai", confidence: 0.9 }],
          }),
          followup({
            id: "done",
            noteTitle: "Alpha closeout",
            text: "Archive the alpha checklist.",
            followupLane: "Alpha",
            status: "done",
            followupState: null,
          }),
        ],
      },
      events: { openNote },
    });

    expect(screen.getByRole("heading", { name: "Follow-ups" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Alpha" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Operations" })).toBeTruthy();
    expect(screen.getAllByText("1 active")).toHaveLength(2);
    expect(screen.getByText("Confirm the alpha date.")).toBeTruthy();
    expect(screen.getByText("Alpha planning")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Alpha planning" }));

    expect(openNote.mock.calls[0][0].detail).toBe("note-alpha");
  });

  it("dispatches updateState, updateLane, complete, and reopen", async () => {
    const updateState = vi.fn();
    const updateLane = vi.fn();
    const complete = vi.fn();
    const reopen = vi.fn();

    render(FollowupsView, {
      props: {
        followups: [
          followup({
            id: "accepted",
            text: "Send the launch follow-up.",
            noteTitle: "Launch source",
            followupLane: "Launch",
          }),
          followup({
            id: "closed",
            text: "Close the renewal loop.",
            noteTitle: "Renewal source",
            followupLane: "Renewals",
            status: "done",
            followupState: null,
          }),
        ],
      },
      events: { updateState, updateLane, complete, reopen },
    });

    await fireEvent.change(screen.getByRole("combobox", { name: "State for: Send the launch follow-up." }), {
      target: { value: "blocked" },
    });

    await fireEvent.input(screen.getByRole("textbox", { name: "Lane for: Send the launch follow-up." }), {
      target: { value: "Escalations" },
    });
    await fireEvent.blur(screen.getByRole("textbox", { name: "Lane for: Send the launch follow-up." }));

    await fireEvent.click(screen.getByRole("button", { name: "Complete follow-up: Send the launch follow-up." }));

    expect(screen.getByText("Done")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Reopen follow-up: Close the renewal loop." }));

    expect(updateState.mock.calls[0][0].detail).toEqual({ id: "accepted", state: "blocked" });
    expect(updateLane.mock.calls[0][0].detail).toEqual({ id: "accepted", lane: "Escalations" });
    expect(complete.mock.calls[0][0].detail).toBe("accepted");
    expect(reopen.mock.calls[0][0].detail).toBe("closed");
  });

  it("shows empty state", () => {
    render(FollowupsView, {
      props: { followups: [] },
    });

    expect(screen.getByText("No follow-ups yet")).toBeTruthy();
  });
});
