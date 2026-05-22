// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import AppShell from "./AppShell.svelte";

afterEach(() => cleanup());

describe("AppShell", () => {
  it("marks today active and emits today navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        activeView: "today",
      },
      events: { navigate },
    });

    const today = screen.getByRole("button", { name: "Today" });
    await fireEvent.click(today);

    expect(today.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("today");
  });

  it("marks archive active and emits archive navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        activeView: "archive",
      },
      events: { navigate },
    });

    const archive = screen.getByRole("button", { name: "Archive" });
    await fireEvent.click(archive);

    expect(archive.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("archive");
  });

  it("marks actions active and emits actions navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [
          { label: "Inbox", value: "3" },
          { label: "Needs review", value: "2" },
        ],
        activeView: "actions",
      },
      events: { navigate },
    });

    const actions = screen.getByRole("button", { name: "Actions" });
    await fireEvent.click(actions);

    expect(actions.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("actions");
  });
});
