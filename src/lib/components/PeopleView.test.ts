// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem, NoteListItem } from "$lib/types";
import PeopleView from "./PeopleView.svelte";

afterEach(() => cleanup());

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Maria launch note",
    rawText: "Maria asked for the launch checklist.",
    summary: "Launch checklist follow-up.",
    captureSource: "quick_capture",
    createdAt: "2026-05-22T10:30:00.000Z",
    updatedAt: "2026-05-22T10:30:00.000Z",
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

describe("PeopleView", () => {
  const notes = [
    note({ id: "maria-note", title: "Maria launch note" }),
    note({
      id: "jin-note",
      title: "Jin budget review",
      createdAt: "2026-05-21T09:00:00.000Z",
      tags: [{ id: "tag-jin", name: "Jin", kind: "person", source: "ai", confidence: 0.9 }],
    }),
  ];

  const actions = [
    action({ id: "you-owe-maria", noteId: "maria-note", owner: "me" }),
    action({
      id: "maria-owes-you",
      noteId: "maria-note",
      owner: "Maria",
      text: "Maria to review launch dates.",
    }),
    action({
      id: "jin-owes-you",
      noteId: "jin-note",
      owner: "jin",
      text: "Jin to confirm budget owner.",
    }),
  ];

  it("renders people rows and selected person detail", () => {
    render(PeopleView, { props: { notes, actions } });

    expect(screen.getByRole("heading", { name: "People" })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Select Maria/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Select Jin/ })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Maria" })).toBeTruthy();
    expect(screen.getByText("You owe Maria")).toBeTruthy();
    expect(screen.getByText("Maria owes you")).toBeTruthy();
    expect(screen.getByText("Send Maria the launch checklist.")).toBeTruthy();
    expect(screen.getByText("Maria to review launch dates.")).toBeTruthy();
    expect(screen.getByText("Recent notes")).toBeTruthy();
    expect(screen.getByText("Maria launch note")).toBeTruthy();
  });

  it("filters the people list by search text", async () => {
    render(PeopleView, { props: { notes, actions } });

    await fireEvent.input(screen.getByRole("textbox", { name: "Search people" }), {
      target: { value: "jin" },
    });

    expect(screen.queryByRole("button", { name: /Select Maria/ })).toBeNull();
    expect(screen.getByRole("button", { name: /Select Jin/ })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Jin" })).toBeTruthy();
    expect(screen.getByText("Jin owes you")).toBeTruthy();
  });

  it("updates the detail panel when a person is selected", async () => {
    render(PeopleView, { props: { notes, actions } });

    await fireEvent.click(screen.getByRole("button", { name: /Select Jin/ }));

    expect(screen.getByRole("heading", { name: "Jin" })).toBeTruthy();
    expect(screen.getByText("You owe Jin")).toBeTruthy();
    expect(screen.getByText("Jin owes you")).toBeTruthy();
    expect(screen.getByText("Jin to confirm budget owner.")).toBeTruthy();
  });

  it("dispatches openNote from action and recent-note rows", async () => {
    const openNote = vi.fn();
    render(PeopleView, {
      props: { notes, actions },
      events: { openNote },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Send Maria the launch checklist." }));
    await fireEvent.click(screen.getByRole("button", { name: "Open note: Maria launch note" }));

    expect(openNote.mock.calls.map((call) => call[0].detail)).toEqual(["maria-note", "maria-note"]);
  });
});
