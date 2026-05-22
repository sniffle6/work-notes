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

  const filteredActions = $derived(actions.filter((action) => actionMatchesSearch(action, query, now)));
  const groups = $derived(groupActionsByDueBucket(filteredActions, now));

  const dispatch = createEventDispatcher<{
    select: string;
    accept: string;
    dismiss: string;
  }>();

  function sourceLabel(title: string): string {
    return title.length > 38 ? `${title.slice(0, 38)}...` : title;
  }

  function selectSourceNote(noteId: string): void {
    dispatch("select", noteId);
  }
</script>

<section class="actions-list" aria-label="Actions">
  <header class="actions-header">
    <div class="mode-row">
      <div class="mode-toggle" aria-label="Actions mode">
        <span class="mode-label active">Actions</span>
        <span class="mode-count">{filteredActions.length}</span>
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
        <p>Nothing open right now.</p>
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
          <div
            class="action-row"
            class:selected={selectedNoteId === action.noteId}
          >
            <button
              class="row-button"
              type="button"
              aria-label={`Open note: ${action.text}`}
              onclick={() => selectSourceNote(action.noteId)}
            ></button>
            <button
              class="accept-button"
              type="button"
              aria-label={`Accept action: ${action.text}`}
              disabled={loading || busyActionId === action.id}
              onclick={() => dispatch("accept", action.id)}
            >
              OK
            </button>
            <div class="action-body">
              <span class="action-text">{action.text}</span>
              <span class="action-meta">
                {#if action.owner}<span class="owner">@{action.owner}</span>{/if}
                {#if due}<span class={`due tone-${dueTone(action.dueDate, now)}`}>{due}</span>{/if}
                <span class="from">from "{sourceLabel(action.noteTitle)}"</span>
              </span>
            </div>
            <button
              class="dismiss-button"
              type="button"
              aria-label={`Dismiss action: ${action.text}`}
              disabled={loading || busyActionId === action.id}
              onclick={() => dispatch("dismiss", action.id)}
            >
              x
            </button>
          </div>
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

  .mode-label,
  .mode-count {
    display: grid;
    place-items: center;
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

  .mode-count {
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
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 8px;
    align-items: flex-start;
    padding: 8px;
    border-radius: 7px;
    cursor: pointer;
  }

  .row-button {
    position: absolute;
    z-index: 1;
    inset: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    cursor: pointer;
  }

  .row-button:focus-visible {
    outline: 1px solid var(--color-accent-primary);
    outline-offset: -2px;
  }

  .action-row:hover,
  .action-row.selected {
    background: var(--color-surface-2);
  }

  .accept-button,
  .dismiss-button,
  .action-body {
    position: relative;
    z-index: 2;
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
    pointer-events: none;
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
