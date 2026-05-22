# Actions View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real Actions primary view that lists suggested actions as grouped, searchable first-class rows and opens the source note in the existing detail panel.

**Architecture:** Reuse the existing `list_suggested_actions`, `accept_action_item`, `dismiss_action_item`, and `get_note` flow. Keep view sequencing in `src/lib/stores/inbox.ts`, Tauri calls in `src/lib/api.ts`, and render-only behavior in Svelte components.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Testing Library, Tauri command API, Rust backend already present.

---

## File Structure

- Create `src/lib/action-groups.ts`: pure helpers for due buckets, due labels, and action search.
- Create `src/lib/action-groups.test.ts`: focused unit tests for grouping/search behavior.
- Create `src/lib/components/ActionsList.svelte`: grouped action-list UI.
- Create `src/lib/components/ActionsList.test.ts`: component interaction tests.
- Modify `src/lib/stores/inbox.ts`: extend view mode with `actions` and add `showActions`.
- Modify `src/lib/stores/inbox.test.ts`: store tests for Actions view entry and selection behavior.
- Modify `src/lib/components/AppShell.svelte`: make Actions a real nav button and count suggested actions.
- Modify `src/lib/components/AppShell.test.ts`: active Actions and navigation event coverage.
- Modify `src/routes/+page.svelte`: render `ActionsList` in Actions view and hide `ReviewQueue` while there.

## Task 1: Add Action Grouping Helpers

**Files:**
- Create: `src/lib/action-groups.ts`
- Create: `src/lib/action-groups.test.ts`

- [ ] **Step 1: Write failing helper tests**

Create `src/lib/action-groups.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import type { ActionReviewItem } from "$lib/types";
import { actionMatchesSearch, groupActionsByDueBucket } from "./action-groups";

const now = new Date("2026-05-22T12:00:00.000Z");

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Maria dashboard owner",
    text: "Decide whether to rebuild the dashboard",
    owner: "me",
    dueDate: null,
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

describe("groupActionsByDueBucket", () => {
  it("groups actions into due buckets in design order", () => {
    const groups = groupActionsByDueBucket(
      [
        action({ id: "no-date", dueDate: null }),
        action({ id: "overdue", dueDate: "2026-05-21" }),
        action({ id: "today", dueDate: "2026-05-22" }),
        action({ id: "week", dueDate: "2026-05-25" }),
        action({ id: "later", dueDate: "2026-06-10" }),
      ],
      now,
    );

    expect(groups.map((group) => [group.label, group.actions.map((item) => item.id)])).toEqual([
      ["Overdue", ["overdue"]],
      ["Today", ["today"]],
      ["This week", ["week"]],
      ["Later", ["later"]],
      ["No date", ["no-date"]],
    ]);
  });
});

describe("actionMatchesSearch", () => {
  it("matches text, owner, due date, and source note title", () => {
    const item = action({
      owner: "Maria",
      dueDate: "2026-05-25",
      noteTitle: "Q3 metrics migration",
      text: "Forward failing queries",
    });

    expect(actionMatchesSearch(item, "failing")).toBe(true);
    expect(actionMatchesSearch(item, "maria")).toBe(true);
    expect(actionMatchesSearch(item, "2026-05-25")).toBe(true);
    expect(actionMatchesSearch(item, "metrics")).toBe(true);
    expect(actionMatchesSearch(item, "payroll")).toBe(false);
  });
});
```

- [ ] **Step 2: Run the tests and verify they fail**

Run:

```powershell
npm test -- action-groups
```

Expected: fail because `src/lib/action-groups.ts` does not exist.

- [ ] **Step 3: Implement helpers**

Create `src/lib/action-groups.ts`:

```ts
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

export function actionMatchesSearch(action: ActionReviewItem, query: string): boolean {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) {
    return true;
  }

  return [action.text, action.owner, action.dueDate, action.noteTitle]
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

export function dueTone(dueDate: string | null | undefined, now = new Date()): "error" | "warning" | "muted" {
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

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function dayDifference(date: Date, now: Date): number {
  const start = Date.UTC(now.getFullYear(), now.getMonth(), now.getDate());
  const target = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  return Math.round((target - start) / 86_400_000);
}
```

