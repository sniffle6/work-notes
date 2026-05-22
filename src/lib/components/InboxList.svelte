<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { InboxFilters, NoteListItem, ParseStatus, ReviewStatus, Tag } from "$lib/types";

  type Props = {
    items: NoteListItem[];
    filters: InboxFilters;
    selectedId?: string;
    loading?: boolean;
  };

  let { items, filters, selectedId, loading = false }: Props = $props();
  let mode = $state<"notes" | "actions">("notes");

  const availableTags = $derived(uniqueTags(items));
  const suggestedActionCount = $derived(items.reduce((total, item) => total + item.suggestedActionItemCount, 0));
  const activeFilter = $derived(filterKey(filters));
  const visibleItems = $derived(
    mode === "actions" ? items.filter((item) => item.suggestedActionItemCount > 0) : items,
  );

  const dispatch = createEventDispatcher<{
    select: string;
    filter: Partial<InboxFilters>;
  }>();

  function filterKey(nextFilters: InboxFilters): "all" | "review" | "failed" {
    if (nextFilters.parseStatuses.includes("failed")) {
      return "failed";
    }
    if (nextFilters.reviewStatuses.includes("needs_review")) {
      return "review";
    }
    return "all";
  }

  function setQuickFilter(nextFilter: "all" | "review" | "failed") {
    if (nextFilter === "review") {
      dispatch("filter", { reviewStatuses: ["needs_review"], parseStatuses: [] });
      return;
    }

    if (nextFilter === "failed") {
      dispatch("filter", { parseStatuses: ["failed"], reviewStatuses: [] });
      return;
    }

    dispatch("filter", { parseStatuses: [], reviewStatuses: [] });
  }

  function updateSearch(event: Event) {
    dispatch("filter", { search: (event.currentTarget as HTMLInputElement).value });
  }

  function updateTag(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    dispatch("filter", { tagIds: value ? [value] : [] });
  }

  function statusLabel(status: ParseStatus, review: ReviewStatus): string {
    if (status === "failed") {
      return "Parse failed";
    }
    if (status === "parsing") {
      return "Parsing";
    }
    if (status === "queued") {
      return "Queued";
    }
    if (review === "needs_review") {
      return "Needs review";
    }
    if (review === "reviewed") {
      return "Reviewed";
    }
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

  function previewText(item: NoteListItem): string {
    return item.cleanedText ?? item.summary ?? item.rawText;
  }

  function formatCapturedAt(value: string): string {
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

  function captureSourceLabel(value: string): string {
    return value.replace(/_/g, " ");
  }

  function uniqueTags(notes: NoteListItem[]): Tag[] {
    const byId = new Map<string, Tag>();
    for (const note of notes) {
      for (const tag of note.tags) {
        byId.set(tag.id, tag);
      }
    }
    return [...byId.values()].sort((left, right) => `${left.kind}:${left.name}`.localeCompare(`${right.kind}:${right.name}`));
  }
</script>

<section class="inbox-list" aria-label="Inbox notes">
  <header class="list-header">
    <div class="mode-row">
      <div class="mode-toggle" aria-label="Inbox mode">
        <button class:active={mode === "notes"} type="button" onclick={() => (mode = "notes")}>Notes</button>
        <button class:active={mode === "actions"} type="button" onclick={() => (mode = "actions")}>
          Actions
          <span>{suggestedActionCount}</span>
        </button>
      </div>
      <span class="load-state">{loading ? "Loading" : `${items.length} open`}</span>
    </div>

    <label class="search-row">
      <span aria-hidden="true">Search</span>
      <input
        aria-label={mode === "actions" ? "Search actions" : "Search notes"}
        placeholder={mode === "actions" ? "Search actions, owners" : "Search notes, people, tags"}
        value={filters.search}
        oninput={updateSearch}
      />
      <kbd>/</kbd>
    </label>

    {#if mode === "notes"}
      <div class="filter-row" aria-label="Inbox filters">
        <button class:active={activeFilter === "all"} type="button" onclick={() => setQuickFilter("all")}>
          All <span>{items.length}</span>
        </button>
        <button class:active={activeFilter === "review"} type="button" onclick={() => setQuickFilter("review")}>
          Needs review <span>{items.filter((item) => item.reviewStatus === "needs_review").length}</span>
        </button>
        <button class:active={activeFilter === "failed"} type="button" onclick={() => setQuickFilter("failed")}>
          Failed <span>{items.filter((item) => item.parseStatus === "failed").length}</span>
        </button>
      </div>

      {#if availableTags.length > 0}
        <label class="tag-filter">
          <span>Tag</span>
          <select aria-label="Tag, project, topic, or person" value={filters.tagIds[0] ?? ""} onchange={updateTag}>
            <option value="">Any tag</option>
            {#each availableTags as tag}
              <option value={tag.id}>{tag.kind}: {tag.name}</option>
            {/each}
          </select>
        </label>
      {/if}
    {/if}
  </header>

  <div class:actions-scroll={mode === "actions"} class="items">
    {#if visibleItems.length === 0}
      <div class="empty-state">
        <div class="empty-mark">WN</div>
        <h2>{mode === "actions" ? "No suggested actions" : "No notes"}</h2>
        <p>{mode === "actions" ? "Parser suggestions will appear here after capture." : "Press the hotkey to capture your first one."}</p>
      </div>
    {/if}

    {#if mode === "notes"}
      {#each visibleItems as item}
        <button
          class:selected={item.id === selectedId}
          class="inbox-item"
          type="button"
          onclick={() => dispatch("select", item.id)}
        >
          <div class="item-topline">
            <span class={`status-dot ${statusClass(item.parseStatus, item.reviewStatus)}`} title={statusLabel(item.parseStatus, item.reviewStatus)}></span>
            <span class="source">{captureSourceLabel(item.captureSource)}</span>
            <span class="captured-at">{formatCapturedAt(item.createdAt)}</span>
          </div>
          <h3>{item.title}</h3>
          <p>{previewText(item)}</p>
          <div class="item-meta">
            {#each item.tags.slice(0, 3) as tag}
              <span class="tag-chip">{tag.name}</span>
            {/each}
            {#if item.tags.length > 3}
              <span class="more-chip">+{item.tags.length - 3}</span>
            {/if}
            {#if item.suggestedActionItemCount > 0}
              <span class="action-count">{item.suggestedActionItemCount} actions</span>
            {/if}
          </div>
        </button>
      {/each}
    {:else}
      <div class="action-groups">
        <div class="group-head">
          <span>Suggested</span>
          <strong>{suggestedActionCount}</strong>
        </div>
        {#each visibleItems as item}
          <button
            class:selected={item.id === selectedId}
            class="action-row"
            type="button"
            onclick={() => dispatch("select", item.id)}
          >
            <span class="check-box" aria-hidden="true"></span>
            <span>
              <strong>{item.suggestedActionItemCount} suggested actions</strong>
              <small>from "{item.title}"</small>
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</section>

<style>
  .inbox-list {
    display: flex;
    flex-direction: column;
    width: 348px;
    min-width: 0;
    min-height: 100vh;
    border-right: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-app-bg) 86%, var(--color-surface-1));
    overflow: hidden;
  }

  :global([data-theme="memphis"]) .inbox-list {
    border-right-width: 2px;
    background-image: repeating-linear-gradient(0deg, transparent 0 23px, color-mix(in srgb, var(--color-text-muted) 8%, transparent) 23px 24px);
  }

  .list-header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px 8px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 78%, transparent);
  }

  :global([data-theme="memphis"]) .list-header {
    border-bottom-width: 2px;
    background: color-mix(in srgb, var(--color-surface-1) 92%, transparent);
  }

  .mode-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .mode-toggle {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: 7px;
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .mode-toggle {
    border: 2px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .mode-toggle button,
  .filter-row button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: 0;
    border-radius: 5px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }

  .mode-toggle button {
    height: 24px;
    padding: 0 10px;
  }

  .mode-toggle button.active {
    color: var(--color-text-primary);
    background: var(--color-app-bg);
  }

  :global([data-theme="memphis"]) .mode-toggle button.active {
    color: var(--color-border-strong);
    background: var(--color-accent-hot);
  }

  .mode-toggle span,
  .filter-row span,
  .group-head strong {
    display: inline-grid;
    min-width: 16px;
    height: 16px;
    place-items: center;
    padding: 0 5px;
    border-radius: 999px;
    color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 13%, var(--color-surface-2));
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 10.5px;
  }

  .load-state {
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
  }

  .search-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-text-muted);
    background: var(--color-surface-input);
  }

  :global([data-theme="memphis"]) .search-row {
    border-width: 2px;
    border-radius: 8px;
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .search-row span {
    font-size: 11px;
    font-weight: 800;
  }

  .search-row input {
    min-width: 0;
    border: 0;
    outline: none;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 13px;
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

  .filter-row {
    display: flex;
    gap: 3px;
    overflow-x: auto;
  }

  .filter-row button {
    min-height: 22px;
    padding: 0 9px;
    border: 1px solid transparent;
    border-radius: 999px;
    white-space: nowrap;
  }

  .filter-row button:hover,
  .filter-row button.active {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .filter-row button.active {
    border-color: var(--color-border-default);
    background: var(--color-surface-2);
  }

  .tag-filter {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .tag-filter select {
    min-width: 0;
    height: 28px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
  }

  .items {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 2px 0;
  }

  :global([data-theme="memphis"]) .items {
    padding: 8px 10px 16px;
  }

  .items.actions-scroll {
    padding-top: 6px;
  }

  .inbox-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    padding: 11px 14px 12px;
    border: 0;
    border-left: 2px solid transparent;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .inbox-item + .inbox-item {
    border-top: 1px solid var(--color-border-default);
  }

  .inbox-item:hover,
  .inbox-item.selected {
    border-left-color: var(--color-accent-primary);
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .inbox-item {
    margin: 0 0 8px;
    padding: 12px 14px 13px;
    border: 2px solid var(--color-border-default);
    border-radius: 10px;
    background: var(--color-surface-1);
    box-shadow: 3px 3px 0 var(--color-border-default);
  }

  :global([data-theme="memphis"]) .inbox-item + .inbox-item {
    border-top: 2px solid var(--color-border-default);
  }

  :global([data-theme="memphis"]) .inbox-item.selected {
    transform: translate(-1px, -1px);
    background: color-mix(in srgb, var(--color-accent-primary) 14%, var(--color-surface-1));
    box-shadow: 4px 4px 0 var(--color-border-default);
  }

  .item-topline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
  }

  .captured-at {
    margin-left: auto;
  }

  .status-dot {
    display: inline-block;
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

  h3,
  p,
  h2 {
    margin: 0;
  }

  h3 {
    display: -webkit-box;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 750;
    line-height: 1.35;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .inbox-item p {
    display: -webkit-box;
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.45;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .item-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin-top: 2px;
  }

  .tag-chip,
  .more-chip,
  .action-count {
    display: inline-flex;
    align-items: center;
    min-height: 18px;
    padding: 0 7px;
    border: 1px solid var(--color-border-default);
    border-radius: 999px;
    color: var(--color-text-muted);
    background: var(--color-surface-1);
    font-size: 10.5px;
    font-weight: 700;
  }

  :global([data-theme="memphis"]) .tag-chip,
  :global([data-theme="memphis"]) .more-chip {
    color: var(--color-text-primary);
  }

  .action-count {
    margin-left: auto;
    border-color: transparent;
    color: var(--color-accent-primary);
    background: transparent;
  }

  .action-groups {
    display: grid;
    gap: 8px;
  }

  .group-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 8px 6px;
    color: var(--color-text-muted);
    font-size: 10.5px;
    font-weight: 800;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }

  :global([data-theme="memphis"]) .group-head {
    color: var(--color-accent-primary);
  }

  .action-row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: start;
    gap: 9px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 7px;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .action-row:hover,
  .action-row.selected {
    background: var(--color-surface-2);
  }

  .check-box {
    width: 16px;
    height: 16px;
    margin-top: 2px;
    border: 1.5px solid var(--color-border-default);
    border-radius: 4px;
    background: var(--color-surface-1);
  }

  .action-row strong {
    display: block;
    color: var(--color-text-primary);
    font-size: 13px;
    line-height: 1.35;
  }

  .action-row small {
    display: block;
    margin-top: 3px;
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-state {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 240px;
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
    color: var(--color-text-primary);
    font-size: 14px;
  }

  .empty-state p {
    margin-top: 4px;
    font-size: 12.5px;
  }

  @media (max-width: 980px) {
    .inbox-list {
      width: 100%;
      min-height: 360px;
      border-right: 0;
      border-bottom: 1px solid var(--color-border-default);
    }
  }
</style>
