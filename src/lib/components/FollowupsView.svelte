<script lang="ts">
  import Check from "@lucide/svelte/icons/check";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import X from "@lucide/svelte/icons/x";
  import { createEventDispatcher } from "svelte";
  import { describeDue, groupFollowupsByLane, lanesToShow, summarizeFollowups } from "$lib/followups";
  import type { FollowupItem, FollowupState } from "$lib/types";

  type Props = {
    followups: FollowupItem[];
    loading?: boolean;
    busyActionId?: string | null;
  };

  let { followups, loading = false, busyActionId = null }: Props = $props();
  let showDone = $state(false);

  const summary = $derived(summarizeFollowups(followups));
  const leftToDo = $derived(summary.open + summary.waiting + summary.blocked);
  const lanes = $derived(lanesToShow(groupFollowupsByLane(followups), showDone));

  const dispatch = createEventDispatcher<{
    openNote: string;
    updateState: { id: string; state: FollowupState };
    updateLane: { id: string; lane: string | null };
    complete: string;
    reopen: string;
    /** Stop tracking follow-ups that no longer matter. `lane` is set when a whole lane is dropped. */
    drop: { ids: string[]; lane: string | null };
  }>();

  function activeIds(laneFollowups: FollowupItem[]): string[] {
    return laneFollowups.filter((followup) => followup.status !== "done").map((followup) => followup.id);
  }

  function controlsDisabled(followup: FollowupItem): boolean {
    return loading || busyActionId === followup.id;
  }

  function stateValue(followup: FollowupItem): FollowupState {
    return followup.followupState ?? "open";
  }

  function updateState(followup: FollowupItem, event: Event): void {
    const state = (event.currentTarget as HTMLSelectElement).value as FollowupState;
    dispatch("updateState", { id: followup.id, state });
  }

  function updateLane(followup: FollowupItem, event: FocusEvent): void {
    const input = event.currentTarget as HTMLInputElement;
    const lane = input.value.trim() || null;
    const originalLane = input.dataset.originalLane?.trim() || null;

    if (lane !== originalLane) {
      dispatch("updateLane", { id: followup.id, lane });
    }
  }

  function commitOnEnter(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      (event.currentTarget as HTMLInputElement).blur();
    }
  }
</script>

