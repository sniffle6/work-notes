<script lang="ts">
  import { createEventDispatcher, tick } from "svelte";
  import type { ActionItem, NoteDetail, ParseStatus, ReviewStatus } from "$lib/types";

  import CleanedEditor from "./CleanedEditor.svelte";
  import MarkdownView from "./MarkdownView.svelte";

  type Props = {
    note: NoteDetail | null;
    loading?: boolean;
    busyActionId?: string | null;
  };

  let { note, loading = false, busyActionId = null }: Props = $props();
  let reparseFeedback = $state("");
  let reparseOpen = $state(false);
  let showRaw = $state(false);
  let editing = $state(false);
  let reparseConfirmOpen = $state(false);
  let pendingReparse = $state<{ kind: "retry" } | { kind: "feedback"; feedback: string } | null>(null);
  let followupOpen = $state(false);
  let followupText = $state("");
  let followupLane = $state("");
  let currentNoteId = $state<string | null>(null);
  let reparseFeedbackInput = $state<HTMLTextAreaElement | null>(null);

  const suggestedActions = $derived(note?.actionItems.filter((action) => action.status === "suggested") ?? []);
  const acceptedActions = $derived(note?.actionItems.filter((action) => action.status === "accepted") ?? []);
  const doneActions = $derived(note?.actionItems.filter((action) => action.status === "done") ?? []);

  const dispatch = createEventDispatcher<{
    retryParse: void;
    reparseWithFeedback: string;
    deleteNote: void;
    restoreNote: void;
    permanentlyDeleteNote: void;
    acceptAction: string;
    dismissAction: string;
    completeAction: string;
    reopenAction: string;
    createFollowup: { text: string; lane: string | null; done: () => void };
    saveCleaned: { title: string; summary: string; cleanedText: string; done: () => void };
  }>();

  $effect(() => {
    const noteId = note?.id ?? null;
    if (noteId !== currentNoteId) {
      currentNoteId = noteId;
      reparseFeedback = "";
      reparseOpen = false;
      showRaw = false;
      editing = false;
      reparseConfirmOpen = false;
      pendingReparse = null;
      followupOpen = false;
      followupText = "";
      followupLane = "";
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

  function statusClass(status: ParseStatus, review: ReviewStatus): string {
    if (status === "failed") return "error";
    if (status === "parsing") return "info";
    if (status === "queued") return "neutral";
    if (review === "needs_review") return "warning";
    if (review === "reviewed") return "success reviewed";
    return "neutral";
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

  async function openReparseDialog() {
    reparseOpen = true;
    await tick();
    reparseFeedbackInput?.focus();
  }

  function closeReparseDialog() {
    reparseOpen = false;
  }

  function dispatchReparseWithFeedback() {
    const feedback = reparseFeedback.trim();
    if (!feedback) {
      return;
    }
    reparseOpen = false;
    if (note?.cleanedEdited) {
      pendingReparse = { kind: "feedback", feedback };
      reparseConfirmOpen = true;
    } else {
      dispatch("reparseWithFeedback", feedback);
      reparseFeedback = "";
    }
  }

  function startEdit() {
    editing = true;
  }

  function onEditorSave(event: CustomEvent<{ title: string; summary: string; cleanedText: string }>) {
    dispatch("saveCleaned", {
      title: event.detail.title,
      summary: event.detail.summary,
      cleanedText: event.detail.cleanedText,
      done: () => {
        editing = false;
      },
    });
  }

  function requestRetry() {
    if (note?.cleanedEdited) {
      pendingReparse = { kind: "retry" };
      reparseConfirmOpen = true;
    } else {
      dispatch("retryParse");
    }
  }

  function confirmReparse() {
    if (pendingReparse?.kind === "retry") {
      dispatch("retryParse");
    } else if (pendingReparse?.kind === "feedback") {
      dispatch("reparseWithFeedback", pendingReparse.feedback);
      reparseFeedback = "";
    }
    pendingReparse = null;
    reparseConfirmOpen = false;
  }

  function cancelReparseConfirm() {
    pendingReparse = null;
    reparseConfirmOpen = false;
  }

  function openFollowupForm() {
    followupOpen = true;
  }

  function dispatchCreateFollowup(event: SubmitEvent) {
    event.preventDefault();

    const text = followupText.trim();
    if (!text) {
      return;
    }

    const noteId = currentNoteId;
    const lane = followupLane.trim() || null;

    dispatch("createFollowup", {
      text,
      lane,
      done: () => {
        if (currentNoteId !== noteId) {
          return;
        }
        followupText = "";
        followupLane = "";
        followupOpen = false;
      },
    });
  }

  function actionMeta(action: ActionItem): string {
    return [action.owner ? `@${action.owner}` : null, action.dueDate ? formatDue(action.dueDate) : null]
      .filter(Boolean)
      .join(" ");
  }

  function formatDue(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  }
</script>

<section class="note-detail" aria-label="Note detail">
  {#if loading && !note}
    <div class="empty-state">
      <div class="empty-mark">WN</div>
      <p>Loading note</p>
    </div>
  {:else if !note}
    <div class="empty-state">
      <div class="empty-mark">WN</div>
      <h2>Select a note</h2>
      <p>Or press Ctrl+Shift+Space to capture a new one.</p>
    </div>
  {:else}
    <header class="detail-header">
      <div class="head-meta">
        <span class={`status-dot ${statusClass(note.parseStatus, note.reviewStatus)}`} title={`${parseLabel(note.parseStatus)} - ${reviewLabel(note.reviewStatus)}`}></span>
        <span>{formatDate(note.createdAt)}</span>
        <span>{parseLabel(note.parseStatus)}</span>
        <span>{reviewLabel(note.reviewStatus)}</span>
      </div>
      <div class="header-actions">
        {#if note.parseStatus === "failed"}
          <button class="ghost-button" type="button" onclick={requestRetry} disabled={loading}>Retry</button>
        {/if}
        <button class="ghost-button" type="button" onclick={requestRetry} disabled={loading}>Reparse</button>
        <button class="ghost-button" type="button" onclick={openReparseDialog} disabled={loading}>
          Reparse with feedback
        </button>
        {#if note.isArchived}
          <button class="ghost-button" type="button" onclick={() => dispatch("restoreNote")} disabled={loading}>
            Restore
          </button>
          <button class="delete-button" type="button" onclick={() => dispatch("permanentlyDeleteNote")} disabled={loading}>
            Delete permanently
          </button>
        {:else}
          <button class="ghost-button" type="button" onclick={openFollowupForm} disabled={loading}>
            Add follow-up
          </button>
          <button class="delete-button" type="button" onclick={() => dispatch("deleteNote")} disabled={loading}>
            Archive
          </button>
        {/if}
      </div>
    </header>

    <div class="detail-scroll">
      <h1>{note.title}</h1>

      {#if followupOpen}
        <form class="followup-form" aria-label="Manual follow-up" onsubmit={dispatchCreateFollowup}>
          <div class="followup-fields">
            <label>
              <span>Follow-up</span>
              <input
                bind:value={followupText}
                aria-label="Follow-up text"
                disabled={loading}
                placeholder="What needs to happen?"
              />
            </label>
            <label>
              <span>Lane</span>
              <input
                bind:value={followupLane}
                aria-label="Follow-up lane"
                disabled={loading}
                placeholder="Optional"
              />
            </label>
          </div>
          <button class="primary-action" type="submit" disabled={loading || !followupText.trim()}>
            Create follow-up
          </button>
        </form>
      {/if}

      {#if note.parseStatus === "failed"}
        <div class="detail-banner error">
          <strong>Parser failed</strong>
          <span>{note.parseError ?? "Parser failed. Raw note text is still saved."}</span>
          <button type="button" onclick={requestRetry} disabled={loading}>Retry</button>
        </div>
      {:else if note.parseStatus === "parsing"}
        <div class="detail-banner info">
          <strong>Parsing in background</strong>
          <span>Codex is cleaning and tagging. Raw note is saved.</span>
        </div>
      {/if}

      {#if note.summary && note.summary !== note.title}
        <section class="summary-block" aria-label="Summary">
          <span>Summary</span>
          <p>{note.summary}</p>
        </section>
      {/if}

      <div class="body-head">
        <div class="tab-row" aria-label="Note text view">
          <button class:active={!showRaw} type="button" onclick={() => (showRaw = false)}>Cleaned</button>
          <button class:active={showRaw} type="button" onclick={() => (showRaw = true)}>Raw</button>
        </div>
      </div>

      <section class="note-body" aria-label={showRaw ? "Raw note" : "Cleaned note"}>
        {#if showRaw}
          <pre>{note.rawText}</pre>
        {:else if editing}
          <CleanedEditor
            title={note.title}
            summary={note.summary ?? ""}
            cleanedText={note.cleanedText ?? ""}
            on:save={onEditorSave}
            on:cancel={() => (editing = false)}
          />
        {:else if note.cleanedText}
          <div class="cleaned-toolbar">
            {#if note.cleanedEdited}
              <span class="edited-indicator">edited</span>
            {/if}
            <button class="ghost-button" type="button" onclick={startEdit} disabled={loading}>Edit</button>
          </div>
          <MarkdownView markdown={note.cleanedText} />
        {:else}
          <p>Waiting for parser output.</p>
        {/if}
      </section>

      <section class="detail-section" aria-label="Tags">
        <div class="section-head">
          <span>Tags</span>
          <small>{note.tags.length}</small>
        </div>
        <div class="tags">
          {#if note.tags.length === 0}
            <span class="muted">No tags</span>
          {:else}
            {#each note.tags as tag}
              <span class="tag">{tag.name}</span>
            {/each}
          {/if}
        </div>
      </section>

      <section class="detail-section" aria-label="Action items">
        <div class="section-head">
          <span>Actions</span>
          <small>{suggestedActions.length} suggested</small>
        </div>
        <div class="actions">
          {#if suggestedActions.length === 0 && acceptedActions.length === 0 && doneActions.length === 0}
            <p class="muted">No actions</p>
          {/if}

          {#each suggestedActions as action}
            <article class="action-row">
              <button
                class="action-check"
                type="button"
                aria-label={`Accept action: ${action.text}`}
                disabled={loading || busyActionId === action.id}
                onclick={() => dispatch("acceptAction", action.id)}
              >
                OK
              </button>
              <div>
                <p>{action.text}</p>
                {#if actionMeta(action)}
                  <small>{actionMeta(action)}</small>
                {/if}
              </div>
              <button
                class="action-dismiss"
                type="button"
                aria-label={`Dismiss action: ${action.text}`}
                disabled={loading || busyActionId === action.id}
                onclick={() => dispatch("dismissAction", action.id)}
              >
                x
              </button>
            </article>
          {/each}

          {#if acceptedActions.length > 0}
            <div class="section-head muted-head">
              <span>Accepted</span>
            </div>
            {#each acceptedActions as action}
              <article class="action-row">
                <button
                  class="action-check"
                  type="button"
                  aria-label={`Complete action: ${action.text}`}
                  disabled={loading || busyActionId === action.id}
                  onclick={() => dispatch("completeAction", action.id)}
                >
                  OK
                </button>
                <div>
                  <p>{action.text}</p>
                  {#if actionMeta(action)}
                    <small>{actionMeta(action)}</small>
                  {/if}
                </div>
              </article>
            {/each}
          {/if}

          {#if doneActions.length > 0}
            <div class="section-head muted-head">
              <span>Done</span>
            </div>
            {#each doneActions as action}
              <article class="action-row done">
                <span class="action-check checked">OK</span>
                <div>
                  <p>{action.text}</p>
                </div>
                <button
                  class="action-dismiss"
                  type="button"
                  aria-label={`Reopen action: ${action.text}`}
                  disabled={loading || busyActionId === action.id}
                  onclick={() => dispatch("reopenAction", action.id)}
                >
                  x
                </button>
              </article>
            {/each}
          {/if}
        </div>
      </section>
    </div>

    {#if reparseOpen}
      <div class="reparse-backdrop" aria-hidden="true" onclick={closeReparseDialog}></div>
      <div
        class="reparse-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="reparse-title"
      >
        <header>
          <h2 id="reparse-title">Reparse with feedback</h2>
          <button class="dialog-close" type="button" aria-label="Close reparse feedback" onclick={closeReparseDialog}>
            x
          </button>
        </header>
        <label>
          <span>Feedback</span>
          <textarea
            bind:this={reparseFeedbackInput}
            bind:value={reparseFeedback}
            aria-label="Feedback"
            disabled={loading}
            placeholder="What did the parser get wrong?"
            rows="5"
          ></textarea>
        </label>
        <footer>
          <button class="secondary-action" type="button" onclick={closeReparseDialog}>Cancel</button>
          <button class="primary-action" type="button" onclick={dispatchReparseWithFeedback} disabled={loading || !reparseFeedback.trim()}>
            Send feedback
          </button>
        </footer>
      </div>
    {/if}

    {#if reparseConfirmOpen}
      <div class="reparse-backdrop" aria-hidden="true" onclick={cancelReparseConfirm}></div>
      <div class="reparse-dialog" role="dialog" aria-modal="true" aria-labelledby="reparse-confirm-title">
        <header>
          <h2 id="reparse-confirm-title">Discard manual edits?</h2>
        </header>
        <p>This note has manual edits. Reparsing will replace them with new parser output.</p>
        <footer>
          <button class="secondary-action" type="button" onclick={cancelReparseConfirm}>Keep my edits</button>
          <button class="primary-action" type="button" onclick={confirmReparse}>Discard & reparse</button>
        </footer>
      </div>
    {/if}
  {/if}
</section>

<style>
  .note-detail {
    display: flex;
    flex: 1;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    min-height: 0;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    overflow: hidden;
  }

  :global([data-theme="memphis"]) .note-detail {
    background-image: radial-gradient(circle at 1px 1px, color-mix(in srgb, var(--color-text-muted) 18%, transparent) 1px, transparent 0);
    background-size: 12px 12px;
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 22px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  :global([data-theme="memphis"]) .detail-header {
    border-bottom-width: 2px;
  }

  .head-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .status-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
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

  .header-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }

  .ghost-button,
  .delete-button,
  .detail-banner button,
  .tab-row button,
  .action-check,
  .action-dismiss,
  .dialog-close,
  .secondary-action,
  .primary-action {
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
    font-weight: 800;
    cursor: pointer;
  }

  .ghost-button,
  .delete-button {
    min-height: 28px;
    padding: 0 10px;
  }

  .delete-button {
    color: var(--color-status-error);
  }

  .ghost-button:disabled,
  .delete-button:disabled,
  .detail-banner button:disabled,
  .action-check:disabled,
  .action-dismiss:disabled,
  .primary-action:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .detail-scroll {
    flex: 1;
    max-width: 760px;
    min-height: 0;
    overflow-y: auto;
    padding: 22px 28px 60px;
  }

  h1,
  h2,
  p,
  pre {
    margin: 0;
  }

  h1 {
    margin-bottom: 18px;
    color: var(--color-text-primary);
    font-size: 22px;
    font-weight: 800;
    line-height: 1.3;
  }

  :global([data-theme="memphis"]) h1 {
    font-size: 24px;
    letter-spacing: 0;
  }

  .detail-banner {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 2px 10px;
    align-items: center;
    margin-bottom: 18px;
    padding: 10px 12px;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
  }

  .detail-banner strong {
    font-size: 12.5px;
  }

  .detail-banner span {
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12px;
  }

  .detail-banner button {
    grid-row: 1 / span 2;
    grid-column: 2;
    min-height: 26px;
    padding: 0 11px;
  }

  .detail-banner.error {
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 12%, var(--color-surface-1));
  }

  .detail-banner.info {
    color: var(--color-accent-hot);
    background: color-mix(in srgb, var(--color-accent-hot) 12%, var(--color-surface-1));
  }

  :global([data-theme="memphis"]) .detail-banner,
  :global([data-theme="memphis"]) .reparse-dialog,
  :global([data-theme="memphis"]) pre {
    border-width: 2px;
    border-radius: 10px;
    box-shadow: 3px 3px 0 var(--color-border-default);
  }

  .summary-block {
    margin-bottom: 22px;
    padding: 10px 14px;
    border-left: 3px solid var(--color-accent-primary);
    border-radius: 0 8px 8px 0;
    background: color-mix(in srgb, var(--color-accent-primary) 10%, var(--color-surface-1));
  }

  :global([data-theme="memphis"]) .summary-block {
    border-left-width: 4px;
    background: var(--color-surface-2);
  }

  .summary-block span,
  .section-head span,
  .reparse-dialog label span,
  .followup-form label span {
    display: inline-flex;
    color: var(--color-accent-primary);
    font-size: 11px;
    font-weight: 900;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .summary-block p {
    margin-top: 4px;
    color: var(--color-text-primary);
    font-size: 14.5px;
    line-height: 1.5;
  }

  .body-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .tab-row {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: 7px;
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .tab-row {
    border: 2px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .tab-row button {
    min-height: 24px;
    padding: 0 11px;
    border: 0;
    color: var(--color-text-muted);
    background: transparent;
  }

  .tab-row button.active {
    color: var(--color-text-primary);
    background: var(--color-app-bg);
  }

  :global([data-theme="memphis"]) .tab-row button.active {
    color: var(--color-border-strong);
    background: var(--color-status-warning);
  }

  .note-body {
    margin-bottom: 22px;
  }

  .note-body p {
    color: var(--color-text-primary);
    font-size: 14px;
    line-height: 1.65;
    white-space: pre-wrap;
  }

  pre {
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    padding: 14px 16px;
    color: var(--color-text-muted);
    background: var(--color-surface-input);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12.5px;
    line-height: 1.6;
    white-space: pre-wrap;
  }

  .reparse-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: color-mix(in srgb, var(--color-app-bg) 70%, rgba(0, 0, 0, 0.68));
    backdrop-filter: blur(2px);
  }

  .reparse-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    display: grid;
    z-index: 41;
    width: min(560px, calc(100vw - 32px));
    gap: 14px;
    transform: translate(-50%, -50%);
    border: 1px solid var(--color-border-default);
    border-radius: 10px;
    padding: 14px;
    color: var(--color-text-primary);
    background: var(--color-surface-1);
    box-shadow: 0 22px 44px -18px rgba(0, 0, 0, 0.58);
  }

  .reparse-dialog header,
  .reparse-dialog footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .reparse-dialog h2 {
    color: var(--color-text-primary);
    font-size: 15px;
    font-weight: 850;
    line-height: 1.2;
  }

  .dialog-close {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    color: var(--color-text-muted);
    background: transparent;
  }

  .reparse-dialog label {
    display: grid;
    gap: 7px;
  }

  .followup-form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 10px;
    margin-bottom: 18px;
    padding: 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-surface-1) 86%, transparent);
  }

  .followup-fields {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(120px, 180px);
    gap: 8px;
    min-width: 0;
  }

  .followup-form label {
    display: grid;
    gap: 5px;
    min-width: 0;
  }

  .followup-form input {
    width: 100%;
    min-width: 0;
    min-height: 30px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    outline: none;
    padding: 0 9px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
  }

  .followup-form input:focus {
    border-color: var(--color-accent-primary);
  }

  .reparse-dialog textarea {
    width: 100%;
    min-height: 118px;
    resize: vertical;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    outline: none;
    padding: 10px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
    line-height: 1.5;
  }

  .reparse-dialog textarea:focus {
    border-color: var(--color-accent-primary);
  }

  .reparse-dialog footer {
    justify-content: flex-end;
  }

  .secondary-action,
  .primary-action {
    min-height: 30px;
    padding: 0 12px;
  }

  .primary-action {
    color: var(--color-surface-1);
    border-color: var(--color-accent-primary);
    background: var(--color-accent-primary);
  }

  .secondary-action {
    color: var(--color-text-primary);
    background: var(--color-surface-input);
  }

  .detail-section {
    display: grid;
    gap: 10px;
    margin-bottom: 22px;
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-head small {
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 10.5px;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .tag,
  .muted {
    display: inline-flex;
    align-items: center;
    min-height: 22px;
    padding: 0 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 999px;
    color: var(--color-text-muted);
    background: var(--color-surface-1);
    font-size: 11.5px;
    font-weight: 700;
  }

  :global([data-theme="memphis"]) .tag {
    border-width: 2px;
    color: var(--color-text-primary);
  }

  .muted {
    border-color: transparent;
    background: transparent;
  }

  .actions {
    display: grid;
    gap: 2px;
  }

  .action-row {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) 28px;
    align-items: start;
    gap: 10px;
    padding: 9px 10px;
    border-radius: 7px;
  }

  .action-row:hover {
    background: var(--color-surface-2);
  }

  .action-row.done {
    opacity: 0.62;
  }

  .action-row p {
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 700;
    line-height: 1.4;
  }

  .action-row small {
    display: block;
    margin-top: 3px;
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .action-check,
  .action-dismiss {
    display: grid;
    min-width: 0;
    height: 22px;
    place-items: center;
    padding: 0;
    color: transparent;
  }

  .action-check {
    width: 22px;
    margin-top: 0;
  }

  .action-check:hover,
  .action-check.checked {
    color: var(--color-border-strong);
    background: var(--color-status-success);
  }

  .action-dismiss {
    color: var(--color-text-muted);
    opacity: 0;
  }

  .action-row:hover .action-dismiss,
  .action-dismiss:focus-visible {
    opacity: 1;
  }

  .muted-head {
    margin-top: 10px;
    opacity: 0.75;
  }

  .empty-state {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 24px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-mark {
    display: grid;
    width: 56px;
    height: 56px;
    place-items: center;
    margin-bottom: 14px;
    border-radius: 14px;
    color: var(--color-text-primary);
    background: var(--color-surface-2);
    font-weight: 900;
  }

  :global([data-theme="memphis"]) .empty-mark {
    border: 2px solid var(--color-border-default);
    background: var(--color-accent-hot);
    box-shadow: 3px 3px 0 var(--color-border-default);
  }

  .empty-state h2 {
    margin: 0 0 6px;
    color: var(--color-text-primary);
    font-size: 15px;
  }

  .empty-state p {
    max-width: 320px;
    color: var(--color-text-muted);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .cleaned-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .edited-indicator {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    border: 1px solid var(--border-default);
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
  }

  @media (max-width: 700px) {
    .detail-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .detail-scroll {
      padding: 18px 18px 48px;
    }

    .detail-banner {
      grid-template-columns: 1fr;
    }

    .detail-banner button {
      grid-row: auto;
      grid-column: auto;
      justify-self: start;
    }

    .followup-form,
    .followup-fields {
      grid-template-columns: 1fr;
    }
  }
</style>
