import { describe, expect, it } from "vitest";

import { buildTagDetail, buildTags, matchesTagSearch } from "./tags";
import type { NoteListItem, Tag, TagKind } from "$lib/types";

function tag(name: string, kind: TagKind = "topic"): Tag {
  return { id: `tag-${kind}-${name}`, name, kind };
}

function note(id: string, createdAt: string, tags: Tag[]): NoteListItem {
  return {
    id,
    title: `Note ${id}`,
    rawText: "",
    cleanedText: null,
    summary: null,
    captureSource: "quick_capture",
    createdAt,
    updatedAt: createdAt,
    parseStatus: "parsed",
    reviewStatus: "none",
    isArchived: false,
    tags,
    actionItemCount: 0,
    suggestedActionItemCount: 0,
  };
}

describe("buildTags", () => {
  it("groups a tag across notes and counts distinct notes", () => {
    const tags = buildTags([
      note("n1", "2026-05-20T10:00:00.000Z", [tag("Finance")]),
      note("n2", "2026-05-21T10:00:00.000Z", [tag("Finance"), tag("Kiosk", "project")]),
    ]);

    const finance = tags.find((t) => t.name === "Finance");
    const kiosk = tags.find((t) => t.name === "Kiosk");
    expect(finance?.noteCount).toBe(2);
    expect(kiosk?.noteCount).toBe(1);
  });

  it("counts a note once even if it carries the same tag twice", () => {
    const tags = buildTags([
      note("n1", "2026-05-20T10:00:00.000Z", [
        { id: "a", name: "Alice", kind: "person", source: "ai" },
        { id: "b", name: "Alice", kind: "person", source: "user" },
      ]),
    ]);

    expect(tags).toHaveLength(1);
    expect(tags[0].noteCount).toBe(1);
  });

  it("merges names case-insensitively but keeps different kinds separate", () => {
    const tags = buildTags([
      note("n1", "2026-05-20T10:00:00.000Z", [tag("Finance", "topic")]),
      note("n2", "2026-05-21T10:00:00.000Z", [tag("finance", "topic")]),
      note("n3", "2026-05-22T10:00:00.000Z", [tag("Finance", "project")]),
    ]);

    const topic = tags.find((t) => t.kind === "topic");
    const project = tags.find((t) => t.kind === "project");
    expect(topic?.noteCount).toBe(2);
    expect(topic?.name).toBe("Finance"); // first-seen display name
    expect(project?.noteCount).toBe(1);
  });

  it("sorts by note count desc, then name asc", () => {
    const tags = buildTags([
      note("n1", "2026-05-20T10:00:00.000Z", [tag("Zebra"), tag("Apple")]),
      note("n2", "2026-05-21T10:00:00.000Z", [tag("Apple")]),
    ]);

    expect(tags.map((t) => t.name)).toEqual(["Apple", "Zebra"]);
  });

  it("tracks the most recent createdAt as lastUsedAt", () => {
    const tags = buildTags([
      note("n1", "2026-05-20T10:00:00.000Z", [tag("Finance")]),
      note("n2", "2026-05-25T10:00:00.000Z", [tag("Finance")]),
    ]);

    expect(tags[0].lastUsedAt).toBe("2026-05-25T10:00:00.000Z");
  });
});

describe("buildTagDetail", () => {
  const notes = [
    note("n1", "2026-05-20T10:00:00.000Z", [tag("Finance")]),
    note("n2", "2026-05-25T10:00:00.000Z", [tag("Finance")]),
    note("n3", "2026-05-22T10:00:00.000Z", [tag("Kiosk", "project")]),
  ];

  it("returns the tag and its notes newest-first", () => {
    const key = buildTags(notes).find((t) => t.name === "Finance")!.key;
    const detail = buildTagDetail(key, notes);

    expect(detail?.tag.name).toBe("Finance");
    expect(detail?.notes.map((n) => n.id)).toEqual(["n2", "n1"]);
  });

  it("returns null for an unknown key", () => {
    expect(buildTagDetail("topic:nope", notes)).toBeNull();
  });
});

describe("matchesTagSearch", () => {
  const finance = buildTags([note("n1", "2026-05-20T10:00:00.000Z", [tag("Finance")])])[0];

  it("matches everything on an empty query", () => {
    expect(matchesTagSearch(finance, "  ")).toBe(true);
  });

  it("matches on name case-insensitively", () => {
    expect(matchesTagSearch(finance, "fin")).toBe(true);
  });

  it("matches on kind", () => {
    expect(matchesTagSearch(finance, "topic")).toBe(true);
  });

  it("returns false when nothing matches", () => {
    expect(matchesTagSearch(finance, "zzz")).toBe(false);
  });
});