- [ ] **Step 4: Run helper tests**

Run:

```powershell
npm test -- action-groups
```

Expected: pass.

## Task 2: Add ActionsList Component

**Files:**
- Create: `src/lib/components/ActionsList.svelte`
- Create: `src/lib/components/ActionsList.test.ts`

- [ ] **Step 1: Write failing component tests**

Create `src/lib/components/ActionsList.test.ts`:

```ts
// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { ActionReviewItem } from "$lib/types";
import ActionsList from "./ActionsList.svelte";

afterEach(() => cleanup());

function action(overrides: Partial<ActionReviewItem> = {}): ActionReviewItem {
  return {
    id: "action-1",
    noteId: "note-1",
    noteTitle: "Maria dashboard owner",
    text: "Decide whether to rebuild the dashboard",
    owner: "me",
    dueDate: "2026-05-22",
    confidence: 0.82,
    createdAt: "2026-05-20T13:42:00.000Z",
    ...overrides,
  };
}

describe("ActionsList", () => {
  it("groups suggested actions and opens the source note", async () => {
    const select = vi.fn();
    render(ActionsList, {
      props: {
        actions: [
          action({ id: "today", noteId: "note-today", text: "Send scope review", dueDate: "2026-05-22" }),
          action({ id: "later", noteId: "note-later", text: "Find design partner", dueDate: "2026-06-10" }),
        ],
        selectedNoteId: "note-today",
        now: new Date("2026-05-22T12:00:00.000Z"),
      },
      events: { select },
    });

    expect(screen.getByText("Today")).toBeTruthy();
    expect(screen.getByText("Later")).toBeTruthy();

    await fireEvent.click(screen.getByRole("button", { name: /Find design partner/ }));

    expect(select.mock.calls[0][0].detail).toBe("note-later");
  });

  it("filters actions by owner and dispatches lifecycle events", async () => {
    const accept = vi.fn();
    const dismiss = vi.fn();
    const select = vi.fn();
    render(ActionsList, {
      props: {
        actions: [
          action({ id: "keep", owner: "Maria", text: "Forward failing queries" }),
          action({ id: "hide", owner: "Sam", text: "Update new-hire deck" }),
        ],
        now: new Date("2026-05-22T12:00:00.000Z"),
      },
      events: { accept, dismiss, select },
    });

    await fireEvent.input(screen.getByRole("textbox", { name: "Search actions" }), {
      target: { value: "maria" },
    });

    expect(screen.getByText("Forward failing queries")).toBeTruthy();
    expect(screen.queryByText("Update new-hire deck")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Accept action: Forward failing queries" }));
    await fireEvent.click(screen.getByRole("button", { name: "Dismiss action: Forward failing queries" }));

    expect(accept.mock.calls[0][0].detail).toBe("keep");
    expect(dismiss.mock.calls[0][0].detail).toBe("keep");
    expect(select).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run the component tests and verify they fail**

Run:

```powershell
npm test -- ActionsList
```

Expected: fail because `ActionsList.svelte` does not exist.

- [ ] **Step 3: Implement the component**

Create `src/lib/components/ActionsList.svelte`:

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import {
    actionMatchesSearch,
    dueTone,
    formatActionDue,
    groupActionsByDueBucket,
  } from "$lib/action-groups";
  import type { ActionReviewItem } from "$lib/types";

  type Props = {
    actions: ActionReviewItem[];
    selectedNoteId?: string | null;
    busyActionId?: string | null;
    loading?: boolean;
    now?: Date;
  };

  let {
    actions,
    selectedNoteId = null,
    busyActionId = null,
    loading = false,
    now = new Date(),
  }: Props = $props();

  let query = $state("");

  const filteredActions = $derived(actions.filter((action) => actionMatchesSearch(action, query)));
  const groups = $derived(groupActionsByDueBucket(filteredActions, now));

  const dispatch = createEventDispatcher<{
    select: string;
    accept: string;
    dismiss: string;
  }>();

  function sourceLabel(title: string): string {
    return title.length > 38 ? `${title.slice(0, 38)}...` : title;
  }
</script>

<section class="actions-list" aria-label="Actions">
  <header class="actions-header">
    <div class="mode-row">
      <div class="mode-toggle" aria-label="Actions mode">
        <button class="active" type="button">Actions</button>
        <span>{filteredActions.length}</span>
      </div>
      <span class="load-state">{loading ? "Loading" : `${filteredActions.length} open`}</span>
    </div>

    <label class="search-row">
      <span aria-hidden="true">Search</span>
      <input
        aria-label="Search actions"
        placeholder="Search actions, owners"
        bind:value={query}
      />
      <kbd>/</kbd>
    </label>
  </header>

  <div class="actions-scroll">
    {#if groups.length === 0}
      <div class="empty-state">
        <div class="empty-mark">WN</div>
        <h2>No suggested actions</h2>
        <p>Parser suggestions will appear here after capture.</p>
      </div>
    {/if}

    {#each groups as group}
      <section class="action-group" aria-label={group.label}>
        <div class="group-head">
          <span>{group.label}</span>
          <strong>{group.actions.length}</strong>
        </div>

        {#each group.actions as action}
          {@const due = formatActionDue(action.dueDate, now)}
          <article class:selected={selectedNoteId === action.noteId} class="action-row">
            <button
              class="accept-button"
              type="button"
              aria-label={`Accept action: ${action.text}`}
              disabled={loading || busyActionId === action.id}
              onclick={() => dispatch("accept", action.id)}
            >
              OK
            </button>
            <button
              class="action-body"
              type="button"
              aria-label={`Open note: ${action.text}`}
              onclick={() => dispatch("select", action.noteId)}
            >
              <span class="action-text">{action.text}</span>
              <span class="action-meta">
                {#if action.owner}<span class="owner">@{action.owner}</span>{/if}
                {#if due}<span class={`due tone-${dueTone(action.dueDate, now)}`}>{due}</span>{/if}
                <span class="from">from "{sourceLabel(action.noteTitle)}"</span>
              </span>
            </button>
            <button
              class="dismiss-button"
              type="button"
              aria-label={`Dismiss action: ${action.text}`}
              disabled={loading || busyActionId === action.id}
              onclick={() => dispatch("dismiss", action.id)}
            >
              x
            </button>
          </article>
        {/each}
      </section>
    {/each}
  </div>
</section>

<style>
  .actions-list {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 100vh;
    border-right: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .actions-header {
    display: grid;
    gap: 9px;
    padding: 12px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 92%, var(--color-app-bg));
  }

  .mode-row,
  .search-row,
  .action-meta,
  .group-head {
    display: flex;
    align-items: center;
  }

  .mode-row {
    justify-content: space-between;
    gap: 10px;
  }

  .mode-toggle {
    display: inline-flex;
    gap: 3px;
    padding: 2px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-2);
  }

  .mode-toggle button,
  .mode-toggle span {
    min-height: 24px;
    border: 0;
    border-radius: 5px;
    padding: 0 9px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
    font-weight: 750;
  }

  .mode-toggle span {
    display: grid;
    place-items: center;
    color: var(--color-accent-primary);
  }

  .load-state,
  .group-head,
  .action-meta,
  .empty-state p {
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .search-row {
    gap: 8px;
    min-height: 32px;
    padding: 0 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-input);
  }

  .search-row input {
    min-width: 0;
    flex: 1;
    border: 0;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    outline: none;
  }

  .actions-scroll {
    flex: 1;
    overflow: auto;
    padding: 8px;
  }

  .action-group {
    display: grid;
    gap: 2px;
    margin-bottom: 10px;
  }

  .group-head {
    gap: 6px;
    padding: 7px 8px 5px;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .group-head strong {
    min-width: 18px;
    border-radius: 999px;
    color: var(--color-text-muted);
    background: var(--color-surface-2);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 10px;
    text-align: center;
  }

  .action-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 8px;
    align-items: flex-start;
    padding: 8px;
    border-radius: 7px;
  }

  .action-row:hover,
  .action-row.selected {
    background: var(--color-surface-2);
  }

  .accept-button,
  .dismiss-button,
  .action-body {
    border: 0;
    background: transparent;
    font: inherit;
    cursor: pointer;
  }

  .accept-button,
  .dismiss-button {
    min-width: 24px;
    min-height: 24px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-muted);
    background: var(--color-surface-input);
    font-size: 10px;
    font-weight: 800;
  }

  .accept-button:hover {
    border-color: var(--color-status-success);
    color: var(--color-status-success);
  }

  .dismiss-button:hover {
    border-color: var(--color-status-error);
    color: var(--color-status-error);
  }

  .action-body {
    display: grid;
    gap: 4px;
    min-width: 0;
    padding: 0;
    color: inherit;
    text-align: left;
  }

  .action-text {
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.35;
  }

  .action-meta {
    flex-wrap: wrap;
    gap: 6px;
    line-height: 1.25;
  }

  .owner {
    color: var(--color-accent-primary);
    font-weight: 750;
  }

  .due {
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  }

  .tone-error {
    color: var(--color-status-error);
  }

  .tone-warning {
    color: var(--color-status-warning);
  }

  .from {
    color: var(--color-text-muted);
    font-style: italic;
  }

  .empty-state {
    display: grid;
    place-items: center;
    gap: 8px;
    min-height: 240px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-mark {
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border-radius: 8px;
    color: var(--color-accent-primary);
    background: var(--color-surface-2);
    font-weight: 900;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 15px;
  }
</style>
```

