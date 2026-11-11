import type { FollowupItem, FollowupLane } from "./types";

export function resolveFollowupLane(followup: FollowupItem): string {
  const explicit = followup.followupLane?.trim();
  if (explicit) {
    return explicit;
  }

  const project = followup.tags.find((tag) => tag.kind === "project")?.name.trim();
  if (project) {
    return project;
  }

  const topic = followup.tags.find((tag) => tag.kind === "topic")?.name.trim();
  if (topic) {
    return topic;
  }

  return "Unsorted";
}

export function displayFollowupState(followup: FollowupItem): "Open" | "Waiting" | "Blocked" | "Done" {
  if (followup.status === "done") {
    return "Done";
  }

  switch (followup.followupState ?? "open") {
    case "waiting":
      return "Waiting";
    case "blocked":
      return "Blocked";
    case "open":
      return "Open";
  }
}

export function groupFollowupsByLane(followups: FollowupItem[]): FollowupLane[] {
  const lanes = new Map<string, FollowupItem[]>();

  for (const followup of followups) {
    const lane = resolveFollowupLane(followup);
    lanes.set(lane, [...(lanes.get(lane) ?? []), followup]);
  }

  return Array.from(lanes.entries())
    .map(([name, laneFollowups]) => ({
      name,
      activeCount: laneFollowups.filter((followup) => followup.status !== "done").length,
      followups: laneFollowups,
    }))
    .sort((left, right) => {
      if (left.name === right.name) {
        return 0;
      }
      if (left.name === "Unsorted") {
        return 1;
      }
      if (right.name === "Unsorted") {
        return -1;
      }
      return left.name.localeCompare(right.name);
    });
}

export type FollowupSummary = {
  /** Still to do and not held up. */
  open: number;
  waiting: number;
  blocked: number;
  /** Still to do with a due date before today. */
  overdue: number;
  done: number;
};

export type DueDescription = {
  text: string;
  overdue: boolean;
};

const MS_PER_DAY = 24 * 60 * 60 * 1000;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const ISO_DATE = /^(\d{4})-(\d{2})-(\d{2})/;

/** Counts follow-ups by what the user still has to do about them. */
export function summarizeFollowups(followups: FollowupItem[], today: Date = new Date()): FollowupSummary {
  const summary: FollowupSummary = { open: 0, waiting: 0, blocked: 0, overdue: 0, done: 0 };

  for (const followup of followups) {
    if (followup.status === "done") {
      summary.done += 1;
      continue;
    }
    summary[followup.followupState ?? "open"] += 1;
    if (describeDue(followup.dueDate, today)?.overdue) {
      summary.overdue += 1;
    }
  }

  return summary;
}

/**
 * Lanes as the Follow-ups view lists them. Lanes holding overdue or blocked
 * work come first, and that work comes first inside its lane; everything else
 * keeps its order. Without `showDone`, finished follow-ups and lanes with
 * nothing left are dropped. With it, finished follow-ups end each lane.
 */
export function lanesToShow(lanes: FollowupLane[], showDone: boolean, today: Date = new Date()): FollowupLane[] {
  const needsAttention = (followup: FollowupItem) =>
    followup.followupState === "blocked" || Boolean(describeDue(followup.dueDate, today)?.overdue);

  const shown = lanes
    .map((lane) => {
      const active = lane.followups.filter((followup) => followup.status !== "done");
      const done = lane.followups.filter((followup) => followup.status === "done");
      const urgent = active.filter(needsAttention);
      const ordered = [...urgent, ...active.filter((followup) => !needsAttention(followup))];
      return {
        lane: { ...lane, followups: showDone ? [...ordered, ...done] : ordered },
        urgent: urgent.length > 0,
      };
    })
    .filter((entry) => entry.lane.followups.length > 0);

  return [...shown.filter((entry) => entry.urgent), ...shown.filter((entry) => !entry.urgent)].map(
    (entry) => entry.lane,
  );
}

/**
 * Describes a due date relative to `today`. Returns `null` without a date.
 * Text that does not start with `YYYY-MM-DD` is shown as written and is
 * never treated as overdue.
 */
export function describeDue(dueDate: string | null | undefined, today: Date = new Date()): DueDescription | null {
  const value = dueDate?.trim();
  if (!value) {
    return null;
  }

  const match = ISO_DATE.exec(value);
  if (!match) {
    return { text: `Due ${value}`, overdue: false };
  }

  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const due = new Date(year, month - 1, day);
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const days = Math.round((due.getTime() - startOfToday.getTime()) / MS_PER_DAY);

  if (days < 0) {
    return { text: `${-days} ${days === -1 ? "day" : "days"} overdue`, overdue: true };
  }
  if (days === 0) {
    return { text: "Due today", overdue: false };
  }
  if (days === 1) {
    return { text: "Due tomorrow", overdue: false };
  }
  const sameYear = year === today.getFullYear();
  return { text: `Due ${MONTHS[month - 1]} ${day}${sameYear ? "" : `, ${year}`}`, overdue: false };
}
