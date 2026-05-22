import type { ActionReviewItem } from "$lib/types";

export type ActionDueBucket = "Overdue" | "Today" | "This week" | "Later" | "No date";

export type GroupedActions = {
  label: ActionDueBucket;
  actions: ActionReviewItem[];
};

const BUCKETS: ActionDueBucket[] = ["Overdue", "Today", "This week", "Later", "No date"];

export function groupActionsByDueBucket(
  actions: ActionReviewItem[],
  now = new Date(),
): GroupedActions[] {
  const grouped = new Map<ActionDueBucket, ActionReviewItem[]>(
    BUCKETS.map((bucket) => [bucket, []]),
  );

  for (const action of actions) {
    grouped.get(bucketForDueDate(action.dueDate, now))?.push(action);
  }

  return BUCKETS.map((label) => ({ label, actions: grouped.get(label) ?? [] })).filter(
    (group) => group.actions.length > 0,
  );
}

export function actionMatchesSearch(
  action: ActionReviewItem,
  query: string,
  now = new Date(),
): boolean {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) {
    return true;
  }

  return [action.text, action.owner, action.dueDate, formatActionDue(action.dueDate, now), action.noteTitle]
    .filter(Boolean)
    .join(" ")
    .toLocaleLowerCase()
    .includes(normalized);
}

export function formatActionDue(dueDate: string | null | undefined, now = new Date()): string | null {
  if (!dueDate) {
    return null;
  }

  const parsed = parseDueDate(dueDate);
  if (!parsed) {
    return dueDate;
  }

  const days = dayDifference(parsed, now);
  if (days < 0) return `${Math.abs(days)}d overdue`;
  if (days === 0) return "today";
  if (days === 1) return "tomorrow";
  if (days < 7) {
    return parsed.toLocaleDateString([], { weekday: "short" });
  }

  return parsed.toLocaleDateString([], { month: "short", day: "numeric" });
}

export function dueTone(
  dueDate: string | null | undefined,
  now = new Date(),
): "error" | "warning" | "muted" {
  const parsed = parseDueDate(dueDate);
  if (!parsed) {
    return "muted";
  }

  const days = dayDifference(parsed, now);
  if (days < 0) return "error";
  if (days <= 1) return "warning";
  return "muted";
}

function bucketForDueDate(dueDate: string | null | undefined, now: Date): ActionDueBucket {
  const parsed = parseDueDate(dueDate);
  if (!parsed) {
    return "No date";
  }

  const days = dayDifference(parsed, now);
  if (days < 0) return "Overdue";
  if (days === 0) return "Today";
  if (days <= 6) return "This week";
  return "Later";
}

function parseDueDate(value: string | null | undefined): Date | null {
  if (!value) {
    return null;
  }

  const dateOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (dateOnly) {
    const [, year, month, day] = dateOnly;
    return new Date(Number(year), Number(month) - 1, Number(day));
  }

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function dayDifference(date: Date, now: Date): number {
  const start = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  const target = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  return Math.round((target - start) / 86_400_000);
}
