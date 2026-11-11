// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";

import type { JiraChatMessage, JiraChatThread } from "$lib/types";
import JiraChat from "./JiraChat.svelte";

afterEach(() => cleanup());

function message(overrides: Partial<JiraChatMessage> = {}): JiraChatMessage {
  return {
    id: "message-1",
    threadId: "thread-1",
    role: "assistant",
    content: "Two tickets are open.",
    status: null,
    result: null,
    createdAt: "2026-11-10T09:00:00.000Z",
    ...overrides,
  };
}

function thread(overrides: Partial<JiraChatThread> = {}): JiraChatThread {
  return {
    id: "thread-1",
    title: "Printer tickets",
    createdAt: "2026-11-10T09:00:00.000Z",
    updatedAt: "2026-11-10T09:00:00.000Z",
    ...overrides,
  };
}

const baseProps = { threads: [thread()], activeThreadId: "thread-1", messages: [] as JiraChatMessage[] };

describe("JiraChat", () => {
  it("sends the trimmed message on Enter and clears the box", async () => {
    const send = vi.fn();
    render(JiraChat, { props: baseProps, events: { send } });
    const box = screen.getByRole("textbox", { name: "Message the Jira agent" }) as HTMLTextAreaElement;

    await fireEvent.input(box, { target: { value: "  what is open?  " } });
    await fireEvent.keyDown(box, { key: "Enter" });

    expect(send.mock.calls[0][0].detail).toBe("what is open?");
    expect(box.value).toBe("");
  });

  it("keeps typing on Shift+Enter and ignores blank messages", async () => {
    const send = vi.fn();
    render(JiraChat, { props: baseProps, events: { send } });
    const box = screen.getByRole("textbox", { name: "Message the Jira agent" }) as HTMLTextAreaElement;

    await fireEvent.keyDown(box, { key: "Enter" });
    await fireEvent.input(box, { target: { value: "line one" } });
    await fireEvent.keyDown(box, { key: "Enter", shiftKey: true });

    expect(send).not.toHaveBeenCalled();
    expect(box.value).toBe("line one");
  });

  it("blocks sending and shows progress while the agent works", async () => {
    const send = vi.fn();
    render(JiraChat, { props: { ...baseProps, busy: true }, events: { send } });
    const box = screen.getByRole("textbox", { name: "Message the Jira agent" }) as HTMLTextAreaElement;

    await fireEvent.input(box, { target: { value: "hello" } });
    await fireEvent.keyDown(box, { key: "Enter" });

    expect(send).not.toHaveBeenCalled();
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Agent is working")).toBeTruthy();
  });

  it("renders the conversation with lookups collapsed", () => {
    render(JiraChat, {
      props: {
        ...baseProps,
        messages: [
          message({ id: "m1", role: "user", content: "what is open?" }),
          message({ id: "m2", role: "tool", content: "Search: project = AB", status: "ok", result: "AB-7 | To Do" }),
          message({ id: "m3" }),
        ],
      },
    });

    expect(screen.getByText("what is open?")).toBeTruthy();
    expect(screen.getByText("Search: project = AB")).toBeTruthy();
    expect(screen.getByText("AB-7 | To Do")).toBeTruthy();
    expect(screen.getByText("Two tickets are open.")).toBeTruthy();
  });

  it("asks for approval before a proposed change and dispatches the decision", async () => {
    const resolve = vi.fn();
    render(JiraChat, {
      props: {
        ...baseProps,
        messages: [message({ id: "p1", role: "proposal", content: 'Move AB-7 to "Done"', status: "pending" })],
      },
      events: { resolve },
    });
    const card = screen.getByRole("group", { name: "Proposed change" });

    expect(within(card).getByText('Move AB-7 to "Done"')).toBeTruthy();
    await fireEvent.click(within(card).getByRole("button", { name: "Approve" }));
    await fireEvent.click(within(card).getByRole("button", { name: "Reject" }));

    expect(resolve.mock.calls[0][0].detail).toEqual({ messageId: "p1", approve: true });
    expect(resolve.mock.calls[1][0].detail).toEqual({ messageId: "p1", approve: false });
  });

  it("disables the decision buttons while a proposal is being applied", () => {
    render(JiraChat, {
      props: {
        ...baseProps,
        busyProposalId: "p1",
        messages: [message({ id: "p1", role: "proposal", content: "Unassign AB-7", status: "pending" })],
      },
    });

    expect((screen.getByRole("button", { name: "Approve" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Reject" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("shows the outcome of resolved proposals without decision buttons", () => {
    render(JiraChat, {
      props: {
        ...baseProps,
        messages: [
          message({ id: "p1", role: "proposal", content: "Comment on AB-7", status: "approved", result: "Comment added to AB-7." }),
          message({ id: "p2", role: "proposal", content: "Unassign AB-7", status: "rejected" }),
          message({ id: "p3", role: "proposal", content: "Move AB-8", status: "failed", result: "Jira rejected the credentials" }),
        ],
      },
    });

    expect(screen.queryByRole("button", { name: "Approve" })).toBeNull();
    expect(screen.getByText("Applied")).toBeTruthy();
    expect(screen.getByText("Comment added to AB-7.")).toBeTruthy();
    expect(screen.getByText("Rejected")).toBeTruthy();
    expect(screen.getByText("Failed")).toBeTruthy();
    expect(screen.getByText("Jira rejected the credentials")).toBeTruthy();
  });

  it("switches, starts, and deletes conversations", async () => {
    const openThread = vi.fn();
    const newThread = vi.fn();
    const deleteThread = vi.fn();
    render(JiraChat, {
      props: { ...baseProps, threads: [thread(), thread({ id: "thread-2", title: "Bill acceptor" })] },
      events: { openThread, newThread, deleteThread },
    });

    await fireEvent.change(screen.getByRole("combobox", { name: "Conversation" }), {
      target: { value: "thread-2" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "New chat" }));
    await fireEvent.click(screen.getByRole("button", { name: "Delete conversation" }));

    expect(openThread.mock.calls[0][0].detail).toBe("thread-2");
    expect(newThread).toHaveBeenCalledTimes(1);
    expect(deleteThread.mock.calls[0][0].detail).toBe("thread-1");
  });

  it("asks to be closed from its header", async () => {
    const close = vi.fn();
    render(JiraChat, { props: baseProps, events: { close } });

    await fireEvent.click(screen.getByRole("button", { name: "Hide agent" }));

    expect(close).toHaveBeenCalledTimes(1);
  });

  it("puts a suggested request in the box without sending it", async () => {
    const send = vi.fn();
    render(JiraChat, { props: baseProps, events: { send } });

    await fireEvent.click(screen.getByRole("button", { name: "Show me the important tickets I need to take care of today" }));

    const box = screen.getByRole("textbox", { name: "Message the Jira agent" }) as HTMLTextAreaElement;
    expect(box.value).toBe("Show me the important tickets I need to take care of today");
    expect(send).not.toHaveBeenCalled();
  });

  it("shows agent errors and the open ticket used as context", () => {
    render(JiraChat, {
      props: { ...baseProps, error: "codex command was not found: codex.cmd", focusIssueKey: "AB-7" },
    });

    expect(screen.getByRole("alert").textContent).toContain("codex command was not found");
    expect(screen.getByText(/AB-7/)).toBeTruthy();
  });
});
