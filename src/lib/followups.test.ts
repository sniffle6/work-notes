import { describe, expect, it } from "vitest";

import type { FollowupItem } from "./types";
import { displayFollowupState, groupFollowupsByLane, resolveFollowupLane } from "./followups";

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
