import type { ActionReviewItem, FollowupItem } from "$lib/types";

export type CalendarTaskStatus = "suggested" | "accepted" | "done";

export type CalendarTask = {
  id: string;
  noteId: string;
  noteTitle: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  status: CalendarTaskStatus;
  createdAt: string;
  completedAt?: string | null;
};

export type CalendarDay = {
  key: string;
  date: Date;
  isCurrentMonth: boolean;
  isToday: boolean;
  openCount: number;
  doneCount: number;
};

export type CalendarDayTasks = {
  open: CalendarTask[];
  done: CalendarTask[];
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
      createdAt: action.createdAt,
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
      createdAt: action.createdAt,
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
    const dayTasks = tasksForCalendarDate(tasks, date);

    return {
      key,
      date,
      isCurrentMonth: date.getMonth() === monthStart.getMonth(),
      isToday: key === todayKey,
      openCount: dayTasks.open.length,
      doneCount: dayTasks.done.length,
    };
  });
}

export function tasksForCalendarDate(
  tasks: CalendarTask[],
  date: Date,
  includeOverdueOpen = false,
): CalendarDayTasks {
  const key = localDateKey(date);

  const open = tasks
    .filter((task) => {
      if (task.status === "done") return false;
      const dueKey = dateValueKey(task.dueDate);
      return dueKey === key || (includeOverdueOpen && dueKey !== null && dueKey < key);
    })
    .sort(compareOpenTasks);

  const done = tasks
    .filter((task) => task.status === "done" && dateValueKey(task.completedAt) === key)
    .sort((left, right) => sortableTime(right.completedAt) - sortableTime(left.completedAt));

  return { open, done };
}

export function unscheduledOpenTasks(tasks: CalendarTask[]): CalendarTask[] {
  return tasks
    .filter((task) => task.status !== "done" && dateValueKey(task.dueDate) === null)
    .sort((left, right) => sortableTime(right.createdAt) - sortableTime(left.createdAt));
}

export function unplacedDoneTasks(tasks: CalendarTask[]): CalendarTask[] {
  return tasks
    .filter((task) => task.status === "done" && dateValueKey(task.completedAt) === null)
    .sort((left, right) => sortableTime(right.createdAt) - sortableTime(left.createdAt));
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

function dateValueKey(value: string | null | undefined): string | null {
  const date = parseDate(value);
  return date ? localDateKey(date) : null;
}

function compareOpenTasks(left: CalendarTask, right: CalendarTask): number {
  const leftDue = sortableTime(left.dueDate);
  const rightDue = sortableTime(right.dueDate);
  if (leftDue !== rightDue) return leftDue - rightDue;
  if (left.status !== right.status) return left.status === "accepted" ? -1 : 1;
  return sortableTime(left.createdAt) - sortableTime(right.createdAt);
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
