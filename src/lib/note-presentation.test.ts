import { describe, expect, it } from "vitest";

import { formatRelativeDate, noteStatusClass, noteStatusLabel } from "./note-presentation";

describe("noteStatusLabel", () => {
  it("prioritizes parse state, then review state, then captured", () => {
    expect(noteStatusLabel("failed", "none")).toBe("Parse failed");
    expect(noteStatusLabel("parsing", "none")).toBe("Parsing");
    expect(noteStatusLabel("queued", "none")).toBe("Queued");
    expect(noteStatusLabel("parsed", "needs_review")).toBe("Needs review");
    expect(noteStatusLabel("parsed", "reviewed")).toBe("Reviewed");
    expect(noteStatusLabel("parsed", "none")).toBe("Captured");
  });
});

describe("noteStatusClass", () => {
  it("maps status to a dot class", () => {
    expect(noteStatusClass("failed", "none")).toBe("error");
    expect(noteStatusClass("parsing", "none")).toBe("info");
    expect(noteStatusClass("queued", "none")).toBe("neutral");
    expect(noteStatusClass("parsed", "needs_review")).toBe("warning");
    expect(noteStatusClass("parsed", "reviewed")).toBe("success reviewed");
    expect(noteStatusClass("parsed", "none")).toBe("neutral");
  });
});

describe("formatRelativeDate", () => {
  const now = new Date("2026-05-25T12:00:00.000Z");

  it("returns 'never' for empty values", () => {
    expect(formatRelativeDate(null, now)).toBe("never");
    expect(formatRelativeDate("   ", now)).toBe("never");
  });

  it("passes through unparseable values", () => {
    expect(formatRelativeDate("not-a-date", now)).toBe("not-a-date");
  });

  it("renders today and yesterday", () => {
    expect(formatRelativeDate("2026-05-25T09:00:00.000Z", now)).toBe("today");
    expect(formatRelativeDate("2026-05-24T09:00:00.000Z", now)).toBe("yesterday");
  });

  it("renders recent days as 'Nd ago'", () => {
    expect(formatRelativeDate("2026-05-22T12:00:00.000Z", now)).toBe("3d ago");
  });

  it("renders older same-year dates as month + day", () => {
    const formatted = formatRelativeDate("2026-01-10T12:00:00.000Z", now);
    expect(formatted).toMatch(/Jan/);
    expect(formatted).toMatch(/10/);
    expect(formatted).not.toMatch(/2026/);
  });

  it("renders prior-year dates with the year", () => {
    const formatted = formatRelativeDate("2024-11-03T12:00:00.000Z", now);
    expect(formatted).toMatch(/2024/);
  });
});
