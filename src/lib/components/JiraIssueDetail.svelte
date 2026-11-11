<script lang="ts">
  import Check from "@lucide/svelte/icons/check";
  import Copy from "@lucide/svelte/icons/copy";
  import { createEventDispatcher, onDestroy } from "svelte";
  import { buildTicketPrompt } from "$lib/jira-prompt";
  import type { JiraIssueDetail, JiraIssueEdit, JiraTransition } from "$lib/types";

  import MarkdownView from "./MarkdownView.svelte";

  type CopyState = "idle" | "copied" | "failed";

  const COPY_FEEDBACK_MS = 2000;
  const COPY_LABELS: Record<CopyState, string> = {
    idle: "Copy agent prompt",
    copied: "Copied",
    failed: "Copy failed",
  };

  type Props = {
    issue: JiraIssueDetail | null;
    loading?: boolean;
    saving?: boolean;
  };

  let { issue, loading = false, saving = false }: Props = $props();
  let editing = $state(false);
  let summaryDraft = $state("");
  let descriptionDraft = $state("");
  let flattenAcknowledged = $state(false);
  let commentText = $state("");
  let currentKey = $state<string | null>(null);
  let copyState = $state<CopyState>("idle");
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined;

  const dispatch = createEventDispatcher<{
    transition: string;
    comment: { text: string; done: () => void };
    saveEdit: { edit: JiraIssueEdit; done: () => void };
    assignToMe: void;
  }>();

  const summaryChanged = $derived(Boolean(issue) && summaryDraft.trim() !== issue?.summary);
  const descriptionChanged = $derived(Boolean(issue) && descriptionDraft !== issue?.description);
  // Saving a plain-text description replaces Jira tables, panels, and inline
  // formatting, so that needs an explicit yes.
  const needsFlattenConfirmation = $derived(
    descriptionChanged && issue !== null && !issue.descriptionEditableLosslessly,
  );
  const canSave = $derived(
    !saving &&
      summaryDraft.trim().length > 0 &&
      (summaryChanged || descriptionChanged) &&
      (!needsFlattenConfirmation || flattenAcknowledged),
  );

  $effect(() => {
    const key = issue?.key ?? null;
    if (key !== currentKey) {
      currentKey = key;
      editing = false;
      commentText = "";
      showCopyState("idle");
    }
  });

  onDestroy(() => clearTimeout(copyResetTimer));

  function showCopyState(next: CopyState) {
    clearTimeout(copyResetTimer);
    copyState = next;
    if (next !== "idle") {
      copyResetTimer = setTimeout(() => (copyState = "idle"), COPY_FEEDBACK_MS);
    }
  }

  async function copyPrompt() {
    if (!issue) {
      return;
    }
    try {
      await navigator.clipboard.writeText(buildTicketPrompt(issue));
      showCopyState("copied");
    } catch {
      showCopyState("failed");
    }
  }

  function startEdit() {
    if (!issue) {
      return;
    }
    summaryDraft = issue.summary;
    descriptionDraft = issue.description;
    flattenAcknowledged = false;
    editing = true;
  }

  function saveEdit(event: SubmitEvent) {
    event.preventDefault();
    if (!canSave) {
      return;
    }

    const edit: JiraIssueEdit = {};
    if (summaryChanged) {
      edit.summary = summaryDraft.trim();
    }
    if (descriptionChanged) {
      edit.description = descriptionDraft;
    }
    dispatch("saveEdit", { edit, done: () => (editing = false) });
  }

  function addComment(event: SubmitEvent) {
    event.preventDefault();
    const text = commentText.trim();
    if (!text || saving) {
      return;
    }
    dispatch("comment", { text, done: () => (commentText = "") });
  }

  function transitionLabel(transition: JiraTransition): string {
    return transition.toStatus || transition.name;
  }

  function formatDate(value: string | null): string {
    if (!value) {
      return "";
    }
    const date = new Date(value);
    return Number.isNaN(date.getTime())
      ? value
      : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  }
</script>

