// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";

import { createJiraStore, type JiraApi } from "$lib/stores/jira";
import type { JiraIssueDetail, JiraIssueSummary, JiraStatus } from "$lib/types";
import JiraView from "./JiraView.svelte";

afterEach(() => {
  cleanup();
  localStorage.clear();
});

function status(overrides: Partial<JiraStatus> = {}): JiraStatus {
  return {
    configured: true,
    missing: [],
    siteUrl: "https://example.atlassian.net",
    projectKey: "AB",
    filters: [
      { name: "My open tickets", jql: "assignee = currentUser()" },
      { name: "All open", jql: "project = AB" },
    ],
    ...overrides,
  };
}

function issue(overrides: Partial<JiraIssueSummary> = {}): JiraIssueSummary {
  return {
    key: "AB-7",
    summary: "Printer jams on reboot",
    status: "To Do",
    statusCategory: "new",
    issueType: "Bug",
    priority: "High",
    assignee: null,
    updated: null,
    labels: [],
    url: "https://example.atlassian.net/browse/AB-7",
    ...overrides,
  };
}

function detail(overrides: Partial<JiraIssueDetail> = {}): JiraIssueDetail {
  return {
    ...issue(),
    description: "Jams after reboot.",
    descriptionEditableLosslessly: true,
    reporter: null,
    parentKey: null,
    subtasks: [],
    linkedIssues: [],
    comments: [],
    transitions: [{ id: "11", name: "Start work", toStatus: "In Progress" }],
    ...overrides,
  };
}

function fakeApi(overrides: Partial<JiraApi> = {}): JiraApi {
  return {
    getJiraStatus: vi.fn(async () => status()),
    listJiraIssues: vi.fn(async () => [issue(), issue({ key: "AB-8", summary: "Bill acceptor rejects notes" })]),
    getJiraIssue: vi.fn(async (key: string) =>
      detail({ key, summary: key === "AB-8" ? "Bill acceptor rejects notes" : "Printer jams on reboot" }),
    ),
    listJiraIssueTypes: vi.fn(async () => ["Bug", "Task"]),
    createJiraIssue: vi.fn(async () => detail({ key: "AB-9", summary: "New ticket" })),
    updateJiraIssue: vi.fn(async (key: string) => detail({ key })),
    transitionJiraIssue: vi.fn(async (key: string) => detail({ key, status: "In Progress" })),
    addJiraComment: vi.fn(async (key: string) => detail({ key })),
    assignJiraIssueToMe: vi.fn(async (key: string) => detail({ key })),
    listJiraChatThreads: vi.fn(async () => []),
    createJiraChatThread: vi.fn(async () => ({ id: "thread-1", title: "New chat", createdAt: "", updatedAt: "" })),
    deleteJiraChatThread: vi.fn(async () => undefined),
    listJiraChatMessages: vi.fn(async () => []),
    sendJiraChatMessage: vi.fn(async () => []),
    resolveJiraChatProposal: vi.fn(async () => []),
    ...overrides,
  };
}

async function renderOpened(api: JiraApi, events: Record<string, (event: CustomEvent) => void> = {}) {
  const jira = createJiraStore(api);
  await jira.open();
  render(JiraView, { props: { jira }, events });
  return jira;
}

