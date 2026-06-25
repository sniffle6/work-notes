<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { dueTone, formatActionDue } from "$lib/action-groups";
  import {
    actionsDueByToday,
    buildWorkWeekActivity,
    formatShortTime,
    formatTodayHeading,
    notesCapturedToday,
  } from "$lib/today";
  import type { ActionReviewItem, NoteListItem, ParseStatus, ReviewStatus } from "$lib/types";

  type Props = {
    notes: NoteListItem[];
    actions: ActionReviewItem[];
    loadingNotes?: boolean;
    loadingActions?: boolean;
    busyActionId?: string | null;
    now?: Date;
  };

  let {
    notes,
    actions,
    loadingNotes = false,
    loadingActions = false,
    busyActionId = null,
    now = new Date(),
  }: Props = $props();

  const dueActions = $derived(actionsDueByToday(actions, now));
  const capturedNotes = $derived(notesCapturedToday(notes, now));
  const weekActivity = $derived(buildWorkWeekActivity(notes, actions, now));
  const reviewCount = $derived(notes.filter((note) => note.reviewStatus === "needs_review").length);
  const summary = $derived(
    `${dueActions.length} ${dueActions.length === 1 ? "action" : "actions"} due - ${capturedNotes.length} captured - ${reviewCount} needs review`,
  );

  const dispatch = createEventDispatcher<{
    openNote: string;
    accept: string;
  }>();

  function sourceLabel(title: string): string {
    return title.length > 42 ? `${title.slice(0, 42)}...` : title;
  }

  function statusLabel(status: ParseStatus, review: ReviewStatus): string {
    if (status === "failed") return "Parse failed";
    if (status === "parsing") return "Parsing";
    if (status === "queued") return "Queued";
    if (review === "needs_review") return "Needs review";
    if (review === "reviewed") return "Reviewed";
    return "Captured";
  }

  function statusClass(status: ParseStatus, review: ReviewStatus): string {
    if (status === "failed") return "error";
    if (status === "parsing") return "info";
    if (status === "queued") return "neutral";
    if (review === "needs_review") return "warning";
    if (review === "reviewed") return "success reviewed";
    return "neutral";
  }

  function tagLabel(note: NoteListItem): string {
    return note.tags.slice(0, 2).map((tag) => `#${tag.name}`).join(" ");
  }

  function actionCountLabel(count: number): string {
    return `${count} ${count === 1 ? "action" : "actions"}`;
  }

  function capturedNoteLabel(note: NoteListItem): string {
    const parts = [
      `Open note: ${note.title}`,
      statusLabel(note.parseStatus, note.reviewStatus),
      formatShortTime(note.createdAt),
    ];

    if (note.suggestedActionItemCount > 0) {
      parts.push(actionCountLabel(note.suggestedActionItemCount));
    }

    return parts.join(", ");
  }
</script>

