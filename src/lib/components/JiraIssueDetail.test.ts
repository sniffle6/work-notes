// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { JiraIssueDetail as JiraIssue } from "$lib/types";
import JiraIssueDetail from "./JiraIssueDetail.svelte";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function issue(overrides: Partial<JiraIssue> = {}): JiraIssue {
  return {
    key: "AB-7",
    summary: "Printer jams on reboot",
    status: "To Do",
    statusCategory: "new",
    issueType: "Bug",
    priority: "High",
    assignee: null,
    updated: "2026-11-01T10:00:00.000Z",
    labels: ["printer"],
    url: "https://example.atlassian.net/browse/AB-7",
    description: "## Context\n\nJams after reboot.",
    descriptionEditableLosslessly: true,
    reporter: "Rina Q",
    parentKey: null,
    subtasks: [],
    linkedIssues: [],
    comments: [{ id: "c1", author: "Maya Lopez", created: "2026-11-02T09:00:00.000Z", body: "Reproduced on bench 1." }],
    transitions: [
      { id: "11", name: "Start work", toStatus: "In Progress" },
      { id: "21", name: "Dev Complete", toStatus: "Dev Complete" },
    ],
    ...overrides,
  };
}

describe("JiraIssueDetail", () => {
  it("prompts for a selection when no ticket is open", () => {
    render(JiraIssueDetail, { props: { issue: null } });

    expect(screen.getByText("Select a ticket")).toBeTruthy();
  });

  it("shows the ticket with a link to Jira, its description, and comments", () => {
    render(JiraIssueDetail, { props: { issue: issue() } });

    const link = screen.getByRole("link", { name: "AB-7" });
    expect(link.getAttribute("href")).toBe("https://example.atlassian.net/browse/AB-7");
    expect(screen.getByRole("heading", { name: "Printer jams on reboot" })).toBeTruthy();
    expect(screen.getByText("To Do")).toBeTruthy();
    expect(screen.getByText("Unassigned")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Context" })).toBeTruthy();
    expect(screen.getByText("Reproduced on bench 1.")).toBeTruthy();
    expect(screen.getByText("Maya Lopez")).toBeTruthy();
  });

  it("offers each available transition and dispatches its id", async () => {
    const transition = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { transition } });

    await fireEvent.click(screen.getByRole("button", { name: "Move to Dev Complete" }));

    expect(screen.getByRole("button", { name: "Move to In Progress" })).toBeTruthy();
    expect(transition.mock.calls[0][0].detail).toBe("21");
  });

  it("copies a prompt for the open ticket and confirms it", async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
    render(JiraIssueDetail, { props: { issue: issue() } });

    await fireEvent.click(screen.getByRole("button", { name: "Copy agent prompt" }));

    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy());
    const prompt = (writeText.mock.calls[0] as unknown as [string])[0];
    expect(prompt).toContain("Work on Jira ticket AB-7: Printer jams on reboot");
    expect(prompt).toContain("Jams after reboot.");
    expect(prompt).toContain("Reproduced on bench 1.");
  });

  it("says so when the prompt cannot be copied", async () => {
    vi.stubGlobal("navigator", {
      ...navigator,
      clipboard: {
        writeText: vi.fn(async () => {
          throw new Error("denied");
        }),
      },
    });
    render(JiraIssueDetail, { props: { issue: issue() } });

    await fireEvent.click(screen.getByRole("button", { name: "Copy agent prompt" }));

    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Copy failed" })).toBeTruthy());
  });

  it("disables ticket changes while a change is being saved", () => {
    render(JiraIssueDetail, { props: { issue: issue(), saving: true } });

    expect((screen.getByRole("button", { name: "Move to In Progress" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Assign to me" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Edit" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("dispatches assign to me", async () => {
    const assignToMe = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { assignToMe } });

    await fireEvent.click(screen.getByRole("button", { name: "Assign to me" }));

    expect(assignToMe).toHaveBeenCalledTimes(1);
  });

  it("sends a trimmed comment and clears the box once it is saved", async () => {
    const comment = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { comment } });
    const box = screen.getByRole("textbox", { name: "Comment" }) as HTMLTextAreaElement;
    const add = screen.getByRole("button", { name: "Add comment" }) as HTMLButtonElement;

    expect(add.disabled).toBe(true);
    await fireEvent.input(box, { target: { value: "  Verified on bench  " } });
    await fireEvent.click(add);

    const detail = comment.mock.calls[0][0].detail;
    expect(detail.text).toBe("Verified on bench");
    expect(box.value).toBe("  Verified on bench  ");
    detail.done();
    await vi.waitFor(() => expect(box.value).toBe(""));
  });

  it("saves only the fields that changed", async () => {
    const saveEdit = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { saveEdit } });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const save = screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    await fireEvent.input(screen.getByRole("textbox", { name: "Summary" }), {
      target: { value: "Printer jams after reboot" },
    });
    await fireEvent.click(save);

    const detail = saveEdit.mock.calls[0][0].detail;
    expect(detail.edit).toEqual({ summary: "Printer jams after reboot" });
    detail.done();
    await vi.waitFor(() => expect(screen.queryByRole("textbox", { name: "Summary" })).toBeNull());
  });

  it("requires confirmation before a description edit flattens Jira formatting", async () => {
    const saveEdit = vi.fn();
    render(JiraIssueDetail, {
      props: { issue: issue({ descriptionEditableLosslessly: false }) },
      events: { saveEdit },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    expect(screen.queryByRole("checkbox")).toBeNull();
    await fireEvent.input(screen.getByRole("textbox", { name: "Description" }), {
      target: { value: "Rewritten body" },
    });
    const save = screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    await fireEvent.click(screen.getByRole("checkbox", { name: /Replace the formatted description/ }));
    expect(save.disabled).toBe(false);
    await fireEvent.click(save);

    expect(saveEdit.mock.calls[0][0].detail.edit).toEqual({ description: "Rewritten body" });
  });

  it("cancels an edit without dispatching", async () => {
    const saveEdit = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { saveEdit } });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await fireEvent.input(screen.getByRole("textbox", { name: "Summary" }), { target: { value: "Changed" } });
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(saveEdit).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Printer jams on reboot" })).toBeTruthy();
  });
});
