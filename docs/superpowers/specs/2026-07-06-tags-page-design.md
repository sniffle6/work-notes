# Design: Tags Page

**Date:** 2026-07-06
**Status:** Approved (design), pending implementation plan
**App:** Work Notes (Tauri 2 + SvelteKit + Svelte 5)

## Problem

The sidebar "Tags" nav item is a leftover placeholder `<a href="/">` (AppShell.svelte:126) that just reloads the SPA root — it does nothing useful. Unlike every sibling nav item (Inbox/Today/Actions/Follow-ups/People/Archive), which is a `<button onclick={() => navigate(view)}>`, Tags is not wired to anything, and there is no Tags view: `InboxViewMode` has no `"tags"`, `navigatePrimary` has no tags case, and no `TagsView` component exists.

## Decision

Build a **Tags page as a two-pane master-detail view that mirrors the existing People view** — a searchable list of tags on the left, the selected tag's notes on the right. This is the natural generalization of People (which browses person-tags) to all tag kinds, and it reuses the People structure almost entirely. Frontend-only; no backend changes.

Approaches considered:
- **A. Master-detail browse (chosen)** — consistent with People, cheap (adapts existing code), gives per-tag counts + drill-in.
- B. Click-a-tag-to-filter-Inbox — lighter but no per-tag overview; rejected as less useful.
- C. Tag management (rename/merge/delete) — needs new backend mutation commands; out of scope for making the page work.

## Decisions / defaults (locked with the user)

- **Shows all tag kinds** (`person`, `project`, `topic`, `urgency`, `category`, `custom`), each with a small kind badge. People remains the richer person-only view; Tags is the complete index across all kinds.
- **Detail pane shows the tag's notes only** — not actions (actions are owner/person-based, already covered by People).
- **Derived from the loaded (non-archived) notes**, exactly like People (which loads the inbox with a high limit).
- **Grouping is case-insensitive by kind + name** (mirrors People's `personKey` normalization): a tag's key is `${kind}:${name.trim().toLowerCase()}`, display name is the first-seen trimmed name. This is friendlier than the DB's case-sensitive `UNIQUE(name, kind)`; acceptable for a browse view.

## Architecture

Two units, mirroring People's `people.ts` (pure derive logic) + `PeopleView.svelte` (presentation):

### `src/lib/tags.ts` (new) — pure derive module

A deep module: a small interface over all the grouping/counting/sorting logic, testable in isolation with no Svelte or Tauri dependency.

```ts
import type { NoteListItem, Tag } from "$lib/types";

export type TagKind = Tag["kind"]; // "person" | "project" | "topic" | "urgency" | "category" | "custom"

export type TagSummary = {
  key: string;            // `${kind}:${lowercased trimmed name}`
  name: string;           // first-seen display name
  kind: TagKind;
  noteCount: number;      // distinct notes carrying this tag
  lastUsedAt: string | null; // most recent createdAt among tagged notes
};

export type TagDetail = {
  tag: TagSummary;
  notes: NoteListItem[];  // notes with this tag, newest first
};

export function buildTags(notes: NoteListItem[]): TagSummary[];
export function buildTagDetail(tagKey: string, notes: NoteListItem[]): TagDetail | null;
export function matchesTagSearch(tag: TagSummary, query: string): boolean;
```

Behaviour:
- `buildTags`: iterate notes; for each note, dedupe its tag keys (a note counts once per tag even if tagged from multiple sources); accumulate `noteCount` and `lastUsedAt` (max `createdAt`). Return sorted by `noteCount` desc, then `name` (localeCompare) asc.
- `buildTagDetail`: resolve the key, return the matching `TagSummary` plus the notes carrying that tag sorted by `createdAt` desc; `null` if the key matches no tag.
- `matchesTagSearch`: case-insensitive substring match on name (and kind), empty query matches all.

### `src/lib/components/TagsView.svelte` (new) — presentation

Adapted from `PeopleView.svelte`. Props: `{ notes: NoteListItem[]; loadingNotes?: boolean }`. Emits `openNote: string`.
- Left pane: search input + list of `buildTags(notes)` rows (name, kind badge, note count), filtered by `matchesTagSearch`; selecting a row sets the active tag key (same `$derived` active-key + `$effect` reset pattern as PeopleView).
- Right pane: `buildTagDetail(activeKey, notes)` — header (tag name, kind, note count) + the tag's notes as clickable rows that dispatch `openNote`. Empty states for "no tags yet" / "no tag selected".

## Wiring (small edits to existing files)

- `src/lib/stores/inbox.ts`:
  - Add `"tags"` to the `InboxViewMode` union.
  - Add `showTags()` (mirrors `showPeople`): `viewMode.set("tags"); filters.set(createInboxFilters({ includeArchived: false })); await loadInbox({ limit: 1000 });` and export it.
- `src/lib/components/AppShell.svelte`: replace the `<a href="/">` Tags item (lines 126–129) with a real button matching its siblings:
  ```svelte
  <button
    class:active={activeView === "tags"}
    type="button"
    aria-current={activeView === "tags" ? "page" : undefined}
    aria-label="Tags"
    onclick={() => navigate("tags")}
  >
    <span class="nav-icon" aria-hidden="true"><Hash size={15} strokeWidth={2.2} /></span>
    <span>Tags</span>
  </button>
  ```
- `src/routes/+page.svelte`:
  - Import `TagsView`.
  - Add to `navigatePrimary`: `if (event.detail === "tags") { await workNotes.showTags(); return; }`.
  - Add an `{:else if $viewMode === "tags"}` branch rendering `<TagsView notes={$inbox} loadingNotes={$loadingInbox} on:openNote={(e) => void openNoteFromTags(e.detail)} />`.
  - Add `openNoteFromTags(noteId)` mirroring `openNoteFromPeople` (`await workNotes.showInbox(); await workNotes.selectNote(noteId);`).

## Data flow

Nav "Tags" click → `navigate("tags")` → `+page.svelte navigatePrimary` → `store.showTags()` → `loadInbox({limit:1000})` populates `$inbox` → `TagsView` derives tags via `buildTags($inbox)` → user selects a tag → `buildTagDetail` lists its notes → clicking a note → `openNote` → `openNoteFromTags` → inbox view with the note selected.

## Testing

- `src/lib/tags.test.ts` (TDD, mirrors `people.test.ts`): `buildTags` grouping across notes, per-note dedupe of counts, `lastUsedAt`, sort order, all kinds represented; `buildTagDetail` filters + sorts and returns `null` for unknown key; `matchesTagSearch` behaviour.
- `src/lib/components/TagsView.test.ts` (mirrors `PeopleView.test.ts`): renders the tag list, selecting a tag shows its notes, clicking a note dispatches `openNote`, empty states.
- Full suite + `npm run check` stay green.

## Out of scope / deferred

- **Clickable sidebar tag chips** (jump straight to a tag) — optional extension; not in this version.
- **Tag management** (rename/merge/delete) — needs backend mutations; separate feature.
- **Including archived notes' tags** — v1 derives from the loaded non-archived notes, like People.

## Known trade-offs

- Case-insensitive grouping can merge DB-distinct tags that differ only by case; accepted for a friendlier browse view.
- Deriving from loaded notes means a tag only appears once at least one of its non-archived notes is loaded — consistent with how People behaves.
