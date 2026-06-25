<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { ActionReviewItem } from "$lib/types";

  import StatusBadge from "./StatusBadge.svelte";

  type Props = {
    actions: ActionReviewItem[];
    busyActionId?: string | null;
    loading?: boolean;
  };

  let { actions, busyActionId = null, loading = false }: Props = $props();

  const dispatch = createEventDispatcher<{
    select: string;
    accept: string;
    dismiss: string;
  }>();

  function actionMetaParts(action: ActionReviewItem): string[] {
    const parts = [action.owner, action.dueDate, confidenceLabel(action.confidence)].filter(Boolean) as string[];
    return parts.length > 0 ? parts : ["Needs confirmation"];
  }

  function confidenceLabel(confidence: number | null | undefined): string | null {
    return typeof confidence === "number" ? `${Math.round(confidence * 100)}%` : null;
  }
</script>

<section class="review-queue" aria-label="Review queue">
  <header class="queue-header">
    <div>
      <p class="eyebrow">Review</p>
      <h2>Suggested actions</h2>
    </div>
    <StatusBadge label={`${actions.length} pending`} tone={actions.length > 0 ? "warning" : "neutral"} />
  </header>

  <div class="queue-list">
    {#if actions.length === 0}
      <p class="empty-state">No suggested actions</p>
    {/if}

    {#each actions as action}
      <article class="action-card">
        <div class="action-copy">
          <button
            class="note-link"
            type="button"
            aria-label={`Open note: ${action.noteTitle}`}
            onclick={() => dispatch("select", action.noteId)}
          >
            {action.noteTitle}
          </button>
          <p>{action.text}</p>
          <small>
            {#each actionMetaParts(action) as part, index}
              <span>{part}</span>{#if index < actionMetaParts(action).length - 1} · {/if}
            {/each}
          </small>
        </div>
        <div class="action-buttons">
          <button
            type="button"
            aria-label={`Accept action: ${action.text}`}
            disabled={loading || busyActionId === action.id}
            onclick={() => dispatch("accept", action.id)}
          >
            Accept
          </button>
          <button
            type="button"
            aria-label={`Dismiss action: ${action.text}`}
            disabled={loading || busyActionId === action.id}
            onclick={() => dispatch("dismiss", action.id)}
          >
            Dismiss
          </button>
        </div>
      </article>
    {/each}
  </div>
</section>

<style>
  .review-queue {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .queue-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-2);
  }

  .eyebrow {
    margin: 0 0 3px;
    color: var(--color-accent-primary);
    font-size: 11px;
    font-weight: 800;
    line-height: 1;
    text-transform: uppercase;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 16px;
    line-height: 1.15;
  }

  .queue-list {
    display: grid;
    flex: 1;
    gap: 8px;
    min-height: 0;
    padding: 10px;
    overflow: auto;
  }

  .action-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    padding: 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-2);
  }

  .action-copy {
    display: grid;
    gap: 5px;
    min-width: 0;
  }

  .action-copy small,
  .empty-state {
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.2;
  }

  .note-link {
    width: fit-content;
    border: 0;
    padding: 0;
    color: var(--color-accent-primary);
    background: transparent;
    font: inherit;
    font-size: 11px;
    font-weight: 800;
    text-align: left;
    text-transform: uppercase;
    cursor: pointer;
  }

  .action-copy small span {
    color: inherit;
    font: inherit;
  }

  .note-link:hover,
  .note-link:focus-visible {
    text-decoration: underline;
    outline: none;
  }

  .action-copy p {
    color: var(--color-text-primary);
    font-size: 13px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .action-buttons {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }

  button {
    min-height: 28px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 0 9px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
    font-weight: 750;
    cursor: pointer;
  }

  button:first-child {
    border-color: var(--color-status-success);
    color: var(--color-status-success);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .empty-state {
    padding: 4px;
  }

  @media (max-width: 640px) {
    .action-card {
      grid-template-columns: 1fr;
    }

    .action-buttons {
      justify-content: flex-end;
    }
  }
</style>
