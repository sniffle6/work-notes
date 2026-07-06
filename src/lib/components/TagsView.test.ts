// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { NoteListItem, Tag, TagKind } from "$lib/types";
import TagsView from "./TagsView.svelte";

afterEach(() => cleanup());

function tag(name: string, kind: TagKind = "topic"): Tag {
  return { id: `tag-${kind}-${name}`, name, kind };
}

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Kiosk telemetry",
    rawText: "",
    summary: null,
    captureSource: "quick_capture",
    createdAt: "2026-05-22T10:30:00.000Z",
    updatedAt: "2026-05-22T10:30:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "none",
    tags: [tag("Kiosk", "project")],
    actionItemCount: 0,
    suggestedActionItemCount: 0,
    ...overrides,
  };
}

describe("TagsView", () => {
  const notes = [
    note({ id: "n1", title: "Kiosk telemetry", tags: [tag("Kiosk", "project"), tag("Finance", "topic")] }),
    note({ id: "n2", title: "Finance export", createdAt: "2026-05-23T09:00:00.000Z", tags: [tag("Finance", "topic")] }),
  ];

  it("renders tag rows and the top tag's notes by default", () => {
    render(TagsView, { props: { notes } });

    expect(screen.getByRole("heading", { name: "Tags" })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Select Finance/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Select Kiosk/ })).toBeTruthy();
    // Finance has 2 notes -> sorts first -> selected by default
    expect(screen.getByRole("heading", { name: "Finance" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open note: Finance export" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open note: Kiosk telemetry" })).toBeTruthy();
  });

  it("filters the tag list by search text", async () => {
    render(TagsView, { props: { notes } });

    await fireEvent.input(screen.getByRole("textbox", { name: "Search tags" }), {
      target: { value: "kiosk" },
    });

    expect(screen.queryByRole("button", { name: /Select Finance/ })).toBeNull();
    expect(screen.getByRole("button", { name: /Select Kiosk/ })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Kiosk" })).toBeTruthy();
  });

  it("updates the detail pane when a tag is selected", async () => {
    render(TagsView, { props: { notes } });

    await fireEvent.click(screen.getByRole("button", { name: /Select Kiosk/ }));

    expect(screen.getByRole("heading", { name: "Kiosk" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open note: Kiosk telemetry" })).toBeTruthy();
  });

  it("dispatches openNote when a note row is clicked", async () => {
    const openNote = vi.fn();
    render(TagsView, { props: { notes }, events: { openNote } });

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Finance export" }));

    expect(openNote.mock.calls.map((call) => call[0].detail)).toEqual(["n2"]);
  });

  it("shows an empty state when there are no tags", () => {
    render(TagsView, { props: { notes: [note({ id: "n0", tags: [] })] } });
    expect(screen.getByText("No tags yet")).toBeTruthy();
  });

  it("shows co-occurring tags and jumps to one when clicked", async () => {
    render(TagsView, { props: { notes } });

    // Finance selected by default; Kiosk co-occurs with it on n1.
    const relatedKiosk = screen.getByRole("button", { name: "Show tag: Kiosk" });
    await fireEvent.click(relatedKiosk);

    expect(screen.getByRole("heading", { name: "Kiosk" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open note: Kiosk telemetry" })).toBeTruthy();
  });

  it("renders a summary preview in the note rows", () => {
    const withSummary = [
      note({
        id: "s1",
        title: "Pricing export mismatch",
        summary: "Finance sees different CSV totals.",
        parseStatus: "failed",
        tags: [tag("Finance", "topic")],
      }),
    ];
    render(TagsView, { props: { notes: withSummary } });

    expect(screen.getByText("Finance sees different CSV totals.")).toBeTruthy();
  });
});
