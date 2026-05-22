import type { ActionReviewItem, NoteListItem } from "$lib/types";

export type WeekActivity = {
  key: string;
  label: string;
  date: Date;
  captureCount: number;
  dueActionCount: number;
  totalCount: number;
  isToday: boolean;
};

export function actionsDueByToday(
  actions: ActionReviewItem[],
  now = new Date(),
): ActionReviewItem[] {
  const today = startOfLocalDay(now);

  return actions
    .filter((action) => {
      const dueDate = parseDate(action.dueDate);
      return dueDate !== null && startOfLocalDay(dueDate).getTime() <= today.getTime();
    })
    .sort((left, right) => {
      const leftDue = startOfLocalDay(parseDate(left.dueDate) as Date).getTime();
      const rightDue = startOfLocalDay(parseDate(right.dueDate) as Date).getTime();
      if (leftDue !== rightDue) {
        return leftDue - rightDue;
      }

      return sortableTime(left.createdAt) - sortableTime(right.createdAt);
    });
}

export function notesCapturedToday(notes: NoteListItem[], now = new Date()): NoteListItem[] {
  const todayKey = localDateKey(now);

  return notes
    .filter((note) => {
      const createdAt = parseDate(note.createdAt);
      return createdAt !== null && localDateKey(createdAt) === todayKey;
    })
    .sort((left, right) => sortableTime(right.createdAt) - sortableTime(left.createdAt));
}

export function buildWorkWeekActivity(
  notes: NoteListItem[],
  actions: ActionReviewItem[],
  now = new Date(),
): WeekActivity[] {
  const monday = startOfWorkWeek(now);

  return Array.from({ length: 5 }, (_, index) => {
    const date = addDays(monday, index);
    const key = localDateKey(date);
    const captureCount = notes.filter((note) => {
      const createdAt = parseDate(note.createdAt);
      return createdAt !== null && localDateKey(createdAt) === key;
    }).length;
    const dueActionCount = actions.filter((action) => {
      const dueDate = parseDate(action.dueDate);
      return dueDate !== null && localDateKey(dueDate) === key;
    }).length;

    return {
      key,
      label: date.toLocaleDateString([], { weekday: "short" }),
      date,
      captureCount,
      dueActionCount,
      totalCount: captureCount + dueActionCount,
      isToday: localDateKey(now) === key,
    };
  });
}

export function formatTodayHeading(now = new Date()): string {
  return now.toLocaleDateString([], {
    weekday: "long",
    month: "long",
    day: "numeric",
  });
}

export function formatShortTime(value: string): string {
  const date = parseDate(value);
  if (!date) {
    return value;
  }

  return date.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
}

function parseDate(value: string | null | undefined): Date | null {
  if (!value) {
    return null;
  }

  const dateOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (dateOnly) {
    const [, year, month, day] = dateOnly;
    const parsedYear = Number(year);
    const parsedMonth = Number(month);
    const parsedDay = Number(day);
    const parsed = new Date(parsedYear, parsedMonth - 1, parsedDay);

    if (
      parsed.getFullYear() !== parsedYear ||
      parsed.getMonth() !== parsedMonth - 1 ||
      parsed.getDate() !== parsedDay
    ) {
      return null;
    }

    return parsed;
  }

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function startOfWorkWeek(date: Date): Date {
  const start = startOfLocalDay(date);
  const mondayOffset = (start.getDay() + 6) % 7;
  return addDays(start, -mondayOffset);
}

function addDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
}

function localDateKey(date: Date): string {
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

function sortableTime(value: string): number {
  return parseDate(value)?.getTime() ?? Number.POSITIVE_INFINITY;
}