<section class="today-view" aria-label="Today">
  <header class="today-head">
    <div class="today-eyebrow">Today</div>
    <h1>{formatTodayHeading(now)}</h1>
    <p class="today-summary">{summary}</p>
  </header>

  <section class="today-section" aria-label="Due today">
    <div class="today-section-head">
      <h2>Due today</h2>
      <span class="today-section-count">{dueActions.length}</span>
      <span class="load-state">{loadingActions ? "Loading" : "Suggested"}</span>
    </div>

    <div class="today-actions">
      {#if dueActions.length === 0}
        <div class="empty-state">
          <strong>{loadingActions ? "Loading actions" : "No due actions"}</strong>
          <span>{loadingActions ? "Checking parser suggestions." : "Suggested work due today will appear here."}</span>
        </div>
      {:else}
        {#each dueActions as action}
          {@const due = formatActionDue(action.dueDate, now)}
          <div class="today-action">
            <button
              class="row-button"
              type="button"
              aria-label={`Open note: ${action.text}`}
              onclick={() => dispatch("openNote", action.noteId)}
            ></button>
            <button
              class="accept-button"
              type="button"
              aria-label={`Accept action: ${action.text}`}
              disabled={loadingActions || busyActionId === action.id}
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
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <section class="today-section" aria-label="Captured today">
    <div class="today-section-head">
      <h2>Captured today</h2>
      <span class="today-section-count">{capturedNotes.length}</span>
      <span class="load-state">{loadingNotes ? "Loading" : "Notes"}</span>
    </div>

    <div class="today-notes">
      {#if capturedNotes.length === 0}
        <div class="empty-state">
          <strong>{loadingNotes ? "Loading notes" : "No notes captured today"}</strong>
          <span>{loadingNotes ? "Refreshing the inbox." : "New captures from today will appear here."}</span>
        </div>
      {:else}
        {#each capturedNotes as note}
          <button
            class="today-note"
            type="button"
            aria-label={capturedNoteLabel(note)}
            onclick={() => dispatch("openNote", note.id)}
          >
            <span
              class={`status-dot ${statusClass(note.parseStatus, note.reviewStatus)}`}
              title={statusLabel(note.parseStatus, note.reviewStatus)}
            ></span>
            <span class="today-note-body">
              <span class="today-note-title">{note.title}</span>
              <span class="today-note-meta">
                <span>{formatShortTime(note.createdAt)}</span>
                {#if tagLabel(note)}
                  <span class="separator">-</span>
                  <span>{tagLabel(note)}</span>
                {/if}
              </span>
            </span>
            {#if note.suggestedActionItemCount > 0}
              <span class="today-note-actions">{actionCountLabel(note.suggestedActionItemCount)}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </section>

  <section class="today-section" aria-label="This week">
    <div class="today-section-head">
      <h2>This week</h2>
    </div>

    <div class="today-week">
      {#each weekActivity as day}
        <div class:active-day={day.isToday} class="today-day">
          <div class="today-day-label">{day.label}</div>
          <div class="today-day-dots" aria-hidden="true">
            {#each Array.from({ length: Math.min(day.totalCount, 5) }) as _}
              <span class="today-day-dot"></span>
            {/each}
          </div>
          <div class="today-day-count">{day.totalCount}</div>
        </div>
      {/each}
    </div>
  </section>
</section>

<style>
  .today-view {
    display: flex;
    flex-direction: column;
    gap: 26px;
    height: 100%;
    min-height: 0;
    padding: 28px 36px 60px;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    overflow: auto;
  }

  .today-head {
    display: grid;
    gap: 5px;
    max-width: 720px;
  }

  .today-eyebrow,
  .today-section-head h2 {
    color: var(--color-accent-primary);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    color: var(--color-text-primary);
    font-size: 26px;
    font-weight: 750;
    line-height: 1.1;
  }

  .today-date {
    color: var(--color-text-primary);
    font-size: 17px;
    font-weight: 650;
  }

  .today-summary,
  .load-state,
  .action-meta,
  .today-note-meta,
  .empty-state span {
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .today-summary,
  .load-state,
  .today-note-meta,
  .today-day-count,
  .due {
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  }

  .today-section {
    display: grid;
    gap: 10px;
    max-width: 720px;
  }

  .today-section-head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 22px;
  }

  .today-section-head h2 {
    color: var(--color-text-primary);
  }

  .today-section-count {
    display: inline-grid;
    min-width: 20px;
    place-items: center;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-text-muted);
    background: var(--color-surface-2);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
    font-weight: 750;
  }

  .load-state {
    margin-left: auto;
  }

  .today-actions,
  .today-notes {
    display: grid;
    gap: 2px;
  }

  .today-action {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    min-width: 0;
    padding: 9px 10px;
    border-radius: 7px;
  }

  .today-action:hover,
  .today-note:hover {
    background: var(--color-surface-2);
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

  .row-button:focus-visible,
  .today-note:focus-visible,
  .accept-button:focus-visible {
    outline: 1px solid var(--color-accent-primary);
    outline-offset: -2px;
  }

  .accept-button,
  .action-body {
    position: relative;
    z-index: 2;
  }

  .accept-button {
    min-width: 24px;
    min-height: 24px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-muted);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 10px;
    font-weight: 800;
    cursor: pointer;
  }

  .accept-button:hover {
    border-color: var(--color-status-success);
    color: var(--color-status-success);
  }

  .accept-button:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .action-body {
    display: grid;
    gap: 4px;
    min-width: 0;
    pointer-events: none;
  }

  .action-text {
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.35;
  }

  .action-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    line-height: 1.25;
  }

  .owner {
    color: var(--color-accent-primary);
    font-weight: 750;
  }

  .tone-error {
    color: var(--color-status-error);
  }

  .tone-warning {
    color: var(--color-status-warning);
  }

  .tone-muted,
  .from {
    color: var(--color-text-muted);
  }

  .from {
    font-style: italic;
  }

  .today-note {
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 46px;
    padding: 9px 10px;
    border: 0;
    border-radius: 7px;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--color-text-muted);
  }

  .status-dot.error {
    background: var(--color-status-error);
  }

  .status-dot.info {
    background: var(--color-accent-hot);
  }

  .status-dot.warning {
    background: var(--color-status-warning);
  }

  .status-dot.success {
    background: var(--color-status-success);
  }

  .status-dot.reviewed {
    border: 1.5px solid var(--color-status-success);
    background: transparent;
  }

  .today-note-body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .today-note-title {
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .today-note-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .today-note-actions {
    display: inline-flex;
    align-items: center;
    min-height: 20px;
    padding: 0 7px;
    border-radius: 999px;
    color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 12%, var(--color-surface-2));
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
    font-weight: 750;
    white-space: nowrap;
  }

  .separator {
    color: var(--color-border-strong);
  }

  .today-week {
    display: grid;
    grid-template-columns: repeat(5, minmax(70px, 1fr));
    gap: 8px;
  }

  .today-day {
    display: grid;
    gap: 6px;
    min-height: 78px;
    padding: 11px;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    text-align: center;
  }

  .today-day.active-day {
    border-color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 12%, var(--color-surface-1));
  }

  .today-day-label {
    color: var(--color-text-muted);
    font-size: 10.5px;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .today-day.active-day .today-day-label {
    color: var(--color-accent-primary);
  }

  .today-day-dots {
    display: flex;
    justify-content: center;
    gap: 2px;
    min-height: 6px;
  }

  .today-day-dot {
    width: 5px;
    height: 5px;
    border-radius: 999px;
    background: var(--color-text-muted);
  }

  .today-day.active-day .today-day-dot {
    background: var(--color-accent-primary);
  }

  .today-day-count {
    color: var(--color-text-primary);
    font-size: 14px;
    font-weight: 800;
  }

  .empty-state {
    display: grid;
    gap: 3px;
    min-height: 64px;
    padding: 14px 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    color: var(--color-text-muted);
    background: color-mix(in srgb, var(--color-surface-1) 82%, transparent);
  }

  .empty-state strong {
    color: var(--color-text-primary);
    font-size: 13px;
  }

  @media (max-width: 760px) {
    .today-view {
      padding: 20px 18px 40px;
    }

    .today-week {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
