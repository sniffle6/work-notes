import type { ActionReviewItem, FollowupItem } from "$lib/types";

export type CalendarTaskStatus = "suggested" | "accepted" | "done";
export type CalendarMomentKind = "captured" | "due" | "completed";
export type CalendarMomentPosition = "past" | "selected" | "future";

export type CalendarTask = {
  id: string;
  noteId: string;
  noteTitle: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  status: CalendarTaskStatus;
  capturedAt: string;
  completedAt?: string | null;
};

export type CalendarOccurrence = {
  key: string;
  dateKey: string;
  kinds: CalendarMomentKind[];
  task: CalendarTask;
};

export type CalendarLifecycleMoment = {
  kind: CalendarMomentKind;
  dateKey: string;
  position: CalendarMomentPosition;
  isOverdue: boolean;
};

export type CalendarDay = {
  key: string;
  date: Date;
  isCurrentMonth: boolean;
  isToday: boolean;
  capturedCount: number;
  dueCount: number;
  completedCount: number;
};

export function buildCalendarTasks(
  suggested: ActionReviewItem[],
  followups: FollowupItem[],
): CalendarTask[] {
  return [
    ...suggested.map((action) => ({
      id: action.id,
      noteId: action.noteId,
      noteTitle: action.noteTitle,
      text: action.text,
      owner: action.owner,
      dueDate: action.dueDate,
      status: "suggested" as const,
      capturedAt: action.createdAt,
      completedAt: null,
    })),
    ...followups.map((action) => ({
      id: action.id,
      noteId: action.noteId,
      noteTitle: action.noteTitle,
      text: action.text,
      owner: action.owner,
      dueDate: action.dueDate,
      status: action.status,
      capturedAt: action.createdAt,
      completedAt: action.completedAt,
    })),
  ];
}

export function buildCalendarMonth(
  tasks: CalendarTask[],
  visibleMonth: Date,
  today = new Date(),
): CalendarDay[] {
  const monthStart = new Date(visibleMonth.getFullYear(), visibleMonth.getMonth(), 1);
  const gridStart = addDays(monthStart, -monthStart.getDay());
  const todayKey = localDateKey(today);

  return Array.from({ length: 42 }, (_, index) => {
    const date = addDays(gridStart, index);
    const key = localDateKey(date);
    const occurrences = calendarOccurrencesForDate(tasks, date);

    return {
      key,
      date,
      isCurrentMonth: date.getMonth() === monthStart.getMonth(),
      isToday: key === todayKey,
      capturedCount: countKind(occurrences, "captured"),
      dueCount: countKind(occurrences, "due"),
      completedCount: countKind(occurrences, "completed"),
    };
  });
}

export function calendarOccurrencesForDate(
  tasks: CalendarTask[],
  date: Date,
): CalendarOccurrence[] {
  const key = localDateKey(date);

  return tasks
    .map((task) => occurrenceForTaskOnDate(task, key))
    .filter((occurrence): occurrence is CalendarOccurrence => occurrence !== null)
    .sort(compareOccurrences);
}

export function buildTaskLifecycle(
  task: CalendarTask,
  selectedDate: Date,
  today = new Date(),
): CalendarLifecycleMoment[] {
  const selectedKey = localDateKey(selectedDate);
  const todayKey = localDateKey(today);
  const values: Array<[CalendarMomentKind, string | null | undefined]> = [
    ["captured", task.capturedAt],
    ["due", task.dueDate],
    ["completed", task.status === "done" ? task.completedAt : null],
  ];

  return values.flatMap(([kind, value]) => {
    const dateKey = dateValueKey(value);
    if (!dateKey) return [];

    return [{
      kind,
      dateKey,
      position: dateKey === selectedKey ? "selected" : dateKey < selectedKey ? "past" : "future",
      isOverdue: kind === "due" && task.status !== "done" && dateKey < todayKey,
    }];
  });
}

export function formatMonthHeading(date: Date): string {
  return date.toLocaleDateString([], { month: "long", year: "numeric" });
}

export function formatSelectedDate(date: Date): string {
  return date.toLocaleDateString([], {
    weekday: "long",
    month: "long",
    day: "numeric",
  });
}

export function localDateKey(date: Date): string {
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

export function dateValueKey(value: string | null | undefined): string | null {
  const date = parseDate(value);
  return date ? localDateKey(date) : null;
}

function occurrenceForTaskOnDate(task: CalendarTask, dateKey: string): CalendarOccurrence | null {
  const kinds: CalendarMomentKind[] = [];

  if (dateValueKey(task.capturedAt) === dateKey) kinds.push("captured");
  if (dateValueKey(task.dueDate) === dateKey) kinds.push("due");
  if (task.status === "done" && dateValueKey(task.completedAt) === dateKey) kinds.push("completed");

  if (kinds.length === 0) return null;

  return {
    key: `${task.id}:${dateKey}`,
    dateKey,
    kinds,
    task,
  };
}

function countKind(occurrences: CalendarOccurrence[], kind: CalendarMomentKind): number {
  return occurrences.filter((occurrence) => occurrence.kinds.includes(kind)).length;
}

function compareOccurrences(left: CalendarOccurrence, right: CalendarOccurrence): number {
  const leftKind = momentSortValue(left.kinds);
  const rightKind = momentSortValue(right.kinds);
  if (leftKind !== rightKind) return leftKind - rightKind;
  if (left.task.status !== right.task.status) {
    if (left.task.status === "accepted") return -1;
    if (right.task.status === "accepted") return 1;
    if (left.task.status === "suggested") return -1;
    if (right.task.status === "suggested") return 1;
  }
  return sortableTime(left.task.capturedAt) - sortableTime(right.task.capturedAt);
}

function momentSortValue(kinds: CalendarMomentKind[]): number {
  if (kinds.includes("captured")) return 0;
  if (kinds.includes("due")) return 1;
  return 2;
}

function parseDate(value: string | null | undefined): Date | null {
  if (!value) return null;

  const dateOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (dateOnly) {
    const [, year, month, day] = dateOnly;
    const parsed = new Date(Number(year), Number(month) - 1, Number(day));
    if (
      parsed.getFullYear() !== Number(year) ||
      parsed.getMonth() !== Number(month) - 1 ||
      parsed.getDate() !== Number(day)
    ) {
      return null;
    }
    return parsed;
  }

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function addDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
}

function sortableTime(value: string | null | undefined): number {
  return parseDate(value)?.getTime() ?? Number.POSITIVE_INFINITY;
}
