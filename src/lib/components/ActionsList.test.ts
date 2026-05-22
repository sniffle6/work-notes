// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem } from "$lib/types";
import ActionsList from "./ActionsList.svelte";

afterEach(() => cleanup());

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Dashboard owner",
    text: "Decide whether to rebuild the dashboard",
    owner: "me",
    dueDate: "2026-05-22",
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

describe("ActionsList", () => {
  it("groups suggested actions and opens the source note", async () => {
    const select = vi.fn();
    const { container } = render(ActionsList, {
      props: {
        actions: [
          action({ id: "today", noteId: "note-today", text: "Send scope review", dueDate: "2026-05-22" }),
          action({ id: "later", noteId: "note-later", text: "Find design partner", dueDate: "2026-06-10" }),
        ],
        selectedNoteId: "note-today",
        now: new Date("2026-05-22T12:00:00.000Z"),
      },
      events: { select },
    });

    expect(screen.getByText("Today")).toBeTruthy();
    expect(screen.getByText("Later")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Find design partner" }));

    expect(select.mock.calls[0][0].detail).toBe("note-later");
    expect(container.querySelector(".action-row")?.getAttribute("role")).toBeNull();
    expect(
      screen.getByRole("button", { name: "Open note: Find design partner" }).classList.contains("row-button"),
    ).toBe(true);
  });

  it("filters actions by owner and dispatches lifecycle events", async () => {
    const accept = vi.fn();
    const dismiss = vi.fn();
    const select = vi.fn();
    render(ActionsList, {
      props: {
        actions: [
          action({ id: "keep", owner: "Maria", text: "Forward failing queries" }),
          action({ id: "hide", owner: "Sam", text: "Update new-hire deck" }),
        ],
        now: new Date("2026-05-22T12:00:00.000Z"),
      },
      events: { accept, dismiss, select },
    });

    await fireEvent.input(screen.getByRole("textbox", { name: "Search actions" }), {
      target: { value: "maria" },
    });

    expect(screen.getByText("Forward failing queries")).toBeTruthy();
    expect(screen.queryByText("Update new-hire deck")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Accept action: Forward failing queries" }));
    await fireEvent.click(screen.getByRole("button", { name: "Dismiss action: Forward failing queries" }));

    expect(accept.mock.calls[0][0].detail).toBe("keep");
    expect(dismiss.mock.calls[0][0].detail).toBe("keep");
    expect(select).not.toHaveBeenCalled();
  });

  it("filters actions by visible due labels", async () => {
    render(ActionsList, {
      props: {
        actions: [
          action({ id: "overdue", text: "Confirm stale invoice", dueDate: "2026-05-21" }),
          action({ id: "today", text: "Send launch summary", dueDate: "2026-05-22" }),
          action({ id: "tomorrow", text: "Prep renewal notes", dueDate: "2026-05-23" }),
        ],
        now: new Date("2026-05-22T12:00:00.000Z"),
      },
    });

    await fireEvent.input(screen.getByRole("textbox", { name: "Search actions" }), {
      target: { value: "today" },
    });

    expect(screen.getByText("Send launch summary")).toBeTruthy();
    expect(screen.queryByText("Confirm stale invoice")).toBeNull();
    expect(screen.queryByText("Prep renewal notes")).toBeNull();

    await fireEvent.input(screen.getByRole("textbox", { name: "Search actions" }), {
      target: { value: "1d overdue" },
    });

    expect(screen.getByText("Confirm stale invoice")).toBeTruthy();
    expect(screen.queryByText("Send launch summary")).toBeNull();

    await fireEvent.input(screen.getByRole("textbox", { name: "Search actions" }), {
      target: { value: "tomorrow" },
    });

    expect(screen.getByText("Prep renewal notes")).toBeTruthy();
    expect(screen.queryByText("Confirm stale invoice")).toBeNull();
  });

  it("renders the Actions header as static text", () => {
    render(ActionsList, {
      props: {
        actions: [],
      },
    });

    expect(screen.getByText("Actions")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Actions" })).toBeNull();
  });
});
