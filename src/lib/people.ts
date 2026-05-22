import type { ActionReviewItem, NoteListItem } from "$lib/types";

export type PersonSummary = {
  key: string;
  name: string;
  noteCount: number;
  actionCount: number;
  lastInteractionAt: string | null;
};

export type PersonAction = ActionReviewItem & {
  sourceNote?: NoteListItem;
};

export type PersonDetail = {
  person: PersonSummary;
  youOwe: PersonAction[];
  theyOwe: PersonAction[];
  recentNotes: NoteListItem[];
};

type MutablePerson = PersonSummary;

export function buildPeople(
  notes: NoteListItem[],
  actions: ActionReviewItem[],
): PersonSummary[] {
  const people = new Map<string, MutablePerson>();

  for (const note of notes) {
    const notePersonKeys = new Set<string>();

    for (const tag of note.tags) {
      if (tag.kind !== "person") {
        continue;
      }

      const key = personKey(tag.name);
      if (!key) {
        continue;
      }

      ensurePerson(people, tag.name, key);
      notePersonKeys.add(key);
    }

    for (const key of notePersonKeys) {
      const person = people.get(key);
      if (!person) {
        continue;
      }

      person.noteCount += 1;
      person.lastInteractionAt = newerInteraction(person.lastInteractionAt, note.createdAt);
    }
  }

  for (const action of actions) {
    const key = personKey(action.owner);
    if (!key || key === "me") {
      continue;
    }

    ensurePerson(people, action.owner ?? key, key).actionCount += 1;
  }

  return Array.from(people.values()).sort(comparePeople);
}

export function buildPersonDetail(
  selectedPersonKey: string,
  notes: NoteListItem[],
  actions: ActionReviewItem[],
): PersonDetail | null {
  const key = personKeyFromInput(selectedPersonKey);
  if (!key) {
    return null;
  }

  const person = buildPeople(notes, actions).find((item) => item.key === key);
  if (!person) {
    return null;
  }

  const sourceNotes = new Map(notes.map((note) => [note.id, note]));
  const recentNotes = notes
    .filter((note) => noteHasPerson(note, key))
    .sort((left, right) => sortableTime(right.createdAt) - sortableTime(left.createdAt));

  const youOwe = actions
    .filter((action) => personKey(action.owner) === "me" && noteHasPerson(sourceNotes.get(action.noteId), key))
    .map((action) => attachSourceNote(action, sourceNotes))
    .sort(compareActions);

  const theyOwe = actions
    .filter((action) => personKey(action.owner) === key)
    .map((action) => attachSourceNote(action, sourceNotes))
    .sort(compareActions);

  return {
    person,
    youOwe,
    theyOwe,
    recentNotes,
  };
}

export function matchesPersonSearch(person: PersonSummary, query: string): boolean {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) {
    return true;
  }

  return [person.name, person.key].join(" ").toLocaleLowerCase().includes(normalized);
}

export function formatPersonWhen(value: string | null): string {
  const date = parseDate(value);
  if (!date) {
    return "never";
  }

  const days = dayDifference(date, new Date());
  if (days === 0) return "today";
  if (days === -1) return "yesterday";
  if (days === 1) return "tomorrow";
  if (days < 0 && days > -7) return `${Math.abs(days)}d ago`;
  if (days > 0 && days < 7) return `in ${days}d`;

  if (date.getFullYear() === new Date().getFullYear()) {
    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  }

  return date.toLocaleDateString([], { month: "short", year: "numeric" });
}

export function avatarHue(name: string): number {
  const trimmed = name.trim();
  if (!trimmed) {
    return 0;
  }

  return (trimmed.charCodeAt(0) * 37) % 360;
}

function ensurePerson(
  people: Map<string, MutablePerson>,
  rawName: string,
  knownKey = personKey(rawName),
): MutablePerson {
  const existing = people.get(knownKey);
  if (existing) {
    return existing;
  }

  const person: MutablePerson = {
    key: knownKey,
    name: rawName.trim(),
    noteCount: 0,
    actionCount: 0,
    lastInteractionAt: null,
  };
  people.set(knownKey, person);
  return person;
}

function personKey(value: string | null | undefined): string {
  return value?.trim().toLocaleLowerCase() ?? "";
}

function personKeyFromInput(value: string): string {
  return value.trim().toLocaleLowerCase();
}

function noteHasPerson(note: NoteListItem | undefined, key: string): boolean {
  return note?.tags.some((tag) => tag.kind === "person" && personKey(tag.name) === key) ?? false;
}

function attachSourceNote(
  action: ActionReviewItem,
  notes: Map<string, NoteListItem>,
): PersonAction {
  return {
    ...action,
    sourceNote: notes.get(action.noteId),
  };
}

function comparePeople(left: PersonSummary, right: PersonSummary): number {
  const leftTime = sortableTime(left.lastInteractionAt);
  const rightTime = sortableTime(right.lastInteractionAt);
  const leftHasInteraction = leftTime !== Number.POSITIVE_INFINITY;
  const rightHasInteraction = rightTime !== Number.POSITIVE_INFINITY;

  if (leftHasInteraction !== rightHasInteraction) {
    return leftHasInteraction ? -1 : 1;
  }

  if (leftTime !== rightTime) {
    return rightTime - leftTime;
  }

  return left.name.localeCompare(right.name);
}

function compareActions(left: ActionReviewItem, right: ActionReviewItem): number {
  const leftDue = sortableTime(left.dueDate);
  const rightDue = sortableTime(right.dueDate);
  if (leftDue !== rightDue) {
    return leftDue - rightDue;
  }

  return sortableTime(left.createdAt) - sortableTime(right.createdAt);
}

function newerInteraction(current: string | null, next: string): string | null {
  if (!current) {
    return next;
  }

  const currentTime = sortableTime(current);
  const nextTime = sortableTime(next);
  if (currentTime === Number.POSITIVE_INFINITY) {
    return nextTime === Number.POSITIVE_INFINITY ? current : next;
  }

  if (nextTime === Number.POSITIVE_INFINITY) {
    return current;
  }

  return nextTime >= currentTime ? next : current;
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

function sortableTime(value: string | null | undefined): number {
  return parseDate(value)?.getTime() ?? Number.POSITIVE_INFINITY;
}

function dayDifference(date: Date, now: Date): number {
  const start = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  const target = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  return Math.round((target - start) / 86_400_000);
}
