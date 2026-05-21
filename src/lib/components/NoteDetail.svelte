<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { NoteDetail, ParseStatus, ReviewStatus } from "$lib/types";

  import StatusBadge from "./StatusBadge.svelte";

  type BadgeTone = "neutral" | "accent" | "hot" | "success" | "warning" | "error";

  type Props = {
    note: NoteDetail | null;
    loading?: boolean;
  };

  let { note, loading = false }: Props = $props();
  let reparseFeedback = $state("");
  let currentNoteId = $state<string | null>(null);

  const dispatch = createEventDispatcher<{
    retryParse: void;
    reparseWithFeedback: string;
    deleteNote: void;
  }>();

  $effect(() => {
    const noteId = note?.id ?? null;
    if (noteId !== currentNoteId) {
      currentNoteId = noteId;
      reparseFeedback = "";
    }
  });

  function parseLabel(status: ParseStatus): string {
    switch (status) {
      case "queued":
        return "Queued";
      case "parsing":
        return "Parsing";
      case "parsed":
        return "Parsed";
      case "failed":
        return "Parse failed";
    }
  }

  function parseTone(status: ParseStatus): BadgeTone {
    switch (status) {
      case "queued":
        return "neutral";
      case "parsing":
        return "accent";
      case "parsed":
        return "success";
      case "failed":
        return "error";
    }
  }

  function reviewLabel(status: ReviewStatus): string {
    switch (status) {
      case "none":
        return "No review";
      case "needs_review":
        return "Needs review";
      case "reviewed":
        return "Reviewed";
    }
  }

  function reviewTone(status: ReviewStatus): BadgeTone {
    switch (status) {
      case "none":
        return "neutral";
      case "needs_review":
        return "warning";
      case "reviewed":
        return "success";
    }
  }

  function formatDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return date.toLocaleString([], {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  }

  function dispatchReparseWithFeedback() {
    const feedback = reparseFeedback.trim();
    if (feedback) {
      dispatch("reparseWithFeedback", feedback);
      reparseFeedback = "";
    }
  }
</script>

<section class="note-detail" aria-label="Note detail">
  {#if loading && !note}
    <p class="empty-state">Loading note</p>
  {:else if !note}
    <p class="empty-state">No note selected</p>
  {:else}
    <header class="detail-header">
      <div>
        <p class="eyebrow">{formatDate(note.createdAt)}</p>
        <h2>{note.title}</h2>
      </div>
      <div class="header-actions">
        <div class="badges" aria-label="Selected note statuses">
          <StatusBadge label={parseLabel(note.parseStatus)} tone={parseTone(note.parseStatus)} />
          <StatusBadge label={reviewLabel(note.reviewStatus)} tone={reviewTone(note.reviewStatus)} />
        </div>
        <button class="delete-button" type="button" onclick={() => dispatch("deleteNote")} disabled={loading}>
          Delete
        </button>
      </div>
    </header>

    {#if note.parseStatus === "failed"}
      <div class="parse-failure">
        <span>{note.parseError ?? "Parser failed"}</span>
        <button type="button" onclick={() => dispatch("retryParse")} disabled={loading}>Retry</button>
      </div>
    {/if}

    <section class="reparse-block" aria-label="Reparse note">
      <label>
        <span>Reparse feedback</span>
        <textarea
          bind:value={reparseFeedback}
          aria-label="Reparse feedback"
          disabled={loading}
          rows="3"
        ></textarea>
      </label>
      <button type="button" onclick={dispatchReparseWithFeedback} disabled={loading || !reparseFeedback.trim()}>
        Reparse with feedback
      </button>
    </section>

    <div class="text-grid">
      <section class="text-block" aria-label="Raw note">
        <h3>Raw</h3>
        <p>{note.rawText}</p>
      </section>

      <section class="text-block" aria-label="Cleaned note">
        <h3>Cleaned</h3>
        <p>{note.cleanedText ?? "Waiting for parser output."}</p>
      </section>
    </div>

    {#if note.summary}
      <section class="summary-block" aria-label="Summary">
        <h3>Summary</h3>
        <p>{note.summary}</p>
      </section>
    {/if}

    <section class="tags-block" aria-label="Tags">
      <h3>Tags</h3>
      <div class="tags">
        {#if note.tags.length === 0}
          <span class="muted">No tags</span>
        {/if}

        {#each note.tags as tag}
          <span class="tag">{tag.name}</span>
        {/each}
      </div>
    </section>

    <section class="actions-block" aria-label="Action items">
      <h3>Actions</h3>
      <div class="actions">
        {#if note.actionItems.length === 0}
          <p class="muted">No actions</p>
        {/if}

        {#each note.actionItems as action}
          <article class="action-row">
            <p>{action.text}</p>
            <StatusBadge label={action.status} tone={action.status === "suggested" ? "warning" : "neutral"} />
          </article>
        {/each}
      </div>
    </section>
  {/if}
</section>

<style>
  .note-detail {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 13px 14px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-2);
  }

  .eyebrow {
    margin: 0 0 4px;
    color: var(--color-accent-primary);
    font-size: 11px;
    font-weight: 800;
    line-height: 1;
    text-transform: uppercase;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 18px;
    line-height: 1.2;
  }

  h3 {
    color: var(--color-text-primary);
    font-size: 12px;
    line-height: 1;
    text-transform: uppercase;
  }

  .header-actions,
  .badges,
  .tags {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .badges {
    justify-content: flex-end;
  }

  .header-actions {
    align-items: flex-end;
    flex-direction: column;
  }

  .parse-failure {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--color-border-default);
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 10%, var(--color-surface-1));
    font-size: 12px;
  }

  .parse-failure button,
  .reparse-block button,
  .delete-button,
  .tag {
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
  }

  .parse-failure button,
  .reparse-block button,
  .delete-button {
    min-height: 28px;
    padding: 0 10px;
    cursor: pointer;
  }

  .delete-button {
    color: var(--color-status-error);
  }

  .reparse-block {
    display: grid;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  .reparse-block label {
    display: grid;
    gap: 6px;
  }

  .reparse-block span {
    color: var(--color-text-primary);
    font-size: 12px;
    font-weight: 800;
    line-height: 1;
    text-transform: uppercase;
  }

  .reparse-block textarea {
    width: 100%;
    min-height: 68px;
    resize: vertical;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 8px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
    line-height: 1.35;
  }

  .text-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    background: var(--color-border-default);
  }

  .text-block,
  .summary-block,
  .tags-block,
  .actions-block {
    display: grid;
    gap: 9px;
    padding: 13px 14px;
    background: var(--color-surface-1);
  }

  .text-block p,
  .summary-block p,
  .actions-block p,
  .muted {
    color: var(--color-text-muted);
    font-size: 13px;
    line-height: 1.45;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .tag {
    padding: 4px 8px;
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .actions {
    display: grid;
    gap: 8px;
  }

  .action-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    min-width: 0;
    padding: 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-2);
  }

  .empty-state {
    margin: 0;
    padding: 16px 14px;
    color: var(--color-text-muted);
    font-size: 13px;
  }

  @media (max-width: 900px) {
    .text-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .detail-header {
      flex-direction: column;
    }

    .badges {
      justify-content: flex-start;
    }
  }
</style>
