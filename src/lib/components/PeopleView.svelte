<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { dueTone, formatActionDue } from "$lib/action-groups";
  import {
    avatarHue,
    buildPeople,
    buildPersonDetail,
    formatPersonWhen,
    matchesPersonSearch,
    type PersonAction,
  } from "$lib/people";
  import type { ActionReviewItem, NoteListItem, ParseStatus, ReviewStatus } from "$lib/types";

  type Props = {
    notes: NoteListItem[];
    actions: ActionReviewItem[];
    loadingNotes?: boolean;
    loadingActions?: boolean;
  };

  let { notes, actions, loadingNotes = false, loadingActions = false }: Props = $props();

  let query = $state("");
  let selectedKey = $state<string | null>(null);

  const people = $derived(buildPeople(notes, actions));
  const visiblePeople = $derived(people.filter((person) => matchesPersonSearch(person, query)));
  const activeKey = $derived(selectedKey ?? people[0]?.key ?? null);
  const detail = $derived(activeKey ? buildPersonDetail(activeKey, notes, actions) : null);
  const loading = $derived(loadingNotes || loadingActions);

  const dispatch = createEventDispatcher<{
    openNote: string;
  }>();

  $effect(() => {
    if (people.length === 0) {
      selectedKey = null;
      return;
    }

    if (!selectedKey || !people.some((person) => person.key === selectedKey)) {
      selectedKey = people[0].key;
    }
  });

  function personRowLabel(person: { name: string; noteCount: number; actionCount: number; lastInteractionAt: string | null }): string {
    return [
      `Select ${person.name}`,
      noteCountLabel(person.noteCount),
      actionCountLabel(person.actionCount),
      `last ${formatPersonWhen(person.lastInteractionAt)}`,
    ].join(", ");
  }

  function noteCountLabel(count: number): string {
    return `${count} ${count === 1 ? "note" : "notes"}`;
  }

  function actionCountLabel(count: number): string {
    return `${count} ${count === 1 ? "open action" : "open actions"}`;
  }

  function sourceTitle(action: PersonAction): string {
    return action.sourceNote?.title ?? action.noteTitle;
  }

  function openActionSource(action: PersonAction): void {
    dispatch("openNote", action.sourceNote?.id ?? action.noteId);
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
</script>

<section class="people-view" aria-label="People">
  <section class="people-list" aria-label="People list">
    <header class="people-list-head">
      <div class="head-row">
        <h1>People</h1>
        <span class="load-state">{loading ? "Loading" : `${people.length} tracked`}</span>
      </div>

      <label class="search-row">
        <span aria-hidden="true">Search</span>
        <input aria-label="Search people" placeholder="Find a person" bind:value={query} />
        <kbd>/</kbd>
      </label>
    </header>

    <div class="people-scroll">
      {#if visiblePeople.length === 0}
        <div class="empty-state">
          <strong>{people.length === 0 ? "No people yet" : "No people match"}</strong>
          <span>{people.length === 0 ? "Person tags and action owners will appear here." : "Try another name."}</span>
        </div>
      {:else}
        {#each visiblePeople as person}
          <button
            class="person-row"
            class:selected={activeKey === person.key}
            type="button"
            aria-current={activeKey === person.key ? "page" : undefined}
            aria-label={personRowLabel(person)}
            onclick={() => (selectedKey = person.key)}
          >
            <span
              class="person-avatar"
              style={`--avatar-hue: ${avatarHue(person.name)}deg`}
              aria-hidden="true"
            >
              {person.name.charAt(0).toLocaleUpperCase()}
            </span>
            <span class="person-body">
              <span class="person-name">{person.name}</span>
              <span class="person-meta">
                {noteCountLabel(person.noteCount)}
                <span aria-hidden="true">-</span>
                last {formatPersonWhen(person.lastInteractionAt)}
              </span>
            </span>
            {#if person.actionCount > 0}
              <span class="person-pending">{person.actionCount}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </section>

  <section class="person-detail" aria-label="Selected person">
    {#if !detail}
      <div class="detail-empty">
        <div class="empty-mark">WN</div>
        <h2>No person selected</h2>
        <p>Person tags and assigned suggested actions will fill this view.</p>
      </div>
    {:else}
      <header class="detail-head">
        <div class="detail-head-meta">
          <span
            class="person-avatar lg"
            style={`--avatar-hue: ${avatarHue(detail.person.name)}deg`}
            aria-hidden="true"
          >
            {detail.person.name.charAt(0).toLocaleUpperCase()}
          </span>
          <div>
            <h2>{detail.person.name}</h2>
            <p>
              {noteCountLabel(detail.person.noteCount)}
              <span aria-hidden="true">-</span>
              last interaction {formatPersonWhen(detail.person.lastInteractionAt)}
            </p>
          </div>
        </div>
        <span class="load-state">{loading ? "Loading" : actionCountLabel(detail.person.actionCount)}</span>
      </header>

      <div class="detail-scroll">
        <div class="person-cols">
          <section class="person-col" aria-label={`You owe ${detail.person.name}`}>
            <div class="section-head">
              <h3>You owe {detail.person.name}</h3>
              <span>{detail.youOwe.length}</span>
            </div>

            {#if detail.youOwe.length === 0}
              <div class="person-empty">Nothing on your plate.</div>
            {:else}
              <div class="action-list">
                {#each detail.youOwe as action}
                  {@const due = formatActionDue(action.dueDate)}
                  <button
                    class="action-row"
                    type="button"
                    aria-label={`Open note: ${action.text}`}
                    onclick={() => openActionSource(action)}
                  >
                    <span class="action-check" aria-hidden="true"></span>
                    <span class="action-body">
                      <span class="action-text">{action.text}</span>
                      <span class="action-meta">
                        {#if due}<span class={`due tone-${dueTone(action.dueDate)}`}>{due}</span>{/if}
                        <span>from "{sourceTitle(action)}"</span>
                      </span>
                    </span>
                  </button>
                {/each}
              </div>
            {/if}
          </section>

          <section class="person-col" aria-label={`${detail.person.name} owes you`}>
            <div class="section-head">
              <h3>{detail.person.name} owes you</h3>
              <span>{detail.theyOwe.length}</span>
            </div>

            {#if detail.theyOwe.length === 0}
              <div class="person-empty">Nothing outstanding.</div>
            {:else}
              <div class="action-list">
                {#each detail.theyOwe as action}
                  {@const due = formatActionDue(action.dueDate)}
                  <button
                    class="action-row"
                    type="button"
                    aria-label={`Open note: ${action.text}`}
                    onclick={() => openActionSource(action)}
                  >
                    <span class="action-bullet" aria-hidden="true"></span>
                    <span class="action-body">
                      <span class="action-text">{action.text}</span>
                      <span class="action-meta">
                        {#if due}<span class={`due tone-${dueTone(action.dueDate)}`}>{due}</span>{/if}
                        <span>from "{sourceTitle(action)}"</span>
                      </span>
                    </span>
                  </button>
                {/each}
              </div>
            {/if}
          </section>
        </div>

        <section class="notes-section" aria-label="Recent notes">
          <div class="section-head">
            <h3>Recent notes</h3>
            <span>{detail.recentNotes.length}</span>
          </div>

          {#if detail.recentNotes.length === 0}
            <div class="person-empty">No notes tagged with {detail.person.name} yet.</div>
          {:else}
            <div class="person-notes">
              {#each detail.recentNotes as note}
                <button
                  class="person-note"
                  type="button"
                  aria-label={`Open note: ${note.title}`}
                  onclick={() => dispatch("openNote", note.id)}
                >
                  <span
                    class={`status-dot ${statusClass(note.parseStatus, note.reviewStatus)}`}
                    title={statusLabel(note.parseStatus, note.reviewStatus)}
                  ></span>
                  <span class="note-body">
                    <span class="note-title">{note.title}</span>
                    <span class="note-meta">{formatPersonWhen(note.createdAt)}</span>
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        </section>
      </div>
    {/if}
  </section>
</section>

<style>
  .people-view {
    display: grid;
    grid-template-columns: 320px minmax(0, 1fr);
    min-width: 0;
    min-height: 100vh;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
  }

  .people-list {
    display: flex;
    min-width: 0;
    min-height: 100vh;
    flex-direction: column;
    border-right: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  .people-list-head {
    display: grid;
    gap: 10px;
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 92%, var(--color-app-bg));
  }

  .head-row,
  .search-row,
  .person-meta,
  .detail-head,
  .detail-head-meta,
  .section-head,
  .action-meta {
    display: flex;
    align-items: center;
  }

  .head-row,
  .detail-head {
    justify-content: space-between;
    gap: 12px;
  }

  h1,
  h2,
  h3,
  p {
    margin: 0;
  }

  h1 {
    color: var(--color-text-primary);
    font-size: 16px;
    font-weight: 750;
    line-height: 1.2;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 17px;
    font-weight: 750;
    line-height: 1.2;
  }

  h3 {
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.07em;
    line-height: 1.2;
    text-transform: uppercase;
  }

  .load-state,
  .person-meta,
  .detail-head p,
  .action-meta,
  .note-meta,
  .person-empty,
  .empty-state span,
  .detail-empty p {
    color: var(--color-text-muted);
    font-size: 11.5px;
  }

  .load-state,
  .person-pending,
  .section-head span,
  .due,
  .note-meta {
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  }

  .search-row {
    gap: 8px;
    min-height: 31px;
    padding: 0 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-text-muted);
    background: var(--color-surface-input);
  }

  .search-row:focus-within {
    border-color: var(--color-accent-primary);
    color: var(--color-text-primary);
  }

  .search-row input {
    flex: 1;
    min-width: 0;
    border: 0;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    outline: none;
  }

  kbd {
    display: inline-grid;
    min-width: 16px;
    height: 17px;
    place-items: center;
    padding: 0 4px;
    border: 1px solid var(--color-border-default);
    border-bottom-width: 2px;
    border-radius: 4px;
    color: var(--color-text-muted);
    background: var(--color-surface-2);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
  }

  .people-scroll {
    flex: 1;
    overflow: auto;
    padding: 2px 0;
  }

  .person-row {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) auto;
    align-items: center;
    gap: 11px;
    width: 100%;
    min-height: 54px;
    padding: 9px 14px;
    border: 0;
    border-left: 2px solid transparent;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .person-row + .person-row {
    border-top: 1px solid var(--color-border-default);
  }

  .person-row:hover,
  .person-row.selected {
    background: var(--color-surface-2);
  }

  .person-row.selected {
    border-left-color: var(--color-accent-primary);
  }

  .person-row:focus-visible,
  .action-row:focus-visible,
  .person-note:focus-visible {
    outline: 1px solid var(--color-accent-primary);
    outline-offset: -2px;
  }

  .person-avatar {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border-radius: 999px;
    color: var(--color-text-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 48%, var(--color-surface-2));
    filter: hue-rotate(var(--avatar-hue, 0deg));
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12px;
    font-weight: 800;
  }

  .person-avatar.lg {
    width: 38px;
    height: 38px;
    font-size: 15px;
  }

  .person-body,
  .action-body,
  .note-body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .person-name,
  .action-text,
  .note-title {
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .person-meta,
  .action-meta {
    gap: 6px;
    min-width: 0;
  }

  .person-pending,
  .section-head span {
    display: inline-grid;
    min-width: 20px;
    min-height: 20px;
    place-items: center;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 12%, var(--color-surface-2));
    font-size: 11px;
    font-weight: 750;
  }

  .person-detail {
    min-width: 0;
    min-height: 100vh;
    background: var(--color-app-bg);
  }

  .detail-head {
    min-height: 64px;
    padding: 12px 22px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-app-bg) 92%, var(--color-surface-1));
  }

  .detail-head-meta {
    gap: 11px;
    min-width: 0;
  }

  .detail-scroll {
    max-width: 820px;
    padding: 22px 28px 60px;
    overflow: auto;
  }

  .person-cols {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 24px;
  }

  .person-col,
  .notes-section {
    display: grid;
    align-content: start;
    gap: 9px;
    min-width: 0;
  }

  .notes-section {
    margin-top: 28px;
  }

  .section-head {
    gap: 8px;
  }

  .action-list,
  .person-notes {
    display: grid;
    gap: 1px;
  }

  .action-row,
  .person-note {
    display: grid;
    width: 100%;
    border: 0;
    border-radius: 7px;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .action-row {
    grid-template-columns: 20px minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    padding: 9px 10px;
  }

  .person-note {
    grid-template-columns: 14px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
  }

  .action-row:hover,
  .person-note:hover {
    background: var(--color-surface-2);
  }

  .action-check {
    width: 16px;
    height: 16px;
    margin-top: 1px;
    border: 1.5px solid var(--color-border-default);
    border-radius: 4px;
    background: transparent;
  }

  .action-bullet,
  .status-dot {
    border-radius: 999px;
  }

  .action-bullet {
    width: 6px;
    height: 6px;
    margin: 8px 0 0 5px;
    background: var(--color-text-muted);
  }

  .action-meta {
    flex-wrap: wrap;
    margin-top: 2px;
    line-height: 1.3;
  }

  .tone-error {
    color: var(--color-status-error);
  }

  .tone-warning {
    color: var(--color-status-warning);
  }

  .tone-muted {
    color: var(--color-text-muted);
  }

  .status-dot {
    width: 7px;
    height: 7px;
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

  .person-empty,
  .empty-state {
    padding: 8px 10px;
  }

  .empty-state,
  .detail-empty {
    display: grid;
    place-items: center;
    gap: 6px;
    min-height: 220px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-state strong,
  .detail-empty h2 {
    color: var(--color-text-primary);
    font-size: 14px;
  }

  .empty-mark {
    display: grid;
    width: 44px;
    height: 44px;
    place-items: center;
    border-radius: 8px;
    color: var(--color-accent-primary);
    background: var(--color-surface-2);
    font-weight: 900;
  }

  @media (max-width: 920px) {
    .people-view {
      grid-template-columns: 1fr;
    }

    .people-list {
      min-height: auto;
      border-right: 0;
      border-bottom: 1px solid var(--color-border-default);
    }

    .people-scroll {
      max-height: 260px;
    }

    .person-cols {
      grid-template-columns: 1fr;
    }
  }
</style>
