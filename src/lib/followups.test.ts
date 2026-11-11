import { describe, expect, it } from "vitest";

import type { FollowupItem } from "./types";
import {
  describeDue,
  displayFollowupState,
  groupFollowupsByLane,
  lanesToShow,
  resolveFollowupLane,
  summarizeFollowups,
} from "./followups";

function followup(overrides: Partial<FollowupItem> = {}): FollowupItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Visitor badge printer",
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

describe("resolveFollowupLane", () => {
  it("uses explicit lane, then project, then topic, then Unsorted", () => {
    expect(resolveFollowupLane(followup({ followupLane: "Ops" }))).toBe("Ops");
    expect(
      resolveFollowupLane(
        followup({
          tags: [
            { id: "topic", name: "Badges", kind: "topic", source: "ai", confidence: 0.8 },
            { id: "project", name: "Front desk", kind: "project", source: "ai", confidence: 0.9 },
          ],
        }),
      ),
    ).toBe("Front desk");
    expect(resolveFollowupLane(followup())).toBe("Badges");
    expect(resolveFollowupLane(followup({ tags: [] }))).toBe("Unsorted");
  });
});

describe("groupFollowupsByLane", () => {
  it("groups and counts active follow-ups by lane", () => {
    const groups = groupFollowupsByLane([
      followup({ id: "open", followupLane: "Ops", followupState: "open" }),
      followup({ id: "blocked", followupLane: "Ops", followupState: "blocked" }),
      followup({ id: "done", followupLane: "Ops", status: "done", followupState: null }),
    ]);

    expect(groups).toEqual([
      {
        name: "Ops",
        activeCount: 2,
        followups: [
          expect.objectContaining({ id: "open" }),
          expect.objectContaining({ id: "blocked" }),
          expect.objectContaining({ id: "done" }),
        ],
      },
    ]);
  });

  it("sorts lanes alphabetically with Unsorted last", () => {
    const groups = groupFollowupsByLane([
      followup({ id: "unsorted", tags: [] }),
      followup({ id: "zebra", followupLane: "Zebra" }),
      followup({ id: "alpha", followupLane: "Alpha" }),
    ]);

    expect(groups.map((group) => group.name)).toEqual(["Alpha", "Zebra", "Unsorted"]);
  });
});

describe("displayFollowupState", () => {
  it("maps done status to Done and active metadata to labels", () => {
    expect(displayFollowupState(followup({ status: "done", followupState: null }))).toBe("Done");
    expect(displayFollowupState(followup({ followupState: "waiting" }))).toBe("Waiting");
    expect(displayFollowupState(followup({ followupState: "blocked" }))).toBe("Blocked");
    expect(displayFollowupState(followup({ followupState: null }))).toBe("Open");
  });
});

describe("summarizeFollowups", () => {
  it("counts what is left to do by state, what is overdue, and what is done", () => {
    const today = new Date(2026, 10, 11);

    expect(
      summarizeFollowups(
        [
          followup({ id: "a" }),
          followup({ id: "b", followupState: null, dueDate: "2026-11-10" }),
          followup({ id: "c", followupState: "waiting", dueDate: "2026-11-11" }),
          followup({ id: "d", followupState: "blocked", dueDate: "2026-11-01" }),
          followup({ id: "e", status: "done", followupState: null, dueDate: "2026-11-01" }),
        ],
        today,
      ),
    ).toEqual({ open: 2, waiting: 1, blocked: 1, overdue: 2, done: 1 });
  });
});

describe("lanesToShow", () => {
  const lanes = groupFollowupsByLane([
    followup({ id: "done-1", followupLane: "Released", status: "done", followupState: null }),
    followup({ id: "open-1", followupLane: "Kiosk" }),
    followup({ id: "done-2", followupLane: "Kiosk", status: "done", followupState: null }),
    followup({ id: "blocked-1", followupLane: "Kiosk", followupState: "blocked" }),
  ]);

  it("hides finished follow-ups and lanes with nothing left, blocked work first, when done is hidden", () => {
    const shown = lanesToShow(lanes, false);

    expect(shown.map((lane) => [lane.name, lane.followups.map((item) => item.id)])).toEqual([
      ["Kiosk", ["blocked-1", "open-1"]],
    ]);
    expect(shown[0].activeCount).toBe(2);
  });

  it("lists finished follow-ups after the open ones when done is shown", () => {
    expect(lanesToShow(lanes, true).map((lane) => [lane.name, lane.followups.map((item) => item.id)])).toEqual([
      ["Kiosk", ["blocked-1", "open-1", "done-2"]],
      ["Released", ["done-1"]],
    ]);
  });
});

describe("lanesToShow ordering", () => {
  const today = new Date(2026, 10, 11);

  it("puts lanes with overdue or blocked work first and keeps the rest in order", () => {
    const lanes = groupFollowupsByLane([
      followup({ id: "a-open", followupLane: "Alpha" }),
      followup({ id: "b-due-later", followupLane: "Bravo", dueDate: "2026-12-01" }),
      followup({ id: "c-open", followupLane: "Charlie" }),
      followup({ id: "c-overdue", followupLane: "Charlie", dueDate: "2026-11-01" }),
      followup({ id: "d-blocked", followupLane: "Delta", followupState: "blocked" }),
      followup({ id: "e-done-late", followupLane: "Echo", status: "done", followupState: null, dueDate: "2026-11-01" }),
      followup({ id: "e-open", followupLane: "Echo" }),
    ]);

    const shown = lanesToShow(lanes, false, today);

    expect(shown.map((lane) => lane.name)).toEqual(["Charlie", "Delta", "Alpha", "Bravo", "Echo"]);
    expect(shown[0].followups.map((item) => item.id)).toEqual(["c-overdue", "c-open"]);
  });
});

describe("describeDue", () => {
  const today = new Date(2026, 10, 11);

  it("says how late, how soon, or on which day something is due", () => {
    expect(describeDue("2026-11-11", today)).toEqual({ text: "Due today", overdue: false });
    expect(describeDue("2026-11-12", today)).toEqual({ text: "Due tomorrow", overdue: false });
    expect(describeDue("2026-11-23", today)).toEqual({ text: "Due Nov 23", overdue: false });
    expect(describeDue("2026-11-10", today)).toEqual({ text: "1 day overdue", overdue: true });
    expect(describeDue("2026-11-01T09:00:00Z", today)).toEqual({ text: "10 days overdue", overdue: true });
    expect(describeDue("2027-01-05", today)).toEqual({ text: "Due Jan 5, 2027", overdue: false });
  });

  it("returns nothing without a date and keeps text it cannot read as a date", () => {
    expect(describeDue(null, today)).toBeNull();
    expect(describeDue("  ", today)).toBeNull();
    expect(describeDue("next sprint", today)).toEqual({ text: "Due next sprint", overdue: false });
  });
});
