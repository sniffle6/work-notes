import type { InboxFilters, NoteListItem } from "$lib/types";

export function createInboxFilters(overrides: Partial<InboxFilters> = {}): InboxFilters {
  return {
    search: overrides.search ?? "",
    tagIds: [...(overrides.tagIds ?? [])],
    parseStatuses: [...(overrides.parseStatuses ?? [])],
    reviewStatuses: [...(overrides.reviewStatuses ?? [])],
    includeArchived: overrides.includeArchived,
    limit: overrides.limit,
  };
}

export function matchesNoteFilters(note: NoteListItem, filters: InboxFilters): boolean {
  if (!filters.includeArchived && note.isArchived) {
    return false;
  }

  const search = filters.search.trim().toLocaleLowerCase();
  if (search && !searchableText(note).includes(search)) {
    return false;
  }

  if (filters.tagIds.length > 0 && !note.tags.some((tag) => filters.tagIds.includes(tag.id))) {
    return false;
  }

  if (filters.parseStatuses.length > 0 && !filters.parseStatuses.includes(note.parseStatus)) {
    return false;
  }

  if (filters.reviewStatuses.length > 0 && !filters.reviewStatuses.includes(note.reviewStatus)) {
    return false;
  }

  return true;
}

function searchableText(note: NoteListItem): string {
  return [
    note.title,
    note.rawText,
    note.cleanedText,
    note.summary,
    note.captureSource,
    ...note.tags.map((tag) => tag.name),
  ]
    .filter(Boolean)
    .join(" ")
    .toLocaleLowerCase();
}
