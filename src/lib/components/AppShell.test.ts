// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import AppShell from "./AppShell.svelte";

afterEach(() => cleanup());

describe("AppShell", () => {
  it("marks calendar active and emits today navigation", async () => {
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

    const calendar = screen.getByRole("button", { name: "Calendar" });
    await fireEvent.click(calendar);

    expect(calendar.getAttribute("aria-current")).toBe("page");
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

  it("marks done active and emits done navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        activeView: "done",
      },
      events: { navigate },
    });

    const done = screen.getByRole("button", { name: "Done" });
    await fireEvent.click(done);

    expect(done.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("done");
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

  it("marks follow-ups active and emits followups navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [
          { label: "Inbox", value: "3" },
          { label: "Needs review", value: "2" },
          { label: "Follow-ups", value: "4" },
        ],
        activeView: "followups",
      },
      events: { navigate },
    });

    const followups = screen.getByRole("button", { name: "Follow-ups" });
    await fireEvent.click(followups);

    expect(followups.getAttribute("aria-current")).toBe("page");
    expect(followups.textContent).toContain("4");
    expect(navigate.mock.calls[0][0].detail).toBe("followups");
  });

  it("marks people active and emits people navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        activeView: "people",
      },
      events: { navigate },
    });

    const people = screen.getByRole("button", { name: "People" });
    await fireEvent.click(people);

    expect(people.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("people");
  });

  it("marks tags active and emits tags navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        activeView: "tags",
      },
      events: { navigate },
    });

    const tags = screen.getByRole("button", { name: "Tags" });
    await fireEvent.click(tags);

    expect(tags.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("tags");
  });

  it("uses nav icons without fake keyboard hints", () => {
    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [
          { label: "Inbox", value: "1" },
          { label: "Needs review", value: "0" },
          { label: "Follow-ups", value: "0" },
        ],
      },
    });

    const primaryNav = screen.getByRole("navigation", { name: "Primary" });

    expect(primaryNav.querySelectorAll("svg")).toHaveLength(8);
    expect(primaryNav.querySelector("kbd")).toBeNull();
  });

  it("makes active parser work obvious in the sidebar footer", () => {
    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        parserQueueCount: 2,
      },
    });

    expect(screen.getByRole("status", { name: "Parser activity" }).textContent).toContain("Parsing notes");
    expect(screen.getByText("2 in queue")).toBeTruthy();
  });
});
