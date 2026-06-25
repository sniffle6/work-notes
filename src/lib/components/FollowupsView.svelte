<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { displayFollowupState, groupFollowupsByLane } from "$lib/followups";
  import type { FollowupItem, FollowupState } from "$lib/types";

  type Props = {
    followups: FollowupItem[];
    loading?: boolean;
    busyActionId?: string | null;
  };

  let { followups, loading = false, busyActionId = null }: Props = $props();

  const lanes = $derived(groupFollowupsByLane(followups));

  const dispatch = createEventDispatcher<{
    openNote: string;
    updateState: { id: string; state: FollowupState };
    updateLane: { id: string; lane: string | null };
    complete: string;
    reopen: string;
  }>();

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
</script>

<section class="followups-view" aria-label="Follow-ups">
  <header class="followups-head">
    <div>
      <h1>Follow-ups</h1>
      <p>{loading ? "Loading" : `${followups.length} tracked`}</p>
    </div>
  </header>

  <div class="lane-list">
    {#if lanes.length === 0}
      <div class="empty-state">No follow-ups yet</div>
    {:else}
      {#each lanes as lane}
        <section class="lane" aria-label={lane.name}>
          <div class="lane-head">
            <h2>{lane.name}</h2>
            <span>{lane.activeCount} active</span>
          </div>

          <div class="followup-rows">
            {#each lane.followups as followup}
              {@const disabled = controlsDisabled(followup)}
              <article class="followup-row" class:done={followup.status === "done"}>
                <div class="followup-copy">
                  <span class="followup-text" title={followup.text}>{followup.text}</span>
                  <button
                    class="note-link"
                    type="button"
                    aria-label={`Open note: ${followup.noteTitle}`}
                    disabled={disabled}
                    onclick={() => dispatch("openNote", followup.noteId)}
                  >
                    {followup.noteTitle}
                  </button>
                </div>

                {#if followup.status === "done"}
                  <div class="done-controls">
                    <span class="state-pill">{displayFollowupState(followup)}</span>
                    <button
                      class="ghost-button"
                      type="button"
                      aria-label={`Reopen follow-up: ${followup.text}`}
                      disabled={disabled}
                      onclick={() => dispatch("reopen", followup.id)}
                    >
                      Reopen
                    </button>
                  </div>
                {:else}
                  <div class="active-controls">
                    <label>
                      <span>State</span>
                      <select
                        aria-label={`State for: ${followup.text}`}
                        value={stateValue(followup)}
                        disabled={disabled}
                        onchange={(event) => updateState(followup, event)}
                      >
                        <option value="open">Open</option>
                        <option value="waiting">Waiting</option>
                        <option value="blocked">Blocked</option>
                      </select>
                    </label>

                    <label>
                      <span>Lane</span>
                      <input
                        aria-label={`Lane for: ${followup.text}`}
                        value={lane.name}
                        data-original-lane={lane.name}
                        disabled={disabled}
                        onblur={(event) => updateLane(followup, event)}
                      />
                    </label>

                    <button
                      class="complete-button"
                      type="button"
                      aria-label={`Complete follow-up: ${followup.text}`}
                      disabled={disabled}
                      onclick={() => dispatch("complete", followup.id)}
                    >
                      Complete
                    </button>
                  </div>
                {/if}
              </article>
            {/each}
          </div>
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
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 92%, var(--color-app-bg));
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    color: var(--color-text-primary);
    font-size: 16px;
    font-weight: 750;
    line-height: 1.2;
  }

  .followups-head p,
  .lane-head,
  .note-link,
  label span,
  .state-pill,
  .empty-state {
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .lane-list {
    display: grid;
    align-content: start;
    gap: 16px;
    min-width: 0;
    min-height: 0;
    padding: 16px;
    overflow: auto;
  }

  .lane {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .lane-head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 22px;
    font-weight: 800;
    text-transform: uppercase;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 12px;
    font-weight: 800;
    line-height: 1.2;
  }

  .lane-head span,
  .state-pill {
    display: inline-grid;
    min-height: 20px;
    place-items: center;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--color-surface-2);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-weight: 750;
    text-transform: none;
    white-space: nowrap;
  }

  .followup-rows {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .followup-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    min-width: 0;
    min-height: 46px;
    padding: 8px 10px;
    border-radius: 7px;
  }

  .followup-row:hover {
    background: var(--color-surface-2);
  }

  .followup-row.done {
    color: var(--color-text-muted);
  }

  .followup-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .followup-text {
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .done .followup-text {
    color: var(--color-text-muted);
  }

  .note-link {
    width: fit-content;
    max-width: 100%;
    overflow: hidden;
    border: 0;
    padding: 0;
    background: transparent;
    font: inherit;
    font-weight: 700;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .note-link:hover:not(:disabled),
  .note-link:focus-visible {
    color: var(--color-accent-primary);
    outline: none;
    text-decoration: underline;
  }

  .active-controls,
  .done-controls {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  label {
    display: grid;
    gap: 3px;
    min-width: 84px;
  }

  label span {
    font-weight: 800;
  }

  select,
  input,
  button {
    min-height: 28px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
  }

  select,
  input {
    min-width: 0;
    outline: none;
  }

  select {
    width: 92px;
    padding: 0 8px;
  }

  input {
    width: 150px;
    padding: 0 8px;
  }

  button {
    padding: 0 9px;
    font-weight: 750;
    cursor: pointer;
  }

  select:focus-visible,
  input:focus-visible,
  button:focus-visible {
    border-color: var(--color-accent-primary);
    outline: 1px solid var(--color-accent-primary);
    outline-offset: 1px;
  }

  .complete-button:hover:not(:disabled) {
    border-color: var(--color-status-success);
    color: var(--color-status-success);
  }

  .ghost-button:hover:not(:disabled) {
    border-color: var(--color-accent-primary);
    color: var(--color-accent-primary);
  }

  button:disabled,
  select:disabled,
  input:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .empty-state {
    display: grid;
    min-height: 180px;
    place-items: center;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-surface-1) 82%, transparent);
    font-weight: 750;
  }

  @media (max-width: 760px) {
    .followups-head,
    .lane-list {
      padding-right: 12px;
      padding-left: 12px;
    }

    .followup-row {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .active-controls,
    .done-controls {
      flex-wrap: wrap;
    }

    input {
      width: min(220px, 100%);
    }
  }
</style>
