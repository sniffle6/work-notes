// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { NoteDetail as NoteDetailType } from "$lib/types";
import NoteDetail from "./NoteDetail.svelte";

afterEach(() => cleanup());

describe("NoteDetail", () => {
  it("dispatches reparse feedback and delete actions for the selected note", async () => {
    const reparseWithFeedback = vi.fn();
    const deleteNote = vi.fn();

    render(NoteDetail, {
      props: { note: noteDetail() },
      events: { reparseWithFeedback, deleteNote },
    });

    const feedback = screen.getByLabelText("Reparse feedback");
    await fireEvent.input(feedback, {
      target: { value: "Tag this as research and make Robert the requester." },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Reparse with feedback" }));
    await fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    expect(reparseWithFeedback).toHaveBeenCalledTimes(1);
    expect(reparseWithFeedback.mock.calls[0][0].detail).toBe(
      "Tag this as research and make Robert the requester.",
    );
    expect(deleteNote).toHaveBeenCalledTimes(1);
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
