// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import AppShell from "./AppShell.svelte";

afterEach(() => {
  cleanup();
  localStorage.clear();
});

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

  it("marks jira active and emits jira navigation", async () => {
    const navigate = vi.fn();

    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
        activeView: "jira",
      },
      events: { navigate },
    });

    const jira = screen.getByRole("button", { name: "Jira" });
    await fireEvent.click(jira);

    expect(jira.getAttribute("aria-current")).toBe("page");
    expect(navigate.mock.calls[0][0].detail).toBe("jira");
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

    expect(primaryNav.querySelectorAll("svg")).toHaveLength(9);
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

  it("collapses the sidebar to icons, keeps navigation working, and remembers the choice", async () => {
    const navigate = vi.fn();
    const props = {
      title: "Work Notes",
      subtitle: "Fast capture",
      workspace: "Local workspace",
      metrics: [{ label: "Inbox", value: "1" }],
    };
    render(AppShell, { props, events: { navigate } });
    const sidebar = screen.getByRole("complementary", { name: "Workspace" });
    expect(sidebar.classList.contains("collapsed")).toBe(false);

    await fireEvent.click(screen.getByRole("button", { name: "Collapse sidebar" }));

    expect(sidebar.classList.contains("collapsed")).toBe(true);
    expect(screen.getByRole("button", { name: "Expand sidebar" }).getAttribute("aria-expanded")).toBe("false");
    await fireEvent.click(screen.getByRole("button", { name: "Jira" }));
    expect(navigate.mock.calls[0][0].detail).toBe("jira");

    cleanup();
    render(AppShell, { props });
    expect(screen.getByRole("complementary", { name: "Workspace" }).classList.contains("collapsed")).toBe(true);
  });

  it("toggles the sidebar with Ctrl+B", async () => {
    render(AppShell, {
      props: {
        title: "Work Notes",
        subtitle: "Fast capture",
        workspace: "Local workspace",
        metrics: [{ label: "Inbox", value: "1" }],
      },
    });
    const sidebar = screen.getByRole("complementary", { name: "Workspace" });

    await fireEvent.keyDown(window, { key: "b", ctrlKey: true });
    expect(sidebar.classList.contains("collapsed")).toBe(true);

    await fireEvent.keyDown(window, { key: "b", ctrlKey: true });
    expect(sidebar.classList.contains("collapsed")).toBe(false);
  });
});
