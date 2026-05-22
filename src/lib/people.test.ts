import { describe, expect, it } from "vitest";

import type { ActionReviewItem, NoteListItem } from "$lib/types";
import {
  avatarHue,
  buildPeople,
  buildPersonDetail,
  formatPersonWhen,
  matchesPersonSearch,
} from "$lib/people";

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Maria launch note",
    rawText: "Maria asked for the launch checklist.",
    summary: "Launch checklist follow-up.",
    captureSource: "quick_capture",
    createdAt: "2026-05-20T13:00:00.000Z",
    updatedAt: "2026-05-20T13:00:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [{ id: "tag-1", name: "Maria", kind: "person", source: "ai", confidence: 0.92 }],
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    ...overrides,
  };
}

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Maria launch note",
    text: "Send Maria the launch checklist.",
    owner: "me",
    dueDate: "2026-05-22",
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

describe("people helpers", () => {
  const notes = [
    note({ id: "maria-old", title: "Maria kickoff", createdAt: "2026-05-20T13:00:00.000Z" }),
    note({
      id: "jin-note",
      title: "Jin budget review",
      createdAt: "2026-05-21T09:00:00.000Z",
      tags: [{ id: "tag-jin", name: "Jin", kind: "person", source: "ai", confidence: 0.9 }],
    }),
    note({
      id: "maria-new",
      title: "Maria launch follow-up",
      createdAt: "2026-05-22T10:30:00.000Z",
      tags: [{ id: "tag-maria-2", name: "maria", kind: "person", source: "ai", confidence: 0.88 }],
    }),
    note({
      id: "untagged-note",
      title: "Private admin",
      tags: [{ id: "tag-topic", name: "Admin", kind: "topic", source: "ai", confidence: 0.7 }],
    }),
  ];

  const actions = [
    action({ id: "you-owe-maria", noteId: "maria-new", owner: "me", text: "Send Maria the launch checklist." }),
    action({
      id: "maria-owes-you",
      noteId: "maria-old",
      owner: "Maria",
      text: "Maria to review the launch dates.",
      dueDate: "2026-05-21",
    }),
    action({
      id: "jin-owes-you",
      noteId: "jin-note",
      owner: "jin",
      text: "Jin to confirm the budget owner.",
      dueDate: null,
    }),
    action({
      id: "me-untagged",
      noteId: "untagged-note",
      owner: "me",
      text: "Write private admin notes.",
    }),
    action({
      id: "you-owe-jin",
      noteId: "jin-note",
      owner: "me",
      text: "Send Jin the vendor export.",
      dueDate: "2026-05-20",
    }),
  ];

  it("builds people from person tags and non-me action owners", () => {
    const people = buildPeople(notes, actions);

    expect(people.map((person) => person.name)).toEqual(["Maria", "Jin"]);
    expect(people.some((person) => person.key === "me")).toBe(false);

    const maria = people.find((person) => person.key === "maria");
    expect(maria).toMatchObject({
      name: "Maria",
      noteCount: 2,
      actionCount: 1,
      lastInteractionAt: "2026-05-22T10:30:00.000Z",
    });
  });

  it("builds selected person detail with direction-aware actions and recent notes", () => {
    const maria = buildPersonDetail("Maria", notes, actions);
    const jin = buildPersonDetail("JIN", notes, actions);

    expect(maria?.youOwe.map((item) => item.id)).toEqual(["you-owe-maria"]);
    expect(maria?.theyOwe.map((item) => item.id)).toEqual(["maria-owes-you"]);
    expect(maria?.youOwe[0].sourceNote?.id).toBe("maria-new");
    expect(maria?.recentNotes.map((item) => item.id)).toEqual(["maria-new", "maria-old"]);

    expect(jin?.youOwe.map((item) => item.id)).toEqual(["you-owe-jin"]);
    expect(jin?.theyOwe.map((item) => item.id)).toEqual(["jin-owes-you"]);
  });

  it("matches search text, formats interaction dates, and returns stable avatar hues", () => {
    const maria = buildPeople(notes, actions)[0];

    expect(matchesPersonSearch(maria, "mari")).toBe(true);
    expect(matchesPersonSearch(maria, "missing")).toBe(false);
    expect(formatPersonWhen(null)).toBe("never");
    expect(formatPersonWhen("not-a-date")).toBe("never");
    expect(formatPersonWhen("2026-05-22T10:30:00.000Z")).not.toBe("never");
    expect(avatarHue("Maria")).toBe(avatarHue("Maria"));
    expect(avatarHue("Maria")).toBeGreaterThanOrEqual(0);
    expect(avatarHue("Maria")).toBeLessThan(360);
  });
});