- [ ] **Step 4: Run component tests**

Run:

```powershell
npm test -- ActionsList action-groups
```

Expected: pass.

## Task 3: Add Actions View Store Flow

**Files:**
- Modify: `src/lib/stores/inbox.ts`
- Modify: `src/lib/stores/inbox.test.ts`

- [ ] **Step 1: Write failing store test**

Add this test to `src/lib/stores/inbox.test.ts`:

```ts
it("enters actions view, refreshes suggested actions, and selects the first source note", async () => {
  const firstAction = reviewItem({ id: "action-1", noteId: "note-action-1", noteTitle: "Action source" });
  const source = note({ id: "note-action-1", title: "Action source" });
  const api = testApi({
    listSuggestedActions: vi.fn().mockResolvedValue([firstAction]),
    getNote: vi.fn().mockResolvedValue({ ...source, actionItems: [] }),
  });
  const store = createWorkNotesStore(api);

  await store.showActions();

  expect(get(store.viewMode)).toBe("actions");
  expect(get(store.suggestedActions)).toEqual([firstAction]);
  expect(get(store.selectedNote)?.id).toBe("note-action-1");
  expect(api.listSuggestedActions).toHaveBeenCalledTimes(1);
});
```

- [ ] **Step 2: Run the store test and verify it fails**

