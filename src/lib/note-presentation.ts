import type { ParseStatus, ReviewStatus } from "$lib/types";

/** Human label for a note's current parse/review state. */
export function noteStatusLabel(parseStatus: ParseStatus, reviewStatus: ReviewStatus): string {
  if (parseStatus === "failed") return "Parse failed";
  if (parseStatus === "parsing") return "Parsing";
  if (parseStatus === "queued") return "Queued";
  if (reviewStatus === "needs_review") return "Needs review";
  if (reviewStatus === "reviewed") return "Reviewed";
  return "Captured";
}

/** Status-dot CSS class for a note's parse/review state (matches the app's `.status-dot` tones). */
export function noteStatusClass(parseStatus: ParseStatus, reviewStatus: ReviewStatus): string {
  if (parseStatus === "failed") return "error";
  if (parseStatus === "parsing") return "info";
  if (parseStatus === "queued") return "neutral";
  if (reviewStatus === "needs_review") return "warning";
  if (reviewStatus === "reviewed") return "success reviewed";
  return "neutral";
}

/**
 * Compact relative date for list rows: "today", "yesterday", "3d ago", "in 2d",
 * "May 22" (same year), or "Nov 2024" (prior year). `now` is injectable for tests.
 */
export function formatRelativeDate(
  value: string | null | undefined,
  now: Date = new Date(),
): string {
  if (!value || !value.trim()) {
    return "never";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const startOfDay = (d: Date) => Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate());
  const days = Math.round((startOfDay(date) - startOfDay(now)) / 86_400_000);

  if (days === 0) return "today";
  if (days === -1) return "yesterday";
  if (days === 1) return "tomorrow";
  if (days < 0 && days > -7) return `${Math.abs(days)}d ago`;
  if (days > 0 && days < 7) return `in ${days}d`;

  if (date.getUTCFullYear() === now.getUTCFullYear()) {
    return date.toLocaleDateString([], { month: "short", day: "numeric", timeZone: "UTC" });
  }

  return date.toLocaleDateString([], { month: "short", year: "numeric", timeZone: "UTC" });
}
