// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem } from "$lib/types";
import ReviewQueue from "./ReviewQueue.svelte";

afterEach(() => cleanup());

describe("ReviewQueue", () => {
  it("renders suggested actions with note context and dispatches queue events", async () => {
    const select = vi.fn();
    const accept = vi.fn();
    const dismiss = vi.fn();

    render(ReviewQueue, {
      props: {
        actions: actions(),
        busyActionId: null,
        loading: false,
      },
      events: { select, accept, dismiss },
    });

    expect(screen.getByText("Kiosk 7 telemetry IDs")).toBeTruthy();
    expect(screen.getByText("Bring serial list into the Tuesday sync.")).toBeTruthy();
    expect(screen.getByText("Maya")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Open note: Kiosk 7 telemetry IDs" }));
    await fireEvent.click(screen.getByRole("button", { name: "Accept action: Bring serial list into the Tuesday sync." }));
    await fireEvent.click(screen.getByRole("button", { name: "Dismiss action: Bring serial list into the Tuesday sync." }));

    expect(select.mock.calls[0][0].detail).toBe("note-1");
    expect(accept.mock.calls[0][0].detail).toBe("action-1");
    expect(dismiss.mock.calls[0][0].detail).toBe("action-1");
  });

  it("collapses to its header, keeps the count visible, and remembers the choice", async () => {
    const props = { actions: actions(), busyActionId: null, loading: false };
    render(ReviewQueue, { props });
    const toggle = screen.getByRole("button", { name: "Suggested actions" });
    expect(toggle.getAttribute("aria-expanded")).toBe("true");

    await fireEvent.click(toggle);

    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByText("Bring serial list into the Tuesday sync.")).toBeNull();
    expect(screen.getByText("1 pending")).toBeTruthy();

    cleanup();
    render(ReviewQueue, { props });
    expect(screen.queryByText("Bring serial list into the Tuesday sync.")).toBeNull();
    localStorage.clear();
  });
});

function actions(): ActionReviewItem[] {
  return [
    {
      id: "action-1",
      noteId: "note-1",
      noteTitle: "Kiosk 7 telemetry IDs",
      text: "Bring serial list into the Tuesday sync.",
      owner: "Maya",
      dueDate: null,
      confidence: 0.82,
      createdAt: "2026-05-20T13:42:00.000Z",
    },
  ];
}
