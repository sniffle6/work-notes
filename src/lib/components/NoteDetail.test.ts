// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";
import { tick } from "svelte";

import type { NoteDetail as NoteDetailType } from "$lib/types";
import NoteDetail from "./NoteDetail.svelte";

afterEach(() => cleanup());

describe("NoteDetail", () => {
  it("renders cleaned markdown instead of plain markdown text", () => {
    render(NoteDetail, {
      props: {
        note: {
          ...noteDetail(),
          parseStatus: "parsed",
          cleanedText: "## Root cause\n\n- SAS ticket-in is not published.\n- `TransactionEvent` is missing.",
          parseError: null,
        },
      },
    });

    expect(screen.getByRole("heading", { name: "Root cause" })).toBeTruthy();
    expect(screen.getByText("SAS ticket-in is not published.")).toBeTruthy();
    expect(screen.getByText("TransactionEvent")).toBeTruthy();
  });

  it("opens a feedback dialog from the reparse action", async () => {
    render(NoteDetail, {
      props: { note: noteDetail() },
    });

    expect(screen.queryByRole("dialog", { name: "Reparse with feedback" })).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Reparse with feedback" }));

    const dialog = screen.getByRole("dialog", { name: "Reparse with feedback" });
    expect(within(dialog).getByLabelText("Feedback")).toBeTruthy();
    expect(within(dialog).getByRole<HTMLButtonElement>("button", { name: "Send feedback" }).disabled).toBe(true);
  });

  it("dispatches reparse feedback and archive actions for the selected note", async () => {
    const reparseWithFeedback = vi.fn();
    const deleteNote = vi.fn();

    render(NoteDetail, {
      props: { note: noteDetail() },
      events: { reparseWithFeedback, deleteNote },
    });

    await fireEvent.click(screen.getByRole("button", { name: /Reparse with feedback/ }));
    const feedback = screen.getByLabelText("Feedback");
    await fireEvent.input(feedback, {
      target: { value: "Tag this as research and make Robert the requester." },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Send feedback" }));
    await fireEvent.click(screen.getByRole("button", { name: "Archive" }));

    expect(reparseWithFeedback).toHaveBeenCalledTimes(1);
    expect(reparseWithFeedback.mock.calls[0][0].detail).toBe(
      "Tag this as research and make Robert the requester.",
    );
    expect(deleteNote).toHaveBeenCalledTimes(1);
  });

  it("shows restore and permanent delete actions for archived notes", async () => {
    const restoreNote = vi.fn();
    const permanentlyDeleteNote = vi.fn();

    render(NoteDetail, {
      props: { note: { ...noteDetail(), isArchived: true } },
      events: { restoreNote, permanentlyDeleteNote },
    });

    expect(screen.queryByRole("button", { name: "Archive" })).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Restore" }));
    await fireEvent.click(screen.getByRole("button", { name: "Delete permanently" }));

    expect(restoreNote).toHaveBeenCalledTimes(1);
    expect(permanentlyDeleteNote).toHaveBeenCalledTimes(1);
  });

  it("dispatches manual follow-up text and lane without clearing until done", async () => {
    const createFollowup = vi.fn();

    render(NoteDetail, {
      props: { note: noteDetail() },
      events: { createFollowup },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Add follow-up" }));

    const textInput = screen.getByLabelText<HTMLInputElement>("Follow-up text");
    const laneInput = screen.getByLabelText<HTMLInputElement>("Follow-up lane");

    await fireEvent.input(textInput, { target: { value: "  Send Robert the AI feasibility summary.  " } });
    await fireEvent.input(laneInput, { target: { value: "  Waiting  " } });
    await fireEvent.click(screen.getByRole("button", { name: "Create follow-up" }));

    expect(createFollowup).toHaveBeenCalledTimes(1);
    expect(createFollowup.mock.calls[0][0].detail.text).toBe("Send Robert the AI feasibility summary.");
    expect(createFollowup.mock.calls[0][0].detail.lane).toBe("Waiting");
    expect(textInput.value).toBe("  Send Robert the AI feasibility summary.  ");
    expect(laneInput.value).toBe("  Waiting  ");

    createFollowup.mock.calls[0][0].detail.done();
    await tick();

    expect(screen.queryByLabelText("Follow-up text")).toBeNull();
  });

  it("does not show manual follow-up creation for archived notes", () => {
    render(NoteDetail, {
      props: { note: { ...noteDetail(), isArchived: true } },
    });

    expect(screen.queryByRole("button", { name: "Add follow-up" })).toBeNull();
    expect(screen.queryByLabelText("Follow-up text")).toBeNull();
  });

  it("dispatches action accept and dismiss from suggested action rows", async () => {
    const acceptAction = vi.fn();
    const dismissAction = vi.fn();

    render(NoteDetail, {
      props: { note: noteDetailWithAction() },
      events: { acceptAction, dismissAction },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Accept action: Follow up with Robert" }));
    await fireEvent.click(screen.getByRole("button", { name: "Dismiss action: Follow up with Robert" }));

    expect(acceptAction.mock.calls[0][0].detail).toBe("action-1");
    expect(dismissAction.mock.calls[0][0].detail).toBe("action-1");
  });

  it("dispatches complete and reopen events for accepted and done actions", async () => {
    const completeAction = vi.fn();
    const reopenAction = vi.fn();

    render(NoteDetail, {
      props: { note: noteDetailWithAcceptedAndDoneActions() },
      events: { completeAction, reopenAction },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Complete action: Send the update" }));
    await fireEvent.click(screen.getByRole("button", { name: "Reopen action: Verify the fix" }));

    expect(completeAction.mock.calls[0][0].detail).toBe("action-accepted");
    expect(reopenAction.mock.calls[0][0].detail).toBe("action-done");
  });
});

function noteDetail(): NoteDetailType {
  return {
    id: "note-1",
    title: "Robert local AI",
    rawText: "Robert said check local AI feasibility.",
    cleanedText: null,
    summary: null,
    captureSource: "quick_capture",
    createdAt: "2026-05-20T13:00:00.000Z",
    updatedAt: "2026-05-20T13:00:00.000Z",
    parseStatus: "failed",
    reviewStatus: "none",
    isArchived: false,
    tags: [],
    actionItemCount: 0,
    suggestedActionItemCount: 0,
    actionItems: [],
    parseError: "Parser failed.",
  };
}

function noteDetailWithAction(): NoteDetailType {
  return {
    ...noteDetail(),
    parseStatus: "parsed",
    parseError: null,
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    actionItems: [
      {
        id: "action-1",
        noteId: "note-1",
        text: "Follow up with Robert",
        owner: "me",
        dueDate: null,
        status: "suggested",
        source: "parser",
        noteTitle: "Robert local AI",
      },
    ],
  };
}

function noteDetailWithAcceptedAndDoneActions(): NoteDetailType {
  return {
    ...noteDetail(),
    parseStatus: "parsed",
    parseError: null,
    actionItemCount: 2,
    suggestedActionItemCount: 0,
    actionItems: [
      {
        id: "action-accepted",
        noteId: "note-1",
        text: "Send the update",
        owner: "me",
        dueDate: null,
        status: "accepted",
        source: "parser",
        noteTitle: "Robert local AI",
      },
      {
        id: "action-done",
        noteId: "note-1",
        text: "Verify the fix",
        owner: "me",
        dueDate: null,
        status: "done",
        source: "parser",
        noteTitle: "Robert local AI",
      },
    ],
  };
}
