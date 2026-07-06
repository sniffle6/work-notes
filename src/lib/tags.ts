import type { NoteListItem, TagKind } from "$lib/types";

export type TagSummary = {
  key: string;
  name: string;
  kind: TagKind;
  noteCount: number;
  lastUsedAt: string | null;
};

export type RelatedTag = {
  key: string;
  name: string;
  kind: TagKind;
  coCount: number;
};

export type TagDetail = {
  tag: TagSummary;
  notes: NoteListItem[];
  relatedTags: RelatedTag[];
};

export function buildTags(notes: NoteListItem[]): TagSummary[] {
  const summaries = new Map<string, TagSummary>();

  for (const note of notes) {
    // Dedupe within a note: same name+kind from multiple sources counts once.
    const seen = new Map<string, { name: string; kind: TagKind }>();
    for (const raw of note.tags) {
      const name = raw.name.trim();
      if (!name) {
        continue;
      }
      const key = tagKey(raw.kind, name);
      if (!seen.has(key)) {
        seen.set(key, { name, kind: raw.kind });
      }
    }

    for (const [key, { name, kind }] of seen) {
      const existing = summaries.get(key);
      if (existing) {
        existing.noteCount += 1;
        existing.lastUsedAt = newerDate(existing.lastUsedAt, note.createdAt);
      } else {
        summaries.set(key, {
          key,
          name,
          kind,
          noteCount: 1,
          lastUsedAt: note.createdAt || null,
        });
      }
    }
  }

  return Array.from(summaries.values()).sort(compareTags);
}

export function buildTagDetail(key: string, notes: NoteListItem[]): TagDetail | null {
  const tag = buildTags(notes).find((summary) => summary.key === key);
  if (!tag) {
    return null;
  }

  const tagged = notes
    .filter((note) => noteHasTagKey(note, key))
    .sort((left, right) => sortableTime(right.createdAt) - sortableTime(left.createdAt));

  const related = new Map<string, RelatedTag>();
  for (const note of tagged) {
    const seen = new Set<string>();
    for (const raw of note.tags) {
      const name = raw.name.trim();
      if (!name) {
        continue;
      }
      const otherKey = tagKey(raw.kind, name);
      if (otherKey === key || seen.has(otherKey)) {
        continue;
      }
      seen.add(otherKey);
      const existing = related.get(otherKey);
      if (existing) {
        existing.coCount += 1;
      } else {
        related.set(otherKey, { key: otherKey, name, kind: raw.kind, coCount: 1 });
      }
    }
  }

  const relatedTags = Array.from(related.values()).sort(
    (left, right) => right.coCount - left.coCount || left.name.localeCompare(right.name),
  );

  return { tag, notes: tagged, relatedTags };
}

export function matchesTagSearch(tag: TagSummary, query: string): boolean {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) {
    return true;
  }
  return [tag.name, tag.kind].join(" ").toLocaleLowerCase().includes(normalized);
}

function tagKey(kind: TagKind, name: string): string {
  return `${kind}:${name.trim().toLocaleLowerCase()}`;
}

function noteHasTagKey(note: NoteListItem, key: string): boolean {
  return note.tags.some((tag) => tag.name.trim() !== "" && tagKey(tag.kind, tag.name) === key);
}

function compareTags(left: TagSummary, right: TagSummary): number {
  if (left.noteCount !== right.noteCount) {
    return right.noteCount - left.noteCount;
  }
  return left.name.localeCompare(right.name);
}

function newerDate(current: string | null, next: string): string | null {
  if (!current) {
    return next || null;
  }
  if (!next) {
    return current;
  }
  return sortableTime(next) >= sortableTime(current) ? next : current;
}

function sortableTime(value: string | null | undefined): number {
  if (!value) {
    return Number.NEGATIVE_INFINITY;
  }
  const parsed = new Date(value).getTime();
  return Number.isNaN(parsed) ? Number.NEGATIVE_INFINITY : parsed;
}
