import { describe, expect, it } from "vitest";

import type { NoteListItem } from "$lib/types";
import { createInboxFilters, matchesNoteFilters } from "./filters";

function note(overrides: Partial<NoteListItem> = {}): NoteListItem {
  return {
    id: "note-1",
    title: "Pricing export mismatch",
    rawText: "Jordan said finance sees a different CSV total.",
    summary: "Finance needs the filtered query checked.",
    captureSource: "quick_capture",
    createdAt: "2026-05-20T13:00:00.000Z",
    updatedAt: "2026-05-20T13:00:00.000Z",
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    tags: [
      {
        id: "tag-1",
        name: "Finance",
        kind: "topic",
        source: "ai",
        confidence: 0.92,
      },
    ],
    actionItemCount: 1,
    suggestedActionItemCount: 1,
    ...overrides,
  };
}

describe("createInboxFilters", () => {
  it("creates empty filters that do not exclude notes", () => {
    const filters = createInboxFilters();

    expect(filters).toEqual({
      search: "",
      tagIds: [],
      parseStatuses: [],
      reviewStatuses: [],
    });
    expect(matchesNoteFilters(note(), filters)).toBe(true);
  });
});

describe("matchesNoteFilters", () => {
  it("matches notes by case-insensitive text across title, raw text, and summary", () => {
    const filters = createInboxFilters({ search: "FILTERED query" });

    expect(matchesNoteFilters(note(), filters)).toBe(true);
    expect(matchesNoteFilters(note({ summary: "No related wording." }), filters)).toBe(false);
  });

  it("requires at least one matching selected tag", () => {
    const filters = createInboxFilters({ tagIds: ["tag-2", "tag-3"] });

    expect(
      matchesNoteFilters(
        note({
          tags: [
            { id: "tag-2", name: "Maya", kind: "person", source: "user", confidence: 1 },
          ],
        }),
        filters,
      ),
    ).toBe(true);
    expect(matchesNoteFilters(note(), filters)).toBe(false);
  });

  it("matches notes by selected parse statuses", () => {
    const filters = createInboxFilters({ parseStatuses: ["failed", "queued"] });

    expect(matchesNoteFilters(note({ parseStatus: "failed" }), filters)).toBe(true);
    expect(matchesNoteFilters(note({ parseStatus: "parsed" }), filters)).toBe(false);
  });

  it("matches notes by selected review statuses", () => {
    const filters = createInboxFilters({ reviewStatuses: ["needs_review"] });

    expect(matchesNoteFilters(note({ reviewStatus: "needs_review" }), filters)).toBe(true);
    expect(matchesNoteFilters(note({ reviewStatus: "reviewed" }), filters)).toBe(false);
  });

  it("combines text, tag, parse status, and review status filters", () => {
    const filters = createInboxFilters({
      search: "pricing",
      tagIds: ["tag-1"],
      parseStatuses: ["parsed"],
      reviewStatuses: ["needs_review"],
    });

    expect(matchesNoteFilters(note(), filters)).toBe(true);
    expect(matchesNoteFilters(note({ parseStatus: "failed" }), filters)).toBe(false);
  });
});