describe("JiraView", () => {
  it("lists the missing setup steps and links to settings when Jira is not configured", async () => {
    const openSettings = vi.fn();
    await renderOpened(
      fakeApi({
        getJiraStatus: vi.fn(async () =>
          status({
            configured: false,
            missing: ["Enter your Jira site URL in Settings > Jira.", "Set the JIRA_API_TOKEN environment variable."],
            filters: [],
          }),
        ),
      }),
      { openSettings },
    );

    expect(screen.getByText("Enter your Jira site URL in Settings > Jira.")).toBeTruthy();
    expect(screen.getByText("Set the JIRA_API_TOKEN environment variable.")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    expect(openSettings).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "New ticket" })).toBeNull();
  });

  it("lists tickets for the active filter and opens the one that is clicked", async () => {
    const api = fakeApi();
    await renderOpened(api);

    expect(screen.getByRole("button", { name: "My open tickets" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("heading", { name: "Printer jams on reboot" })).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "AB-8: Bill acceptor rejects notes" }));

    await vi.waitFor(() => expect(screen.getByRole("heading", { name: "Bill acceptor rejects notes" })).toBeTruthy());
    expect(api.getJiraIssue).toHaveBeenLastCalledWith("AB-8");
  });

  it("switches filters and refreshes on demand", async () => {
    const api = fakeApi();
    await renderOpened(api);

    await fireEvent.click(screen.getByRole("button", { name: "All open" }));
    await vi.waitFor(() => expect(api.listJiraIssues).toHaveBeenLastCalledWith("project = AB"));
    await vi.waitFor(() =>
      expect(screen.getByRole("button", { name: "All open" }).getAttribute("aria-pressed")).toBe("true"),
    );

    await fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await vi.waitFor(() => expect(api.listJiraIssues).toHaveBeenCalledTimes(3));
  });

  it("says so when a filter has no tickets", async () => {
    await renderOpened(fakeApi({ listJiraIssues: vi.fn(async () => []) }));

    expect(screen.getByText("No tickets match this filter")).toBeTruthy();
  });

  it("creates a ticket from the new-ticket form", async () => {
    const api = fakeApi();
    await renderOpened(api);

    await fireEvent.click(screen.getByRole("button", { name: "New ticket" }));
    const summary = await screen.findByRole("textbox", { name: "Summary" });
    await fireEvent.input(summary, { target: { value: "Meter export is empty" } });
    await fireEvent.change(screen.getByRole("combobox", { name: "Issue type" }), { target: { value: "Task" } });
    await fireEvent.input(screen.getByRole("textbox", { name: "Description" }), { target: { value: "Steps" } });
    await fireEvent.click(screen.getByRole("button", { name: "Create ticket" }));

    await vi.waitFor(() =>
      expect(api.createJiraIssue).toHaveBeenCalledWith({
        issueType: "Task",
        summary: "Meter export is empty",
        description: "Steps",
        noteId: null,
      }),
    );
    await vi.waitFor(() => expect(screen.queryByRole("textbox", { name: "Summary" })).toBeNull());
  });

  it("moves the open ticket through a transition", async () => {
    const api = fakeApi();
    await renderOpened(api);

    await fireEvent.click(screen.getByRole("button", { name: "Move to In Progress" }));

    await vi.waitFor(() => expect(api.transitionJiraIssue).toHaveBeenCalledWith("AB-7", "11"));
  });

  it("groups tickets under their status with a count", async () => {
    await renderOpened(
      fakeApi({
        listJiraIssues: vi.fn(async () => [
          issue({ key: "AB-1", summary: "In review", status: "In QA", statusCategory: "indeterminate" }),
          issue({ key: "AB-2", summary: "First to do" }),
          issue({ key: "AB-3", summary: "Second to do" }),
        ]),
      }),
    );

    const groups = screen.getAllByRole("group").filter((group) => group.classList.contains("issue-group"));
    expect(groups.map((group) => group.getAttribute("aria-label"))).toEqual(["To Do, 2 tickets", "In QA, 1 ticket"]);
    expect(within(groups[0]).getAllByRole("button").map((row) => row.getAttribute("aria-label"))).toEqual([
      "AB-2: First to do",
      "AB-3: Second to do",
    ]);
  });

  it("narrows the list as the search box is typed in", async () => {
    await renderOpened(fakeApi());

    await fireEvent.input(screen.getByRole("searchbox", { name: "Search tickets" }), {
      target: { value: "bill" },
    });

    expect(screen.queryByRole("button", { name: "AB-7: Printer jams on reboot" })).toBeNull();
    expect(screen.getByRole("button", { name: "AB-8: Bill acceptor rejects notes" })).toBeTruthy();
    expect(screen.getByText("1 of 2 tickets · AB")).toBeTruthy();

    await fireEvent.input(screen.getByRole("searchbox", { name: "Search tickets" }), {
      target: { value: "nothing like this" },
    });
    expect(screen.getByText("No tickets match the search")).toBeTruthy();
  });

  it("offers the agent's search as a filter that can be dismissed", async () => {
    const api = fakeApi();
    const jira = await renderOpened(api);

    await jira.applyViewAction({ kind: "showIssues", title: "Important today", jql: "priority = High" });

    const chip = await screen.findByRole("button", { name: "Important today" });
    expect(chip.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "My open tickets" }).getAttribute("aria-pressed")).toBe("false");

    await fireEvent.click(screen.getByRole("button", { name: "Remove Important today" }));

    await vi.waitFor(() => expect(screen.queryByRole("button", { name: "Important today" })).toBeNull());
    expect(screen.getByRole("button", { name: "My open tickets" }).getAttribute("aria-pressed")).toBe("true");
  });

  it("hides and restores the ticket list from its toggle and remembers the choice", async () => {
    const api = fakeApi();
    await renderOpened(api);
    expect(screen.getByRole("searchbox", { name: "Search tickets" })).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: "Hide ticket list" }));
    expect(screen.queryByRole("searchbox", { name: "Search tickets" })).toBeNull();
    expect(screen.getByRole("heading", { name: "Printer jams on reboot" })).toBeTruthy();

    cleanup();
    await renderOpened(api);
    expect(screen.queryByRole("searchbox", { name: "Search tickets" })).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Show ticket list" }));
    expect(screen.getByRole("searchbox", { name: "Search tickets" })).toBeTruthy();
  });

  it("toggles the ticket list with [ and the agent with ] unless a field has focus", async () => {
    await renderOpened(fakeApi());

    await fireEvent.keyDown(window, { key: "[" });
    expect(screen.queryByRole("searchbox", { name: "Search tickets" })).toBeNull();
    await fireEvent.keyDown(window, { key: "[" });
    const search = screen.getByRole("searchbox", { name: "Search tickets" });

    await fireEvent.keyDown(window, { key: "]" });
    expect(screen.getByRole("textbox", { name: "Message the Jira agent" })).toBeTruthy();

    await fireEvent.keyDown(search, { key: "]" });
    await fireEvent.keyDown(search, { key: "[" });
    expect(screen.getByRole("textbox", { name: "Message the Jira agent" })).toBeTruthy();
    expect(screen.getByRole("searchbox", { name: "Search tickets" })).toBeTruthy();
  });

  it("keeps the agent closed until asked for and remembers the choice", async () => {
    const api = fakeApi();
    await renderOpened(api);
    expect(screen.queryByRole("textbox", { name: "Message the Jira agent" })).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Agent" }));
    expect(screen.getByRole("textbox", { name: "Message the Jira agent" })).toBeTruthy();

    cleanup();
    await renderOpened(api);
    expect(screen.getByRole("textbox", { name: "Message the Jira agent" })).toBeTruthy();
  });

  it("sends agent messages with the open ticket as context and can hide the agent", async () => {
    const api = fakeApi();
    await renderOpened(api);
    await fireEvent.click(screen.getByRole("button", { name: "Agent" }));

    const box = screen.getByRole("textbox", { name: "Message the Jira agent" });
    await fireEvent.input(box, { target: { value: "summarize this" } });
    await fireEvent.keyDown(box, { key: "Enter" });
    await vi.waitFor(() =>
      expect(api.sendJiraChatMessage).toHaveBeenCalledWith("thread-1", "summarize this", "AB-7"),
    );

    await fireEvent.click(screen.getByRole("button", { name: "Agent" }));
    expect(screen.queryByRole("textbox", { name: "Message the Jira agent" })).toBeNull();
  });

  it("shows Jira errors", async () => {
    await renderOpened(
      fakeApi({
        listJiraIssues: vi.fn(async () => {
          throw new Error("Jira rejected the credentials");
        }),
      }),
    );

    expect(screen.getByRole("alert").textContent).toContain("Jira rejected the credentials");
  });
});