Run:

```powershell
npm test -- inbox
```

Expected: fail because `showActions` does not exist and `InboxViewMode` does not include `actions`.

- [ ] **Step 3: Implement store flow**

In `src/lib/stores/inbox.ts`, update the type:

```ts
export type InboxViewMode = "inbox" | "archive" | "actions";
```

Update the derived filter to keep archived notes excluded outside Archive:

```ts
includeArchived: $viewMode === "archive",
```

Add `showActions` near `showArchive`:

```ts
async function showActions(): Promise<void> {
  viewMode.set("actions");
  filters.update((current) => createInboxFilters({ ...current, includeArchived: false }));
  await loadSuggestedActions();

  const actions = get(suggestedActions);
  const currentSelected = get(selectedNote);
  if (
    actions.length > 0 &&
    (!currentSelected || !actions.some((action) => action.noteId === currentSelected.id))
  ) {
    await selectNote(actions[0].noteId);
  }
}
```

Return `showActions` from the store object.

- [ ] **Step 4: Run store tests**

Run:

```powershell
npm test -- inbox
```

Expected: pass.

## Task 4: Wire Sidebar And Route

**Files:**
- Modify: `src/lib/components/AppShell.svelte`
- Modify: `src/lib/components/AppShell.test.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Write failing AppShell test**

Add this test to `src/lib/components/AppShell.test.ts`:

```ts
it("marks actions active and emits actions navigation", async () => {
  const navigate = vi.fn();

  render(AppShell, {
    props: {
      title: "Work Notes",
      subtitle: "Fast capture",
      workspace: "Local workspace",
      metrics: [
        { label: "Inbox", value: "3" },
        { label: "Needs review", value: "2" },
      ],
      activeView: "actions",
    },
    events: { navigate },
  });

  const actions = screen.getByRole("button", { name: "Actions" });
  await fireEvent.click(actions);

  expect(actions.getAttribute("aria-current")).toBe("page");
  expect(navigate.mock.calls[0][0].detail).toBe("actions");
});
```

- [ ] **Step 2: Run AppShell test and verify it fails**

Run:

```powershell
npm test -- AppShell
```

Expected: fail because Actions is still a link and does not emit `navigate`.

- [ ] **Step 3: Implement AppShell Actions nav**

In `src/lib/components/AppShell.svelte`, convert the existing Actions link to a button:

```svelte
<button
  class:active={activeView === "actions"}
  type="button"
  aria-current={activeView === "actions" ? "page" : undefined}
  aria-label="Actions"
  onclick={() => navigate("actions")}
