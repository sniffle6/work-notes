// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { InboxFilters, NoteListItem } from "$lib/types";
import InboxList from "./InboxList.svelte";

afterEach(() => cleanup());

describe("InboxList", () => {
  it("uses the v3 quick filters and action mode", async () => {
    const filter = vi.fn();

    render(InboxList, {
      props: {
        items: notes(),
        filters: filters(),
        selectedId: "n1",
      },
      events: { filter },
    });

    await fireEvent.click(screen.getByRole("button", { name: /Needs review/ }));
    expect(filter.mock.calls[0][0].detail).toEqual({ reviewStatuses: ["needs_review"], parseStatuses: [] });

    await fireEvent.click(screen.getByRole("button", { name: /Actions/ }));
    expect(screen.getByText("2 suggested actions")).toBeTruthy();
  });

  it("uses singular action labels for one suggested action", async () => {
    render(InboxList, {
      props: {
        items: [{ ...notes()[0], actionItemCount: 1, suggestedActionItemCount: 1 }],
        filters: filters(),
        selectedId: "n1",
      },
    });

    expect(screen.getByText("1 action")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: /Actions/ }));

    expect(screen.getByText("1 suggested action")).toBeTruthy();
  });

  it("uses archive empty copy in archive mode", () => {
    render(InboxList, {
      props: {
        items: [],
        filters: { ...filters(), includeArchived: true },
        selectedId: undefined,
        viewMode: "archive",
      },
    });

    expect(screen.getByText("No archived notes")).toBeTruthy();
  });
});

function filters(): InboxFilters {
  return {
    search: "",
    tagIds: [],
    parseStatuses: [],
    reviewStatuses: [],
    includeArchived: false,
  };
}

function notes(): NoteListItem[] {
  return [
    {
      id: "n1",
      title: "Maria dashboard owner",
      rawText: "Maria needs dashboard owner by Friday.",
      cleanedText: "Maria needs a dashboard owner by Friday.",
      summary: "Dashboard owner decision needed.",
      captureSource: "quick_capture",
      createdAt: "2026-05-21T15:00:00.000Z",
      updatedAt: "2026-05-21T15:00:00.000Z",
      parseStatus: "parsed",
      reviewStatus: "needs_review",
      tags: [{ id: "t1", name: "Maria", kind: "person" }],
      actionItemCount: 2,
      suggestedActionItemCount: 2,
    },
  ];
}
