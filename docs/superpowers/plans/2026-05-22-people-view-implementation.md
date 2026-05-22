# People View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the existing `09 · People — Maria · Memphis` menu design as a real top-level Work Notes view.

**Architecture:** Keep the slice frontend/store-only. The store owns primary-view sequencing; pure People helpers own person/action derivation; `PeopleView.svelte` owns rendering and emits events instead of calling persistence directly.

**Tech Stack:** Svelte 5, Svelte stores, TypeScript, Vitest, Testing Library Svelte.

---

## File Map

- Create `src/lib/people.ts`: pure derivation helpers for person summaries, selected person detail, action grouping, and date labels.
- Create `src/lib/people.test.ts`: focused tests for person extraction and action/note grouping.
- Create `src/lib/components/PeopleView.svelte`: production People surface matching the supplied design shape.
- Create `src/lib/components/PeopleView.test.ts`: component rendering, filtering, and event tests.
- Modify `src/lib/stores/inbox.ts`: add `people` view mode and `showPeople()`.
- Modify `src/lib/stores/inbox.test.ts`: cover People store sequencing.
- Modify `src/lib/components/AppShell.svelte`: replace inert People link with navigation button.
- Modify `src/lib/components/AppShell.test.ts`: cover People active/nav behavior.
- Modify `src/routes/+page.svelte`: render `PeopleView` when `viewMode === "people"` and wire open-note events.

## Task 1: People Derivation Helpers

**Files:**

- Create: `src/lib/people.ts`
- Create: `src/lib/people.test.ts`

- [ ] Add helper tests.

Use notes with `person` tags for `Maria` and `Jin`, plus suggested actions:

- owner `me` on a note tagged `Maria`;
- owner `Maria`;
- owner `jin`;
- owner `me` on a note without a person tag;
- owner `me` on a note tagged `Jin`.

Verify:

- `buildPeople()` returns `Maria` and `Jin`, but not `me`.
- `Maria` has the correct note count, last interaction, and action count.
- `buildPersonDetail("Maria", notes, actions)` returns:
  - `youOwe` containing owner `me` actions from Maria-tagged notes;
  - `theyOwe` containing owner `Maria` actions;
  - `recentNotes` containing Maria-tagged notes newest first.
- owner matching is case-insensitive.

- [ ] Implement `src/lib/people.ts`.

Exports:

```ts
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

export function buildPeople(notes: NoteListItem[], actions: ActionReviewItem[]): PersonSummary[];
export function buildPersonDetail(personKey: string, notes: NoteListItem[], actions: ActionReviewItem[]): PersonDetail | null;
export function matchesPersonSearch(person: PersonSummary, query: string): boolean;
export function formatPersonWhen(value: string | null): string;
export function avatarHue(name: string): number;
```

Implementation notes:

- Person key is `name.trim().toLocaleLowerCase()`.
- Include person tags with `kind === "person"`.
- Include action owners other than `me`, even if no matching tag exists.
- Use `noteId` to attach source notes to actions.
- Sort people by most recent `lastInteractionAt`, then by name.
- Sort actions by `dueDate` if present, then `createdAt`.
- Sort recent notes by `createdAt` descending.
- `formatPersonWhen` returns `never` for null and compact relative labels for valid dates.

- [ ] Run targeted helper tests.

```powershell
npm test -- src/lib/people.test.ts
```

Expected: helper tests pass.

## Task 2: Store And Shell Navigation

**Files:**

- Modify: `src/lib/stores/inbox.ts`
- Modify: `src/lib/stores/inbox.test.ts`
- Modify: `src/lib/components/AppShell.svelte`
- Modify: `src/lib/components/AppShell.test.ts`

- [ ] Add failing tests.

Store test:

- Set a stale search filter.
- `showPeople()` sets `viewMode` to `people`.
- It calls `listInbox` with `createInboxFilters({ includeArchived: false })`.
- It calls `listSuggestedActions`.
- It filters archived notes out of `inbox`.

AppShell test:

- rendering with `activeView: "people"` marks People with `aria-current="page"`;
- clicking People emits `navigate` detail `"people"`.

- [ ] Implement store and shell changes.

Required code shape:

- Change `InboxViewMode` to include `"people"`.
- Add:

```ts
async function showPeople(): Promise<void> {
  viewMode.set("people");
  filters.set(createInboxFilters({ includeArchived: false }));
  await loadInbox();
  await loadSuggestedActions();
}
```

- Return `showPeople`.
- Replace the sidebar People `<a>` with a `<button>` matching Today/Actions/Archive navigation.

- [ ] Run targeted tests.

```powershell
npm test -- src/lib/stores/inbox.test.ts src/lib/components/AppShell.test.ts
```

Expected: targeted tests pass.

## Task 3: PeopleView Component And Route Wiring

**Files:**

- Create: `src/lib/components/PeopleView.svelte`
- Create: `src/lib/components/PeopleView.test.ts`
- Modify: `src/routes/+page.svelte`

- [ ] Add component tests.

Use Testing Library Svelte to verify:

- people rows render from notes/actions;
- search filters the people list;
- selecting a person updates the detail header;
- `You owe Maria`, `Maria owes you`, and `Recent notes` sections render the expected rows;
- clicking an action row dispatches `openNote` with its source note id;
- clicking a recent note row dispatches `openNote` with its note id.

- [ ] Implement `PeopleView.svelte`.

Props:

```ts
type Props = {
  notes: NoteListItem[];
  actions: ActionReviewItem[];
  loadingNotes?: boolean;
  loadingActions?: boolean;
};
```

Events:

```ts
openNote: string;
```

Rendering:

- Full-height two-column `<section class="people-view" aria-label="People">`.
- Left list column with header, search input, and person rows.
- Right detail column with selected person header, two action columns, and recent notes.
- Preserve compact operational styling using semantic CSS variables only.
- Do not render a working `New note about <person>` button in this slice; if shown visually, keep it disabled or omit it.

- [ ] Wire `src/routes/+page.svelte`.

Import `PeopleView`. In `navigatePrimary`, handle `"people"` with `await workNotes.showPeople()`.

When `$viewMode === "people"`, render:

```svelte
<PeopleView
  notes={$inbox}
  actions={$suggestedActions}
  loadingNotes={$loadingInbox}
  loadingActions={$loadingSuggestedActions}
  on:openNote={(event) => void openNoteFromPeople(event.detail)}
/>
```

Add `openNoteFromPeople(noteId)` to call `await workNotes.showInbox(); await workNotes.selectNote(noteId);`.

- [ ] Run targeted frontend tests.

```powershell
npm test -- src/lib/people.test.ts src/lib/components/PeopleView.test.ts src/lib/components/AppShell.test.ts src/lib/stores/inbox.test.ts
```

Expected: targeted tests pass.

## Task 4: Verification

**Files:**

- No new files.

- [ ] Run full frontend verification.

```powershell
npm test
npm run check
npm run build
```

Expected: all pass.

- [ ] Run Rust verification.

```powershell
scripts\cargo-test.cmd
```

Expected: all Rust tests pass.

- [ ] Check the working tree.

```powershell
git status --short --branch
git diff --check
```

Expected: only intended People-view files changed, and no whitespace errors.
