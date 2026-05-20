<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { InboxFilters, NoteListItem, ParseStatus, ReviewStatus, Tag } from "$lib/types";

  import StatusBadge from "./StatusBadge.svelte";

  type BadgeTone = "neutral" | "accent" | "hot" | "success" | "warning" | "error";

  type Props = {
    items: NoteListItem[];
    filters: InboxFilters;
    selectedId?: string;
    loading?: boolean;
  };

  let { items, filters, selectedId, loading = false }: Props = $props();
  const availableTags = $derived(uniqueTags(items));

  const dispatch = createEventDispatcher<{
    select: string;
    filter: Partial<InboxFilters>;
  }>();

  function parseLabel(status: ParseStatus): string {
    switch (status) {
      case "queued":
        return "Queued";
      case "parsing":
        return "Parsing";
      case "parsed":
        return "Parsed";
      case "failed":
        return "Failed";
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
        return "Review";
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

  function previewText(item: NoteListItem): string {
    return item.summary ?? item.cleanedText ?? item.rawText;
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

  function updateSearch(event: Event) {
    dispatch("filter", { search: (event.currentTarget as HTMLInputElement).value });
  }

  function updateParseStatus(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value as ParseStatus | "";
    dispatch("filter", { parseStatuses: value ? [value] : [] });
  }

  function updateReviewStatus(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value as ReviewStatus | "";
    dispatch("filter", { reviewStatuses: value ? [value] : [] });
  }

  function updateTag(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    dispatch("filter", { tagIds: value ? [value] : [] });
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
  <div class="list-header">
    <div>
      <p class="eyebrow">Inbox</p>
      <h2>Drive-by captures</h2>
    </div>
    <StatusBadge label={loading ? "Loading" : `${items.length} open`} tone="accent" />
  </div>

  <div class="filters" aria-label="Inbox filters">
    <input
      aria-label="Search notes"
      placeholder="Search notes"
      value={filters.search}
      oninput={updateSearch}
    />
    <select aria-label="Parse status" value={filters.parseStatuses[0] ?? ""} onchange={updateParseStatus}>
      <option value="">Any parse</option>
      <option value="queued">Queued</option>
      <option value="parsing">Parsing</option>
      <option value="parsed">Parsed</option>
      <option value="failed">Failed</option>
    </select>
    <select aria-label="Review status" value={filters.reviewStatuses[0] ?? ""} onchange={updateReviewStatus}>
      <option value="">Any review</option>
      <option value="none">No review</option>
      <option value="needs_review">Needs review</option>
      <option value="reviewed">Reviewed</option>
    </select>
    <select aria-label="Tag, project, topic, or person" value={filters.tagIds[0] ?? ""} onchange={updateTag}>
      <option value="">Any tag</option>
      {#each availableTags as tag}
        <option value={tag.id}>{tag.kind}: {tag.name}</option>
      {/each}
    </select>
  </div>

  <div class="items">
    {#if items.length === 0}
      <p class="empty-state">No notes</p>
    {/if}

    {#each items as item}
      <button
        class:selected={item.id === selectedId}
        class="inbox-item"
        type="button"
        onclick={() => dispatch("select", item.id)}
      >
        <div class="item-topline">
          <span class="source">{item.captureSource}</span>
          <span class="captured-at">{formatCapturedAt(item.createdAt)}</span>
        </div>
        <div class="item-title-row">
          <h3>{item.title}</h3>
          <div class="badges" aria-label="Note statuses">
            <StatusBadge label={parseLabel(item.parseStatus)} tone={parseTone(item.parseStatus)} />
            <StatusBadge label={reviewLabel(item.reviewStatus)} tone={reviewTone(item.reviewStatus)} />
          </div>
        </div>
        <p>{previewText(item)}</p>
      </button>
    {/each}
  </div>
</section>

<style>
  .inbox-list {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-2);
  }

  .filters {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 10px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  .filters input,
  .filters select {
    min-width: 0;
    min-height: 30px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 0 8px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
  }

  .filters input {
    grid-column: 1 / -1;
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
  h3,
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 17px;
    line-height: 1.15;
  }

  .items {
    display: grid;
    gap: 1px;
    min-height: 0;
    overflow: auto;
    background: var(--color-border-default);
  }

  .inbox-item {
    display: grid;
    gap: 7px;
    width: 100%;
    padding: 12px 14px;
    border: 0;
    background: var(--color-surface-1);
    border-left: 3px solid transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .inbox-item.selected,
  .inbox-item:hover,
  .inbox-item:focus-visible {
    border-left-color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-surface-2) 78%, var(--color-accent-primary));
  }

  .item-topline,
  .item-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-width: 0;
  }

  .badges {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 5px;
    flex-wrap: wrap;
  }

  .source {
    color: var(--color-text-primary);
    font-size: 12px;
    font-weight: 800;
    line-height: 1.1;
  }

  .captured-at {
    color: var(--color-text-muted);
    font-size: 11px;
    line-height: 1.1;
    white-space: nowrap;
  }

  h3 {
    min-width: 0;
    color: var(--color-text-primary);
    font-size: 14px;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }

  .inbox-item p {
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.35;
  }

  .empty-state {
    margin: 0;
    padding: 16px 14px;
    color: var(--color-text-muted);
    background: var(--color-surface-1);
    font-size: 12px;
  }

  @media (max-width: 720px) {
    .list-header {
      align-items: flex-start;
    }

    .item-title-row {
      align-items: flex-start;
      flex-direction: column;
      gap: 7px;
    }
  }
</style>
