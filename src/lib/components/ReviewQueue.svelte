<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import { createEventDispatcher } from "svelte";
  import type { ActionReviewItem } from "$lib/types";
  import { readFlag, writeFlag } from "$lib/ui-prefs";

  import StatusBadge from "./StatusBadge.svelte";

  type Props = {
    actions: ActionReviewItem[];
    busyActionId?: string | null;
    loading?: boolean;
  };

  const COLLAPSED_KEY = "work-notes.review-queue-collapsed";

  let { actions, busyActionId = null, loading = false }: Props = $props();
  let collapsed = $state(readFlag(COLLAPSED_KEY, false));

  function toggleCollapsed() {
    collapsed = !collapsed;
    writeFlag(COLLAPSED_KEY, collapsed);
  }

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

<section class="review-queue" class:collapsed aria-label="Review queue">
  <header class="queue-header">
    <button
      class="queue-toggle"
      type="button"
      aria-expanded={!collapsed}
      aria-controls="review-queue-list"
      title={collapsed ? "Show suggested actions" : "Hide suggested actions"}
      onclick={toggleCollapsed}
    >
      <ChevronDown class="queue-chevron" size={14} strokeWidth={1.8} aria-hidden="true" />
      <h2>Suggested actions</h2>
    </button>
    <StatusBadge label={`${actions.length} pending`} tone={actions.length > 0 ? "warning" : "neutral"} />
  </header>

  {#if !collapsed}
  <div class="queue-list" id="review-queue-list">
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
  {/if}
</section>

<style>
  .review-queue {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border-bottom: 1px solid var(--rule);
    overflow: hidden;
  }

  .queue-header {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 0 12px 0 28px;
  }

  .queue-toggle {
    display: inline-flex;
    flex: 1 1 auto;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    margin-left: -8px;
    border: 0;
    padding: 0 8px;
    color: var(--color-text-primary);
    background: transparent;
    text-align: left;
  }

  .queue-toggle:hover {
    background: var(--color-surface-2);
  }

  .review-queue.collapsed :global(.queue-chevron) {
    transform: rotate(-90deg);
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: var(--weight-medium);
  }

  .queue-list {
    flex: 1;
    min-height: 0;
    padding: 0 12px 6px 28px;
    overflow: auto;
  }

  .action-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-top: 1px solid var(--rule);
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
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-size: 11.5px;
    text-align: left;
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
    font: inherit;
    font-size: 12.5px;
    border-radius: var(--radius-s);
    cursor: pointer;
  }

  .action-buttons button {
    min-height: 26px;
    border: 1px solid transparent;
    padding: 0 9px;
    color: var(--color-text-muted);
    background: transparent;
  }

  .action-buttons button:hover:not(:disabled) {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .action-buttons button:first-child {
    border-color: color-mix(in srgb, var(--color-status-success) 55%, transparent);
    color: var(--color-status-success);
  }

  button:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: 1px;
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
