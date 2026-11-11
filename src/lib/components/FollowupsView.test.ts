// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";

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

function mixedFollowups(): FollowupItem[] {
  return [
    followup({
      id: "alpha",
      noteId: "note-alpha",
      noteTitle: "Alpha planning",
      text: "Confirm the alpha date.",
      followupLane: "Alpha",
      dueDate: "2020-01-01",
    }),
    followup({
      id: "ops",
      noteId: "note-ops",
      noteTitle: "Ops review",
      text: "Review the ops checklist.",
      owner: null,
      followupState: "waiting",
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
    followup({
      id: "released",
      noteTitle: "Release notes",
      text: "Tag the release.",
      followupLane: "Released",
      status: "done",
      followupState: null,
    }),
  ];
}

describe("FollowupsView", () => {
  it("leads with what is left to do and leaves finished work out", () => {
    render(FollowupsView, { props: { followups: mixedFollowups() } });

    expect(screen.getByRole("heading", { name: "Follow-ups" })).toBeTruthy();
    expect(screen.getByText("2 left to do")).toBeTruthy();
    expect(screen.getByText("1 overdue")).toBeTruthy();
    expect(screen.getByText("1 waiting")).toBeTruthy();
    expect(screen.queryByText(/blocked/)).toBeNull();

    expect(screen.getByRole("heading", { name: "Alpha" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Operations" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Released" })).toBeNull();
    expect(screen.getByText("Confirm the alpha date.")).toBeTruthy();
    expect(screen.queryByText("Archive the alpha checklist.")).toBeNull();
  });

  it("shows who owns each follow-up, when it is due, and the note it came from", async () => {
    const openNote = vi.fn();
    render(FollowupsView, { props: { followups: mixedFollowups() }, events: { openNote } });
    const alpha = screen.getByRole("region", { name: "Alpha" });

    expect(within(alpha).getByText("Rina")).toBeTruthy();
    expect(within(alpha).getByText(/days overdue/)).toBeTruthy();
    await fireEvent.click(within(alpha).getByRole("button", { name: "Open note: Alpha planning" }));

    expect(openNote.mock.calls[0][0].detail).toBe("note-alpha");
  });

  it("shows finished follow-ups on request and lets them be reopened", async () => {
    const reopen = vi.fn();
    render(FollowupsView, { props: { followups: mixedFollowups() }, events: { reopen } });
    const toggle = screen.getByRole("button", { name: "Show 2 done" });
    expect(toggle.getAttribute("aria-pressed")).toBe("false");

    await fireEvent.click(toggle);

    expect(toggle.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("heading", { name: "Released" })).toBeTruthy();
    expect(screen.getByText("Archive the alpha checklist.")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Reopen follow-up: Tag the release." }));
    expect(reopen.mock.calls[0][0].detail).toBe("released");
  });

  it("dispatches updateState, updateLane, and complete", async () => {
    const updateState = vi.fn();
    const updateLane = vi.fn();
    const complete = vi.fn();

    render(FollowupsView, {
      props: {
        followups: [
          followup({
            id: "accepted",
            text: "Send the launch follow-up.",
            noteTitle: "Launch source",
            followupLane: "Launch",
          }),
        ],
      },
      events: { updateState, updateLane, complete },
    });

    await fireEvent.change(screen.getByRole("combobox", { name: "State for: Send the launch follow-up." }), {
      target: { value: "blocked" },
    });

    await fireEvent.input(screen.getByRole("textbox", { name: "Lane for: Send the launch follow-up." }), {
      target: { value: "Escalations" },
    });
    await fireEvent.blur(screen.getByRole("textbox", { name: "Lane for: Send the launch follow-up." }));

    await fireEvent.click(screen.getByRole("button", { name: "Complete follow-up: Send the launch follow-up." }));

    expect(updateState.mock.calls[0][0].detail).toEqual({ id: "accepted", state: "blocked" });
    expect(updateLane.mock.calls[0][0].detail).toEqual({ id: "accepted", lane: "Escalations" });
    expect(complete.mock.calls[0][0].detail).toBe("accepted");
  });

  it("drops one stale follow-up or every open one in a lane", async () => {
    const drop = vi.fn();
    render(FollowupsView, {
      props: {
        followups: [
          followup({ id: "one", text: "Move the build machine.", followupLane: "Build" }),
          followup({ id: "two", text: "Rotate the signing key.", followupLane: "Build" }),
          followup({ id: "old", text: "Old build task.", followupLane: "Build", status: "done", followupState: null }),
          followup({ id: "other", text: "Unrelated.", followupLane: "Other" }),
        ],
      },
      events: { drop },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Drop follow-up: Rotate the signing key." }));
    await fireEvent.click(screen.getByRole("button", { name: "Drop all 2 in Build" }));

    expect(drop.mock.calls[0][0].detail).toEqual({ ids: ["two"], lane: null });
    expect(drop.mock.calls[1][0].detail).toEqual({ ids: ["one", "two"], lane: "Build" });
    expect(screen.queryByRole("button", { name: /Drop all 1 in Other/ })).toBeNull();
  });

  it("says when nothing is left while finished follow-ups are hidden", () => {
    render(FollowupsView, {
      props: { followups: [followup({ status: "done", followupState: null })] },
    });

    expect(screen.getByText("Nothing left to do")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Show 1 done" })).toBeTruthy();
  });

  it("explains the page when there are no follow-ups at all", () => {
    render(FollowupsView, {
      props: { followups: [] },
    });

    expect(screen.getByText("No follow-ups yet")).toBeTruthy();
    expect(screen.getByText(/Accept a suggested action on a note/)).toBeTruthy();
  });
});