<section class="followups-view" aria-label="Follow-ups">
  <header class="followups-head">
    <h1>Follow-ups</h1>
    <p class="summary">
      {#if loading}
        <span>Loading</span>
      {:else}
        <span class="summary-lead">{leftToDo} left to do</span>
        {#if summary.overdue > 0}<span class="overdue">{summary.overdue} overdue</span>{/if}
        {#if summary.blocked > 0}<span class="blocked">{summary.blocked} blocked</span>{/if}
        {#if summary.waiting > 0}<span class="waiting">{summary.waiting} waiting</span>{/if}
      {/if}
    </p>
    {#if summary.done > 0}
      <button class="done-toggle" type="button" aria-pressed={showDone} onclick={() => (showDone = !showDone)}>
        {showDone ? "Hide" : "Show"} {summary.done} done
      </button>
    {/if}
  </header>

  <div class="lane-list">
    {#if followups.length === 0}
      <div class="empty-state">
        <strong>No follow-ups yet</strong>
        <p>Accept a suggested action on a note, or add a follow-up from the note's toolbar. It is tracked here until you finish it.</p>
      </div>
    {:else if lanes.length === 0}
      <div class="empty-state">
        <strong>Nothing left to do</strong>
      </div>
    {:else}
      {#each lanes as lane (lane.name)}
        <section class="lane" aria-label={lane.name}>
          <div class="lane-head">
            <h2>{lane.name}</h2>
            {#if lane.activeCount > 0}
              <span>{lane.activeCount}</span>
            {/if}
            {#if lane.activeCount > 1}
              <button
                class="quiet-button lane-drop"
                type="button"
                aria-label={`Drop all ${lane.activeCount} in ${lane.name}`}
                title="Stop tracking everything open in this lane. Nothing is marked done."
                disabled={loading}
                onclick={() => dispatch("drop", { ids: activeIds(lane.followups), lane: lane.name })}
              >
                Drop all
              </button>
            {/if}
          </div>

          {#each lane.followups as followup (followup.id)}
            {@const disabled = controlsDisabled(followup)}
            {@const due = followup.status === "done" ? null : describeDue(followup.dueDate)}
            {@const state = stateValue(followup)}
            <article class="followup-row" class:done={followup.status === "done"}>
              {#if followup.status === "done"}
                <span class="check-mark checked" aria-hidden="true"><Check size={12} strokeWidth={2.4} /></span>
              {:else}
                <button
                  class="check-mark"
                  type="button"
                  aria-label={`Complete follow-up: ${followup.text}`}
                  title="Mark done"
                  {disabled}
                  onclick={() => dispatch("complete", followup.id)}
                >
                  <Check size={12} strokeWidth={2.4} aria-hidden="true" />
                </button>
              {/if}

              <div class="followup-copy">
                <span class="followup-text">{followup.text}</span>
                <div class="followup-meta">
                  {#if due}
                    <span class:overdue={due.overdue}>{due.text}</span>
                  {/if}
                  {#if followup.owner}
                    <span>{followup.owner}</span>
                  {/if}
                  <button
                    class="note-link"
                    type="button"
                    aria-label={`Open note: ${followup.noteTitle}`}
                    {disabled}
                    onclick={() => dispatch("openNote", followup.noteId)}
                  >
                    {followup.noteTitle}
                  </button>
                </div>
              </div>

              {#if followup.status === "done"}
                <div class="row-controls">
                  <span class="done-label">Done</span>
                  <button
                    class="quiet-button"
                    type="button"
                    aria-label={`Reopen follow-up: ${followup.text}`}
                    {disabled}
                    onclick={() => dispatch("reopen", followup.id)}
                  >
                    <RotateCcw size={13} strokeWidth={1.8} aria-hidden="true" />
                    Reopen
                  </button>
                </div>
              {:else}
                <div class="row-controls">
                  <input
                    class="lane-input"
                    aria-label={`Lane for: ${followup.text}`}
                    title="Move to another lane"
                    value={lane.name}
                    data-original-lane={lane.name}
                    {disabled}
                    onblur={(event) => updateLane(followup, event)}
                    onkeydown={commitOnEnter}
                  />
                  <select
                    class={`state-select ${state}`}
                    aria-label={`State for: ${followup.text}`}
                    value={state}
                    {disabled}
                    onchange={(event) => updateState(followup, event)}
                  >
                    <option value="open">Open</option>
                    <option value="waiting">Waiting</option>
                    <option value="blocked">Blocked</option>
                  </select>
                  <button
                    class="quiet-button row-drop"
                    type="button"
                    aria-label={`Drop follow-up: ${followup.text}`}
                    title="Drop: no longer relevant. It is not marked done."
                    {disabled}
                    onclick={() => dispatch("drop", { ids: [followup.id], lane: null })}
                  >
                    <X size={13} strokeWidth={1.8} aria-hidden="true" />
                    Drop
                  </button>
                </div>
              {/if}
            </article>
          {/each}
        </section>
      {/each}
    {/if}
  </div>
</section>

<style>
  .followups-view {
    display: flex;
    height: 100%;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    overflow: hidden;
  }

  .followups-head {
    display: flex;
    align-items: baseline;
    gap: 14px;
    min-height: 44px;
    padding: 11px 24px 10px;
    border-bottom: 1px solid var(--rule);
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: var(--weight-strong);
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--color-text-muted);
    font-size: 12.5px;
  }

  .summary-lead {
    color: var(--color-text-primary);
  }

  .overdue,
  .blocked {
    color: var(--color-status-error);
  }

  .waiting {
    color: var(--color-status-warning);
  }

  .done-toggle {
    margin-left: auto;
  }

  .lane-list {
    min-height: 0;
    padding: 6px 24px 40px;
    overflow: auto;
  }

  .lane {
    max-width: 980px;
    padding-top: 18px;
  }

  .lane-head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--rule);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  h2 {
    color: var(--color-text-primary);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: var(--weight-strong);
  }

  /* A row is the commitment and one tick box. The rarely used controls stay
     out of the way until the row is in hand. */
  .followup-row {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    align-items: start;
    gap: 12px;
    padding: 9px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--rule) 55%, transparent);
  }

  .check-mark {
    display: grid;
    width: 18px;
    height: 18px;
    min-height: 0;
    margin-top: 1px;
    place-items: center;
    border: 1.5px solid var(--color-border-strong);
    border-radius: 50%;
    padding: 0;
    color: transparent;
    background: transparent;
  }

  button.check-mark:hover:not(:disabled) {
    border-color: var(--color-status-success);
    color: var(--color-status-success);
  }

  .check-mark.checked {
    border-color: transparent;
    color: var(--color-app-bg);
    background: color-mix(in srgb, var(--color-status-success) 70%, transparent);
  }

  .followup-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .followup-text {
    font-size: 13px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .done .followup-text {
    color: var(--color-text-muted);
  }

  .followup-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 12px;
    color: var(--color-text-muted);
    font-size: 11.5px;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 26px;
    border: 0;
    border-radius: var(--radius-s);
    padding: 0 8px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .note-link {
    max-width: 100%;
    min-height: 0;
    overflow: hidden;
    padding: 0;
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note-link:hover:not(:disabled) {
    color: var(--color-text-primary);
    text-decoration: underline;
  }

  .done-toggle:hover,
  .quiet-button:hover:not(:disabled) {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .done-toggle[aria-pressed="true"] {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .row-controls {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .done-label {
    padding: 0 6px;
    color: var(--color-text-muted);
    font-size: 12px;
  }

  select,
  input {
    height: 26px;
    border: 1px solid transparent;
    border-radius: var(--radius-s);
    padding: 0 6px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-size: 12px;
    outline: none;
  }

  select {
    cursor: pointer;
  }

  select option {
    color: var(--color-text-primary);
    background: var(--color-surface-1);
  }

  .lane-input {
    width: 150px;
    text-align: right;
  }

  /* A state that holds work up is always visible; the rest waits for the row. */
  .lane-input,
  .state-select.open,
  .row-drop,
  .lane-drop,
  .done .quiet-button {
    opacity: 0;
  }

  .lane-drop {
    min-height: 22px;
    margin-left: auto;
    font-family: var(--font-ui);
  }

  .lane:hover .lane-drop,
  .lane-drop:focus-visible,
  .row-drop:focus-visible {
    opacity: 1;
  }

  .row-drop:hover:not(:disabled),
  .lane-drop:hover:not(:disabled) {
    color: var(--color-status-error);
  }

  .followup-row:hover .lane-input,
  .followup-row:hover .state-select,
  .followup-row:hover .quiet-button,
  .followup-row:focus-within .lane-input,
  .followup-row:focus-within .state-select,
  .followup-row:focus-within .quiet-button {
    opacity: 1;
  }

  .state-select.waiting {
    color: var(--color-status-warning);
  }

  .state-select.blocked {
    color: var(--color-status-error);
  }

  select:hover:not(:disabled),
  input:hover:not(:disabled) {
    border-color: var(--rule);
  }

  select:focus-visible,
  input:focus-visible {
    border-color: var(--color-accent-hot);
    color: var(--color-text-primary);
  }

  button:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: 1px;
  }

  button:disabled,
  select:disabled,
  input:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .empty-state {
    display: grid;
    gap: 6px;
    max-width: 460px;
    padding-top: 32px;
    color: var(--color-text-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .empty-state strong {
    color: var(--color-text-primary);
    font-weight: var(--weight-medium);
  }

  @media (max-width: 760px) {
    .followups-head,
    .lane-list {
      padding-right: 14px;
      padding-left: 14px;
    }

    .followup-row {
      grid-template-columns: 20px minmax(0, 1fr);
    }

    .row-controls {
      grid-column: 2;
    }

    .lane-input {
      text-align: left;
    }
  }
</style>