<section class="jira-detail" aria-label="Ticket detail">
  {#if !issue}
    <div class="empty-state">
      <p>{loading ? "Loading ticket" : "Select a ticket"}</p>
    </div>
  {:else}
    <header class="detail-header">
      <div class="head-meta">
        <a class="issue-key" href={issue.url} target="_blank" rel="noreferrer" title="Open in Jira">{issue.key}</a>
        <span class={`status-pill ${issue.statusCategory}`}>{issue.status}</span>
        <span>{issue.issueType}</span>
        {#if issue.priority}
          <span>{issue.priority}</span>
        {/if}
        <span>{issue.assignee ?? "Unassigned"}</span>
      </div>
      <div class="header-actions">
        <button class="ghost-button" type="button" disabled={saving} onclick={() => dispatch("assignToMe")}>
          Assign to me
        </button>
        <button class="ghost-button" type="button" disabled={saving || editing} onclick={startEdit}>Edit</button>
        <button
          class="primary-button copy-button"
          class:failed={copyState === "failed"}
          type="button"
          title="Copy a prompt with this ticket's details to paste into your coding agent"
          onclick={() => void copyPrompt()}
        >
          {#if copyState === "copied"}
            <Check size={14} aria-hidden="true" />
          {:else}
            <Copy size={14} aria-hidden="true" />
          {/if}
          {COPY_LABELS[copyState]}
        </button>
      </div>
    </header>

    <div class="detail-scroll" class:loading>
      <div class="detail-content">
      {#if editing}
        <form class="edit-form" aria-label="Edit ticket" onsubmit={saveEdit}>
          <label>
            <span>Summary</span>
            <input bind:value={summaryDraft} aria-label="Summary" disabled={saving} />
          </label>
          <label>
            <span>Description</span>
            <textarea bind:value={descriptionDraft} aria-label="Description" rows="14" disabled={saving}></textarea>
            <small>Blank lines separate paragraphs. Start lines with "- " for bullets or "## " for headings.</small>
          </label>
          {#if needsFlattenConfirmation}
            <label class="flatten-warning">
              <input type="checkbox" bind:checked={flattenAcknowledged} disabled={saving} />
              <span>
                Replace the formatted description with this plain text. Tables, panels, and inline formatting
                from Jira will be lost.
              </span>
            </label>
          {/if}
          <div class="form-actions">
            <button class="ghost-button" type="button" disabled={saving} onclick={() => (editing = false)}>
              Cancel
            </button>
            <button class="primary-button" type="submit" disabled={!canSave}>Save</button>
          </div>
        </form>
      {:else}
        <h1>{issue.summary}</h1>

        <div class="detail-facts">
          {#if issue.reporter}<span>Reporter: {issue.reporter}</span>{/if}
          {#if issue.parentKey}<span>Parent: {issue.parentKey}</span>{/if}
          {#if issue.updated}<span>Updated: {formatDate(issue.updated)}</span>{/if}
          {#each issue.labels as label}
            <span class="label-chip">{label}</span>
          {/each}
        </div>

        {#if issue.transitions.length > 0}
          <div class="transition-row" role="group" aria-label="Move ticket">
            <span>Move to</span>
            {#each issue.transitions as transition}
              <button
                class="ghost-button"
                type="button"
                disabled={saving}
                aria-label={`Move to ${transitionLabel(transition)}`}
                title={transition.name}
                onclick={() => dispatch("transition", transition.id)}
              >
                {transitionLabel(transition)}
              </button>
            {/each}
          </div>
        {/if}

        <section class="detail-section" aria-label="Description">
          <h2>Description</h2>
          {#if issue.description.trim()}
            <MarkdownView markdown={issue.description} preserveLineBreaks />
          {:else}
            <p class="quiet">No description</p>
          {/if}
        </section>
      {/if}

      <section class="detail-section" aria-label="Comments">
        <h2>Comments ({issue.comments.length})</h2>
        {#each issue.comments as comment (comment.id)}
          <article class="comment">
            <div class="comment-head">
              <strong>{comment.author}</strong>
              <span>{formatDate(comment.created)}</span>
            </div>
            <MarkdownView markdown={comment.body} preserveLineBreaks />
          </article>
        {/each}

        <form class="comment-form" aria-label="Add comment" onsubmit={addComment}>
          <textarea
            bind:value={commentText}
            aria-label="Comment"
            rows="3"
            placeholder="Add a comment"
            disabled={saving}
          ></textarea>
          <div class="form-actions">
            <button class="primary-button" type="submit" disabled={saving || !commentText.trim()}>
              Add comment
            </button>
          </div>
        </form>
      </section>
      </div>
    </div>
  {/if}
</section>

<style>
  .jira-detail {
    display: flex;
    min-width: 0;
    min-height: 0;
    height: 100%;
    flex-direction: column;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    overflow: hidden;
  }

  .empty-state {
    display: grid;
    flex: 1;
    place-items: center;
    color: var(--color-text-muted);
    font-size: 13px;
  }

  .detail-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 6px 12px;
    min-height: 44px;
    padding: 6px 12px 6px 24px;
    border-bottom: 1px solid var(--rule);
  }

  .head-meta,
  .header-actions,
  .detail-facts,
  .transition-row,
  .form-actions,
  .comment-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    min-width: 0;
  }

  .header-actions,
  .transition-row {
    gap: 4px;
  }

  .head-meta,
  .detail-facts,
  .transition-row > span,
  .comment-head span,
  .quiet,
  small,
  label > span {
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .issue-key {
    color: var(--color-accent-hot);
    font-family: var(--font-mono);
    font-size: 12.5px;
    text-decoration: none;
  }

  .issue-key:hover,
  .issue-key:focus-visible {
    text-decoration: underline;
    outline: none;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--color-text-primary);
    font-size: 12px;
    font-weight: var(--weight-medium);
    white-space: nowrap;
  }

  .status-pill::before {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-muted);
    content: "";
  }

  .status-pill.indeterminate::before {
    background: var(--color-accent-hot);
  }

  .status-pill.done::before {
    background: var(--color-status-success);
  }

  .label-chip::before {
    content: "#";
    opacity: 0.55;
  }

  .detail-scroll {
    flex: 1 1 auto;
    min-height: 0;
    padding: 24px 24px 32px;
    overflow: auto;
  }

  .detail-scroll.loading {
    opacity: 0.6;
  }

  /* Long ticket text is easier to read in a bounded column. */
  .detail-content {
    display: grid;
    align-content: start;
    gap: 14px;
    max-width: 860px;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: var(--weight-strong);
    line-height: 1.3;
  }

  h2 {
    margin-bottom: 10px;
    color: var(--color-text-muted);
    font-size: 12px;
    font-weight: var(--weight-medium);
  }

  .transition-row > span {
    margin-right: 6px;
  }

  .detail-section {
    min-width: 0;
    margin-top: 6px;
    padding-top: 16px;
    border-top: 1px solid var(--rule);
  }

  .comment {
    display: grid;
    gap: 4px;
    margin-bottom: 16px;
  }

  .comment-head strong {
    font-size: 12.5px;
    font-weight: var(--weight-medium);
  }

  .edit-form,
  .comment-form,
  label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .edit-form {
    gap: 14px;
  }

  input:not([type="checkbox"]),
  textarea {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-m);
    padding: 8px 10px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
    line-height: 1.5;
    outline: none;
    resize: vertical;
  }

  .edit-form textarea {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }

  input:focus-visible,
  textarea:focus-visible {
    border-color: var(--color-accent-hot);
  }

  button:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: 1px;
  }

  .flatten-warning {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 9px 12px;
    border-left: 2px solid var(--color-status-warning);
    background: color-mix(in srgb, var(--color-status-warning) 8%, transparent);
  }

  .flatten-warning span {
    color: var(--color-text-primary);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .form-actions {
    justify-content: flex-end;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding: 0 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-s);
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }

  .ghost-button {
    color: var(--color-text-muted);
  }

  .ghost-button:hover:not(:disabled) {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  /* Status moves are choices, so they keep an outline to read as buttons. */
  .transition-row .ghost-button {
    border-color: var(--rule);
    color: var(--color-text-primary);
  }

  .primary-button {
    border-color: color-mix(in srgb, var(--color-accent-primary) 60%, transparent);
    background: color-mix(in srgb, var(--color-accent-primary) 20%, transparent);
    font-weight: var(--weight-medium);
  }

  .primary-button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-accent-primary) 32%, transparent);
  }

  .copy-button.failed {
    border-color: var(--color-status-error);
    color: var(--color-status-error);
  }

  button:disabled,
  input:disabled,
  textarea:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
</style>
