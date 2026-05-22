# Today View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the existing `03 · Today · Memphis` menu design as a real top-level Work Notes view.

**Architecture:** Keep the slice frontend/store-only. The store owns primary-view sequencing; pure Today helpers own date bucketing; `TodayView.svelte` owns rendering and emits events instead of calling persistence directly.

**Tech Stack:** Svelte 5, Svelte stores, TypeScript, Vitest, Testing Library Svelte.

---

## File Map

- Create `src/lib/today.ts`: pure derivation helpers for due actions, captured-today notes, and weekday activity.
- Create `src/lib/today.test.ts`: focused tests for date and bucketing behavior.
- Create `src/lib/components/TodayView.svelte`: production Today dashboard matching the supplied design shape.
- Create `src/lib/components/TodayView.test.ts`: component rendering and event tests.
- Modify `src/lib/stores/inbox.ts`: add `today` view mode and `showToday()`.
- Modify `src/lib/stores/inbox.test.ts`: cover Today store sequencing.
- Modify `src/lib/components/AppShell.svelte`: replace inert Today link with navigation button.
- Modify `src/lib/components/AppShell.test.ts`: cover Today active/nav behavior.
- Modify `src/routes/+page.svelte`: render `TodayView` when `viewMode === "today"` and wire events.

## Task 1: Today Derivation Helpers

**Files:**

- Create: `src/lib/today.ts`
- Create: `src/lib/today.test.ts`

- [ ] Add helper tests for due actions, captured notes, and week counts.

Use fixed `now = new Date("2026-05-22T15:00:00")`. Test that:

- an action due `2026-05-21` and an action due `2026-05-22` are included by `actionsDueByToday`;
- an action due `2026-05-23` and an action with `dueDate: null` are excluded;
- notes created on `2026-05-22` local time are included by `notesCapturedToday`;
- `buildWorkWeekActivity` returns five weekday entries and combines capture and due-action counts.

- [ ] Implement `src/lib/today.ts`.

Export:

```ts
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

export function actionsDueByToday(actions: ActionReviewItem[], now = new Date()): ActionReviewItem[];
export function notesCapturedToday(notes: NoteListItem[], now = new Date()): NoteListItem[];
export function buildWorkWeekActivity(notes: NoteListItem[], actions: ActionReviewItem[], now = new Date()): WeekActivity[];
export function formatTodayHeading(now = new Date()): string;
export function formatShortTime(value: string): string;
```

Implementation notes:

- Treat invalid dates as absent.
- Compare dates with local year/month/day boundaries.
- Sort due actions by due date, then `createdAt`.
- Build Monday-Friday around the week containing `now`.

- [ ] Run `npm test -- src/lib/today.test.ts`.

Expected: Today helper tests pass.

## Task 2: Store And Shell Navigation

**Files:**

- Modify: `src/lib/stores/inbox.ts`
- Modify: `src/lib/stores/inbox.test.ts`
- Modify: `src/lib/components/AppShell.svelte`
- Modify: `src/lib/components/AppShell.test.ts`

- [ ] Add failing tests.

Store test:

- `showToday()` sets `viewMode` to `today`.
- It calls `listInbox` with `includeArchived: false`.
- It calls `listSuggestedActions`.
- It filters out archived notes.

AppShell test:

- rendering with `activeView: "today"` marks Today with `aria-current="page"`;
- clicking Today emits `navigate` detail `"today"`.

- [ ] Implement store and shell changes.

Required code shape:

- Change `InboxViewMode` to include `"today"`.
- In `loadInbox`, use archive filtering only when `mode === "archive"`; otherwise keep non-archived notes.
- Add:

```ts
async function showToday(): Promise<void> {
  viewMode.set("today");
  filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
  await loadInbox();
  await loadSuggestedActions();
}
```

- Return `showToday`.
- Replace the sidebar Today `<a>` with a `<button>` matching Inbox/Actions/Archive navigation.

- [ ] Run targeted tests.

```powershell
npm test -- src/lib/stores/inbox.test.ts src/lib/components/AppShell.test.ts
```

Expected: targeted tests pass.

## Task 3: TodayView Component And Route Wiring

**Files:**

- Create: `src/lib/components/TodayView.svelte`
- Create: `src/lib/components/TodayView.test.ts`
- Modify: `src/routes/+page.svelte`

- [ ] Add component tests.

Use Testing Library Svelte to verify:

- due action text and source note title render;
- captured-today note title renders;
- week day labels render;
- clicking Accept on a due action dispatches `accept` with the action id;
- clicking a due action source row dispatches `openNote` with the note id.

- [ ] Implement `TodayView.svelte`.

Props:

```ts
type Props = {
  notes: NoteListItem[];
  actions: ActionReviewItem[];
  loadingNotes?: boolean;
  loadingActions?: boolean;
  busyActionId?: string | null;
  now?: Date;
};
```

Events:

```ts
openNote: string;
accept: string;
```

Rendering:

- Full-height `<section class="today-view" aria-label="Today">`.
- Header with `Today`, `formatTodayHeading(now)`, and counts.
- Due section uses `actionsDueByToday`.
- Captured section uses `notesCapturedToday`.
- Week section uses `buildWorkWeekActivity`.
- Accept button must not trigger row open.

- [ ] Wire `src/routes/+page.svelte`.

Import `TodayView`. In `navigatePrimary`, handle `"today"` with `await workNotes.showToday()`.

When `$viewMode === "today"`, render:

```svelte
<TodayView
  notes={$inbox}
  actions={$suggestedActions}
  loadingNotes={$loadingInbox}
  loadingActions={$loadingSuggestedActions}
  busyActionId={$busyActionId}
  on:openNote={(event) => void openNoteFromToday(event.detail)}
  on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
/>
```

Add `openNoteFromToday(noteId)` to call `await workNotes.showInbox(); await workNotes.selectNote(noteId);`.

- [ ] Run targeted frontend tests.

```powershell
npm test -- src/lib/today.test.ts src/lib/components/TodayView.test.ts src/lib/components/AppShell.test.ts src/lib/stores/inbox.test.ts
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

Expected: only intended Today-view files changed, and no whitespace errors.
