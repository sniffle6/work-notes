<script lang="ts">
  import Check from "@lucide/svelte/icons/check";
  import Copy from "@lucide/svelte/icons/copy";
  import Download from "@lucide/svelte/icons/download";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import X from "@lucide/svelte/icons/x";
  import Play from "@lucide/svelte/icons/play";
  import ZoomIn from "@lucide/svelte/icons/zoom-in";
  import ZoomOut from "@lucide/svelte/icons/zoom-out";
  import { createEventDispatcher, onDestroy, tick } from "svelte";
  import { formatFileSize } from "$lib/file-size";
  import { zoomIn, zoomLabel, zoomOut } from "$lib/image-zoom";
  import { isVideoAttachment } from "$lib/media-type";
  import { buildTicketPrompt } from "$lib/jira-prompt";
  import { isTypingTarget } from "$lib/ui-prefs";
  import type {
    JiraAttachment,
    JiraAttachmentDownload,
    JiraAttachmentView,
    JiraIssueDetail,
    JiraIssueEdit,
    JiraTransition,
  } from "$lib/types";

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
    /** The open ticket's attachments that are already saved to disk. */
    attachmentDownload?: JiraAttachmentDownload | null;
    downloadingAttachments?: boolean;
    /** Preview image URLs by attachment id. */
    attachmentThumbnails?: Record<string, string>;
    /** The image open in the viewer, if any. */
    attachmentViewer?: JiraAttachmentView | null;
    /** The attachment being fetched for the viewer, if any. */
    openingAttachmentId?: string | null;
    /** Builds the prompt to copy. Defaults to the ticket text without attachment files. */
    preparePrompt?: (issue: JiraIssueDetail) => Promise<string>;
  };

  let {
    issue,
    loading = false,
    saving = false,
    attachmentDownload = null,
    downloadingAttachments = false,
    attachmentThumbnails = {},
    attachmentViewer = null,
    openingAttachmentId = null,
    preparePrompt,
  }: Props = $props();
  let preparingPrompt = $state(false);
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
    /** An attachment id, or null for every attachment. */
    downloadAttachments: string | null;
    /** The path of a downloaded file to show in its folder. */
    revealAttachment: string;
    /** The id of an image attachment to open in the viewer. */
    openAttachment: string;
    closeAttachmentViewer: void;
  }>();

  const ACTUAL_SIZE = 1;
  // The viewed image's scale relative to its real size; null fits it to the viewer.
  let zoom = $state<number | null>(null);
  let naturalWidth = $state(0);
  let viewerImage = $state<HTMLImageElement | null>(null);
  const zoomedWidth = $derived(zoom !== null && naturalWidth > 0 ? Math.round(naturalWidth * zoom) : null);

  const PRIMARY_MOUSE_BUTTON = 0;
  // Movement below this is a click, not a drag.
  const PAN_THRESHOLD_PX = 3;
  let viewerStage = $state<HTMLDivElement | null>(null);
  // Where a drag started, and how far the stage was scrolled then.
  let pan = $state<{ startX: number; startY: number; scrollLeft: number; scrollTop: number } | null>(null);
  let panMoved = false;

  $effect(() => {
    // Each image opens fitted to the viewer.
    void attachmentViewer?.url;
    zoom = null;
    naturalWidth = 0;
  });

  const savedPathById = $derived(new Map((attachmentDownload?.files ?? []).map((file) => [file.id, file.path])));

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
    if (!issue || preparingPrompt) {
      return;
    }
    preparingPrompt = true;
    try {
      const prompt = preparePrompt ? await preparePrompt(issue) : buildTicketPrompt(issue);
      await navigator.clipboard.writeText(prompt);
      showCopyState("copied");
    } catch {
      showCopyState("failed");
    } finally {
      preparingPrompt = false;
    }
  }

  /** The image's scale right now: the chosen zoom, or the scale fitting gives it. */
  function currentScale(): number {
    if (zoom !== null) {
      return zoom;
    }
    return viewerImage && naturalWidth > 0 && viewerImage.clientWidth > 0
      ? viewerImage.clientWidth / naturalWidth
      : ACTUAL_SIZE;
  }

  function startPan(event: MouseEvent) {
    if (event.button !== PRIMARY_MOUSE_BUTTON || !viewerStage) {
      return;
    }
    // Stops the browser from selecting text or dragging the image instead.
    event.preventDefault();
    panMoved = false;
    pan = {
      startX: event.clientX,
      startY: event.clientY,
      scrollLeft: viewerStage.scrollLeft,
      scrollTop: viewerStage.scrollTop,
    };
  }

  function movePan(event: MouseEvent) {
    if (!pan || !viewerStage) {
      return;
    }
    const deltaX = event.clientX - pan.startX;
    const deltaY = event.clientY - pan.startY;
    if (Math.abs(deltaX) > PAN_THRESHOLD_PX || Math.abs(deltaY) > PAN_THRESHOLD_PX) {
      panMoved = true;
    }
    viewerStage.scrollLeft = pan.scrollLeft - deltaX;
    viewerStage.scrollTop = pan.scrollTop - deltaY;
  }

  function zoomWithWheel(event: WheelEvent) {
    // Without Ctrl the wheel scrolls a zoomed image.
    if (!event.ctrlKey || event.deltaY === 0) {
      return;
    }
    event.preventDefault();
    const anchor = imagePointAt(event.clientX, event.clientY);
    zoom = event.deltaY < 0 ? zoomIn(currentScale()) : zoomOut(currentScale());
    if (anchor) {
      void keepPointUnderCursor(anchor, event.clientX, event.clientY);
    }
  }

  /** Where a window position falls on the image, as fractions of its width and height. */
  function imagePointAt(clientX: number, clientY: number): { x: number; y: number } | null {
    const rect = viewerImage?.getBoundingClientRect();
    if (!rect || rect.width === 0 || rect.height === 0) {
      return null;
    }
    const withinImage = (fraction: number) => Math.min(1, Math.max(0, fraction));
    return {
      x: withinImage((clientX - rect.left) / rect.width),
      y: withinImage((clientY - rect.top) / rect.height),
    };
  }

  /** Scrolls the stage so the image point is back at the window position once the new size is laid out. */
  async function keepPointUnderCursor(point: { x: number; y: number }, clientX: number, clientY: number) {
    await tick();
    const rect = viewerImage?.getBoundingClientRect();
    if (!rect || !viewerStage) {
      return;
    }
    viewerStage.scrollLeft += rect.left + point.x * rect.width - clientX;
    viewerStage.scrollTop += rect.top + point.y * rect.height - clientY;
  }

  function handleViewerKey(event: KeyboardEvent) {
    if (!attachmentViewer) {
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("closeAttachmentViewer");
      return;
    }
    // Only an image zooms; a video keeps its keys for its own controls.
    if (attachmentViewer.kind !== "image") {
      return;
    }
    if (event.ctrlKey || event.altKey || event.metaKey || isTypingTarget(event.target)) {
      return;
    }
    if (event.key === "+" || event.key === "=") {
      zoom = zoomIn(currentScale());
    } else if (event.key === "-") {
      zoom = zoomOut(currentScale());
    } else if (event.key === "0") {
      zoom = null;
    }
  }

  function attachmentMeta(attachment: JiraAttachment): string {
    return [
      attachment.size > 0 ? formatFileSize(attachment.size) : "",
      attachment.author ?? "",
      formatDate(attachment.created),
    ]
      .filter(Boolean)
      .join(" · ");
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
          title="Copy a prompt with this ticket's details to paste into your coding agent. Attachments are downloaded first."
          disabled={preparingPrompt}
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

      {#if issue.attachments.length > 0}
        {@const firstSavedPath = attachmentDownload?.files[0]?.path}
        <section class="detail-section" aria-label="Attachments">
          <div class="section-head">
            <h2>Attachments ({issue.attachments.length})</h2>
            <div class="section-actions">
              {#if firstSavedPath}
                <button
                  class="ghost-button"
                  type="button"
                  title={attachmentDownload?.folder}
                  onclick={() => dispatch("revealAttachment", firstSavedPath)}
                >
                  <FolderOpen size={14} aria-hidden="true" />
                  Show folder
                </button>
              {/if}
              <button
                class="ghost-button"
                type="button"
                disabled={downloadingAttachments}
                title="Save every attachment to Downloads\Work Notes\{issue.key}"
                onclick={() => dispatch("downloadAttachments", null)}
              >
                <Download size={14} aria-hidden="true" />
                {downloadingAttachments ? "Downloading" : "Download all"}
              </button>
            </div>
          </div>
          <ul class="attachment-list">
            {#each issue.attachments as attachment (attachment.id)}
              {@const savedPath = savedPathById.get(attachment.id)}
              {@const thumbnail = attachmentThumbnails[attachment.id]}
              <li class="attachment">
                <span class="attachment-main">
                  {#if thumbnail}
                    <button
                      class="thumbnail-button"
                      type="button"
                      disabled={openingAttachmentId !== null || (!savedPath && downloadingAttachments)}
                      aria-label={`Open ${attachment.filename}`}
                      title={savedPath ? "Open in the app" : "Download and open in the app"}
                      onclick={() => dispatch("openAttachment", attachment.id)}
                    >
                      <img src={thumbnail} alt="" />
                    </button>
                  {:else if isVideoAttachment(attachment)}
                    <button
                      class="ghost-button"
                      type="button"
                      disabled={openingAttachmentId !== null || (!savedPath && downloadingAttachments)}
                      aria-label={`Open ${attachment.filename}`}
                      title={savedPath ? "Play in the app" : "Download and play in the app"}
                      onclick={() => dispatch("openAttachment", attachment.id)}
                    >
                      <Play size={14} aria-hidden="true" />
                    </button>
                  {/if}
                  <span class="attachment-name" title={attachment.filename}>{attachment.filename}</span>
                </span>
                <span class="attachment-meta">{attachmentMeta(attachment)}</span>
                {#if savedPath}
                  <button
                    class="ghost-button"
                    type="button"
                    aria-label={`Show ${attachment.filename} in folder`}
                    title={savedPath}
                    onclick={() => dispatch("revealAttachment", savedPath)}
                  >
                    <FolderOpen size={14} aria-hidden="true" />
                  </button>
                {:else}
                  <button
                    class="ghost-button"
                    type="button"
                    disabled={downloadingAttachments}
                    aria-label={`Download ${attachment.filename}`}
                    title="Download"
                    onclick={() => dispatch("downloadAttachments", attachment.id)}
                  >
                    <Download size={14} aria-hidden="true" />
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
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

    {#if attachmentViewer}
      {@const viewed = attachmentViewer}
      <!-- Escape closes the viewer from the window key handler. -->
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <div
        class="viewer"
        role="dialog"
        aria-modal="true"
        aria-label={viewed.filename}
        tabindex="-1"
        onclick={(event) => {
          if (event.target === event.currentTarget) {
            dispatch("closeAttachmentViewer");
          }
        }}
      >
        <header class="viewer-head">
          <span class="viewer-name" title={viewed.path}>{viewed.filename}</span>
          {#if viewed.kind === "image"}
          <button
            class="ghost-button"
            type="button"
            aria-label="Zoom out"
            title="Zoom out (-)"
            onclick={() => (zoom = zoomOut(currentScale()))}
          >
            <ZoomOut size={14} aria-hidden="true" />
          </button>
          <button
            class="ghost-button zoom-level"
            type="button"
            aria-label="Fit to window"
            title="Fit to window (0)"
            onclick={() => (zoom = null)}
          >
            {zoomLabel(zoom)}
          </button>
          <button
            class="ghost-button"
            type="button"
            aria-label="Zoom in"
            title="Zoom in (+)"
            onclick={() => (zoom = zoomIn(currentScale()))}
          >
            <ZoomIn size={14} aria-hidden="true" />
          </button>
          {/if}
          <button
            class="ghost-button"
            type="button"
            aria-label="Show in folder"
            title={viewed.path}
            onclick={() => dispatch("revealAttachment", viewed.path)}
          >
            <FolderOpen size={14} aria-hidden="true" />
          </button>
          <button
            class="ghost-button"
            type="button"
            aria-label="Close"
            title="Close (Esc)"
            onclick={() => dispatch("closeAttachmentViewer")}
          >
            <X size={14} aria-hidden="true" />
          </button>
        </header>
        {#if viewed.kind === "video"}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div
          class="viewer-stage"
          onclick={(event) => {
            if (event.target === event.currentTarget) {
              dispatch("closeAttachmentViewer");
            }
          }}
        >
          <!-- An attached recording has no captions to offer. -->
          <!-- svelte-ignore a11y_media_has_caption -->
          <video class="viewer-video" src={viewed.url} controls autoplay></video>
        </div>
        {:else}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div
          class="viewer-stage"
          class:panning={pan !== null}
          bind:this={viewerStage}
          onclick={(event) => {
            // The click that ends a drag is not a request to close.
            if (panMoved) {
              panMoved = false;
              return;
            }
            if (event.target === event.currentTarget) {
              dispatch("closeAttachmentViewer");
            }
          }}
          onmousedown={startPan}
          onwheel={zoomWithWheel}
        >
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <img
            class="viewer-image"
            class:zoomed={zoomedWidth !== null}
            bind:this={viewerImage}
            src={viewed.url}
            alt={viewed.filename}
            draggable="false"
            style:width={zoomedWidth === null ? null : `${zoomedWidth}px`}
            onload={() => (naturalWidth = viewerImage?.naturalWidth ?? 0)}
            ondblclick={() => (zoom = zoom === null ? ACTUAL_SIZE : null)}
          />
        </div>
        {/if}
      </div>
    {/if}
  {/if}
</section>

<svelte:window onkeydown={handleViewerKey} onmousemove={movePan} onmouseup={() => (pan = null)} />

<style>
  .jira-detail {
    position: relative;
    display: flex;
    min-width: 0;
    min-height: 0;
    height: 100%;
    flex-direction: column;
    color: var(--color-text-primary);
    background: var(--color-app-bg-paint, var(--color-app-bg));
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

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-width: 0;
    margin-bottom: 6px;
  }

  .section-head h2 {
    margin-bottom: 0;
    white-space: nowrap;
  }

  .section-actions {
    display: flex;
    flex: none;
    gap: 4px;
  }

  .attachment-list {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .attachment {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 12px;
    min-height: 30px;
  }

  .attachment-main {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .thumbnail-button {
    flex: none;
    min-height: 0;
    margin: 4px 0;
    padding: 0;
    border-color: var(--rule);
    overflow: hidden;
  }

  .thumbnail-button:hover:not(:disabled) {
    border-color: var(--color-accent-hot);
  }

  .thumbnail-button img {
    display: block;
    width: 72px;
    height: 48px;
    object-fit: cover;
  }

  /* The viewer covers the ticket pane, so the list and agent stay usable. */
  .viewer {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    min-height: 0;
    flex-direction: column;
    background: color-mix(in srgb, var(--color-app-bg) 94%, transparent);
  }

  .viewer-head {
    display: flex;
    flex: none;
    align-items: center;
    gap: 4px;
    min-height: 44px;
    padding: 6px 12px 6px 24px;
    border-bottom: 1px solid var(--rule);
    background: var(--color-app-bg-paint, var(--color-app-bg));
  }

  .viewer-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .zoom-level {
    min-width: 52px;
    justify-content: center;
    font-family: var(--font-mono);
  }

  /* Scrolls a zoomed image; a fitted one is centred by its auto margins. */
  .viewer-stage {
    display: flex;
    flex: 1 1 auto;
    min-height: 0;
    padding: 24px;
    overflow: auto;
  }

  .viewer-image {
    max-width: 100%;
    max-height: 100%;
    margin: auto;
    object-fit: contain;
    cursor: zoom-in;
  }

  .viewer-video {
    max-width: 100%;
    max-height: 100%;
    margin: auto;
  }

  .viewer-image.zoomed {
    flex: none;
    max-width: none;
    max-height: none;
    cursor: grab;
  }

  .viewer-stage.panning,
  .viewer-stage.panning .viewer-image {
    cursor: grabbing;
  }

  .attachment-name {
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .attachment-meta {
    color: var(--color-text-muted);
    font-size: 12px;
    white-space: nowrap;
  }

  .comment {
    display: grid;
    gap: 8px;
    padding: 14px 10px;
    border-top: 1px solid var(--rule);
  }

  /* Every other comment is tinted from the text colour, so it follows the theme. */
  .comment:nth-of-type(even) {
    background: color-mix(in srgb, var(--color-text-muted) 7%, transparent);
  }

  .comment:last-of-type {
    margin-bottom: 8px;
    border-bottom: 1px solid var(--rule);
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