>
  <span aria-hidden="true">A</span>
  <span>Actions</span>
  <strong aria-hidden="true">{metrics.find((metric) => metric.label === "Needs review")?.value ?? "0"}</strong>
</button>
```

Ensure `.nav-stack button.active` styling applies to Actions as it already does for Inbox and Archive.

- [ ] **Step 4: Wire route view selection**

In `src/routes/+page.svelte`, import `ActionsList`:

```ts
import ActionsList from "$lib/components/ActionsList.svelte";
```

Update `navigatePrimary`:

```ts
async function navigatePrimary(event: CustomEvent<InboxViewMode>) {
  if (event.detail === "archive") {
    await workNotes.showArchive();
    return;
  }

  if (event.detail === "actions") {
    await workNotes.showActions();
    return;
  }

  await workNotes.showInbox();
}
```

Render `ActionsList` instead of `InboxList` while Actions view is active:

```svelte
{#if $viewMode === "actions"}
  <ActionsList
    actions={$suggestedActions}
    selectedNoteId={$selectedNote?.id ?? null}
    busyActionId={$busyActionId}
    loading={$loadingSuggestedActions}
    on:select={(event) => void workNotes.selectNote(event.detail)}
    on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
    on:dismiss={(event) => void workNotes.dismissSuggestedAction(event.detail)}
  />
{:else}
  <InboxList
    items={$filteredInbox}
    filters={$filters}
    {selectedId}
    loading={$loadingInbox}
    viewMode={$viewMode}
    on:select={(event) => void workNotes.selectNote(event.detail)}
    on:filter={(event) => void workNotes.updateFilters(event.detail)}
  />
{/if}
```

Hide the top ReviewQueue in Actions view:

```svelte
{#if $viewMode !== "actions" && ($suggestedActions.length > 0 || $loadingSuggestedActions)}
```

- [ ] **Step 5: Run AppShell and Svelte checks**

Run:

```powershell
npm test -- AppShell
npm run check
```

Expected: pass.

## Task 5: Full Verification

**Files:**
- All files touched above.

- [ ] **Step 1: Run focused frontend tests**

Run:

```powershell
npm test -- action-groups ActionsList AppShell inbox
```

Expected: pass.

- [ ] **Step 2: Run full verification**

Run:

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```

Expected: pass.

- [ ] **Step 3: Browser smoke**

With the Vite dev server at `http://localhost:1420`, verify:

1. Click `Actions` in the sidebar.
2. Actions nav is active.
3. Action rows render in grouped sections.
4. Search filters action rows.
5. Clicking an action row opens the source note.
6. Accept or dismiss removes the action from the list after refresh.

- [ ] **Step 4: Commit**

Stage only the Actions View implementation and docs:

```powershell
git add docs/superpowers/specs/2026-05-22-actions-view-design.md docs/superpowers/plans/2026-05-22-actions-view-implementation.md
git add src/lib/action-groups.ts src/lib/action-groups.test.ts src/lib/components/ActionsList.svelte src/lib/components/ActionsList.test.ts
git add src/lib/components/AppShell.svelte src/lib/components/AppShell.test.ts src/lib/stores/inbox.ts src/lib/stores/inbox.test.ts src/routes/+page.svelte
git commit -m "feat: add actions view"
```

Leave `work-notes-handoff.zip` untracked.

