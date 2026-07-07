import type { ActionReviewItem, FollowupItem, NoteListItem } from "$lib/types";

/**
 * Stable counts and tag list for the sidebar/nav chrome.
 *
 * These numbers describe the whole workspace, not whatever list the active
 * view happens to be showing. Deriving them from a dedicated snapshot (rather
 * than the transient `inbox`/`suggestedActions`/`followups` view stores) keeps
 * the nav badges from shifting when the user merely navigates — e.g. opening
 * Archive must not repaint the "Inbox" badge with the archived count.
 */
export type NavSummary = {
  inbox: number;
  needsReview: number;
  followups: number;
  parseFailed: number;
  parseQueue: number;
  tags: string[];
};

export const EMPTY_NAV_SUMMARY: NavSummary = {
  inbox: 0,
  needsReview: 0,
  followups: 0,
  parseFailed: 0,
  parseQueue: 0,
  tags: [],
};

/**
 * Build the nav summary from canonical, view-independent inputs:
 * the full set of notes, the suggested-action queue, and all follow-ups.
 * Archived notes never contribute to inbox counts, the parser queue, or tags.
 */
export function buildNavSummary(
  notes: NoteListItem[],
  suggestedActions: ActionReviewItem[],
  followups: FollowupItem[],
): NavSummary {
  const active = notes.filter((note) => !note.isArchived);

  const tags = Array.from(
    new Set(active.flatMap((note) => note.tags.map((tag) => tag.name))),
  ).sort((left, right) => left.localeCompare(right));

  return {
    inbox: active.length,
    needsReview: suggestedActions.length,
    followups: followups.filter((item) => item.status !== "done").length,
    parseFailed: active.filter((note) => note.parseStatus === "failed").length,
    parseQueue: active.filter(
      (note) => note.parseStatus === "queued" || note.parseStatus === "parsing",
    ).length,
    tags,
  };
}
