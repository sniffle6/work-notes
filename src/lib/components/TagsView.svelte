<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { buildTagDetail, buildTags, matchesTagSearch, type TagSummary } from "$lib/tags";
  import { formatRelativeDate, noteStatusClass, noteStatusLabel } from "$lib/note-presentation";
  import type { NoteListItem } from "$lib/types";

  type Props = {
    notes: NoteListItem[];
    loadingNotes?: boolean;
  };

  let { notes, loadingNotes = false }: Props = $props();

  let query = $state("");
  let selectedKey = $state<string | null>(null);

  const tags = $derived(buildTags(notes));
  const visibleTags = $derived(tags.filter((tag) => matchesTagSearch(tag, query)));
  const activeKey = $derived(
    selectedKey && visibleTags.some((tag) => tag.key === selectedKey)
      ? selectedKey
      : visibleTags[0]?.key ?? null,
  );
  const detail = $derived(activeKey ? buildTagDetail(activeKey, notes) : null);

  const dispatch = createEventDispatcher<{ openNote: string }>();

  $effect(() => {
    if (tags.length === 0) {
      selectedKey = null;
      return;
    }
    if (!selectedKey || !tags.some((tag) => tag.key === selectedKey)) {
      selectedKey = tags[0].key;
    }
  });

  function tagRowLabel(tag: TagSummary): string {
    return `Select ${tag.name}, ${tag.kind}, ${noteCountLabel(tag.noteCount)}`;
  }

  function noteCountLabel(count: number): string {
    return `${count} ${count === 1 ? "note" : "notes"}`;
  }

  function notePreview(note: NoteListItem): string {
    const text = (note.summary ?? note.cleanedText ?? "").trim();
    if (!text) {
      return "";
    }
    return text.length > 90 ? `${text.slice(0, 88)}…` : text;
  }

  function selectTag(key: string) {
    selectedKey = key;
  }
</script>

<section class="tags-view" aria-label="Tags">
  <section class="tags-list" aria-label="Tag list">
    <header class="tags-list-head">
      <div class="head-row">
        <h1>Tags</h1>
        <span class="load-state">{loadingNotes ? "Loading" : `${tags.length} ${tags.length === 1 ? "tag" : "tags"}`}</span>
      </div>
      <label class="search-row">
        <span aria-hidden="true">Search</span>
        <input aria-label="Search tags" placeholder="Find a tag" bind:value={query} />
        <kbd>/</kbd>
      </label>
    </header>

    <div class="tags-scroll">
      {#if visibleTags.length === 0}
        <div class="empty-state">
          <strong>{tags.length === 0 ? "No tags yet" : "No tags match"}</strong>
          <span>{tags.length === 0 ? "Tags from your notes will appear here." : "Try another search."}</span>
        </div>
      {:else}
        {#each visibleTags as tag}
          <button
            class="tag-row"
            class:selected={activeKey === tag.key}
            type="button"
            aria-current={activeKey === tag.key ? "page" : undefined}
            aria-label={tagRowLabel(tag)}
            onclick={() => selectTag(tag.key)}
          >
            <span class="tag-row-head">
              <span class={`kind-badge kind-${tag.kind}`}>{tag.kind}</span>
              <span class="tag-name">{tag.name}</span>
              <span class="tag-count">{tag.noteCount}</span>
            </span>
            <span class="tag-meta">last used {formatRelativeDate(tag.lastUsedAt)}</span>
          </button>
        {/each}
      {/if}
    </div>
  </section>

  <section class="tag-detail" aria-label="Selected tag">
    {#if !detail}
      <div class="detail-empty">
        <div class="empty-mark">WN</div>
        <h2>No tag selected</h2>
        <p>Pick a tag to see the notes it appears on.</p>
      </div>
    {:else}
      <header class="detail-head">
        <div class="detail-head-meta">
          <span class={`kind-badge lg kind-${detail.tag.kind}`}>{detail.tag.kind}</span>
          <div>
            <h2>{detail.tag.name}</h2>
            <p>{noteCountLabel(detail.tag.noteCount)} · last used {formatRelativeDate(detail.tag.lastUsedAt)}</p>
          </div>
        </div>
      </header>

      <div class="detail-scroll">
        <section class="detail-section" aria-label="Tagged notes">
          <div class="section-head">
            <h3>Notes</h3>
            <span>{detail.notes.length}</span>
          </div>

          {#if detail.notes.length === 0}
            <div class="tag-empty">No notes with this tag yet.</div>
          {:else}
            <div class="tag-notes">
              {#each detail.notes as note}
                {@const preview = notePreview(note)}
                <button
                  class="tag-note"
                  type="button"
                  aria-label={`Open note: ${note.title}`}
                  onclick={() => dispatch("openNote", note.id)}
                >
                  <span
                    class={`status-dot ${noteStatusClass(note.parseStatus, note.reviewStatus)}`}
                    title={noteStatusLabel(note.parseStatus, note.reviewStatus)}
                  ></span>
                  <span class="note-body">
                    <span class="note-title">{note.title}</span>
                    {#if preview}<span class="note-preview">{preview}</span>{/if}
                    <span class="note-when">{formatRelativeDate(note.createdAt)}</span>
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        </section>

        {#if detail.relatedTags.length > 0}
          <section class="detail-section" aria-label="Related tags">
            <div class="section-head">
              <h3>Related tags</h3>
              <span>{detail.relatedTags.length}</span>
            </div>
            <div class="related-chips">
              {#each detail.relatedTags as related}
                <button
                  class="related-chip"
                  type="button"
                  aria-label={`Show tag: ${related.name}`}
                  onclick={() => selectTag(related.key)}
                >
                  <span class={`kind-badge kind-${related.kind}`}>{related.kind}</span>
                  <span class="related-name">{related.name}</span>
                  <span class="related-count" title={`${related.coCount} shared ${related.coCount === 1 ? "note" : "notes"}`}>{related.coCount}</span>
                </button>
              {/each}
            </div>
          </section>
        {/if}
      </div>
    {/if}
  </section>
</section>

<style>
  .tags-view {
    display: grid;
    grid-template-columns: 320px minmax(0, 1fr);
    height: 100%;
    min-width: 0;
    min-height: 0;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    overflow: hidden;
  }

  .tags-list {
    display: flex;
    height: 100%;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .tags-list-head {
    display: grid;
    gap: 10px;
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 92%, var(--color-app-bg));
  }

  .head-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h1 {
    margin: 0;
    color: var(--color-text-primary);
    font-size: 16px;
    font-weight: 750;
    line-height: 1.2;
  }

  h2 {
    margin: 0;
    color: var(--color-text-primary);
    font-size: 17px;
    font-weight: 750;
    line-height: 1.2;
  }

  h3 {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.07em;
    line-height: 1.2;
    text-transform: uppercase;
  }

  .load-state,
  .detail-head p,
  .note-when,
  .note-preview,
  .tag-meta,
  .tag-empty,
  .empty-state span,
  .detail-empty p {
    color: var(--color-text-muted);
    font-size: 11.5px;
  }

  .load-state,
  .tag-count,
  .tag-meta,
  .section-head span,
  .related-count,
  .note-when {
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  }

  .search-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 31px;
    padding: 0 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-text-muted);
    background: var(--color-surface-input);
  }

  .search-row:focus-within {
    border-color: var(--color-accent-primary);
    color: var(--color-text-primary);
  }

  .search-row input {
    flex: 1;
    min-width: 0;
    border: 0;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    outline: none;
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

  .tags-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 2px 0;
  }

  .tag-row {
    display: grid;
    gap: 3px;
    width: 100%;
    min-height: 48px;
    padding: 8px 14px;
    border: 0;
    border-left: 2px solid transparent;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .tag-row-head {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
  }

  .tag-row + .tag-row {
    border-top: 1px solid var(--color-border-default);
  }

  .tag-row:hover,
  .tag-row.selected {
    background: var(--color-surface-2);
  }

  .tag-row.selected {
    border-left-color: var(--color-accent-primary);
  }

  .tag-row:focus-visible,
  .tag-note:focus-visible,
  .related-chip:focus-visible {
    outline: 1px solid var(--color-accent-primary);
    outline-offset: -2px;
  }

  .tag-name,
  .related-name {
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag-count {
    min-width: 20px;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 12%, var(--color-surface-2));
    font-size: 11px;
    text-align: center;
  }

  .tag-meta {
    padding-left: 2px;
  }

  /* Kind badges — one distinct, theme-derived tint per tag kind via currentColor. */
  .kind-badge {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 7px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, currentColor 32%, transparent);
    background: color-mix(in srgb, currentColor 13%, var(--color-surface-1));
    color: var(--color-text-muted);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: lowercase;
    white-space: nowrap;
  }

  .kind-badge.lg {
    height: 20px;
    padding: 0 9px;
    font-size: 11px;
  }

  .kind-person {
    color: var(--color-accent-primary);
  }

  .kind-project {
    color: var(--color-status-success);
  }

  .kind-topic {
    color: var(--color-accent-hot);
  }

  .kind-urgency {
    color: var(--color-status-error);
  }

  .kind-category {
    color: var(--color-status-warning);
  }

  .kind-custom {
    color: var(--color-text-muted);
  }

  .tag-detail {
    display: flex;
    height: 100%;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--color-app-bg);
    overflow: hidden;
  }

  .detail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 64px;
    padding: 12px 22px;
    border-bottom: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-app-bg) 92%, var(--color-surface-1));
  }

  .detail-head-meta {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }

  .detail-head p {
    margin: 2px 0 0;
  }

  .detail-scroll {
    flex: 1;
    max-width: 760px;
    min-height: 0;
    padding: 18px 24px 60px;
    overflow: auto;
  }

  .detail-section {
    display: grid;
    align-content: start;
    gap: 9px;
    min-width: 0;
  }

  .detail-section + .detail-section {
    margin-top: 26px;
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-head span {
    display: inline-grid;
    min-width: 20px;
    min-height: 20px;
    place-items: center;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 12%, var(--color-surface-2));
    font-size: 11px;
    font-weight: 750;
  }

  .tag-notes {
    display: grid;
    gap: 1px;
  }

  .tag-note {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    border: 0;
    border-radius: 7px;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .tag-note:hover {
    background: var(--color-surface-2);
  }

  .note-body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .note-title {
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note-preview {
    overflow: hidden;
    line-height: 1.4;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    margin-top: 5px;
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

  .related-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .related-chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 8px;
    border: 1px solid var(--color-border-default);
    border-radius: 999px;
    color: inherit;
    background: var(--color-surface-1);
    font: inherit;
    cursor: pointer;
  }

  .related-chip:hover {
    border-color: var(--color-accent-primary);
    background: var(--color-surface-2);
  }

  .related-count {
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .tag-empty {
    padding: 8px 10px;
  }

  .empty-state,
  .detail-empty {
    display: grid;
    place-items: center;
    gap: 6px;
    min-height: 220px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-state {
    padding: 8px 10px;
  }

  .empty-state strong,
  .detail-empty h2 {
    color: var(--color-text-primary);
    font-size: 14px;
  }

  .empty-mark {
    display: grid;
    width: 44px;
    height: 44px;
    place-items: center;
    border-radius: 8px;
    color: var(--color-accent-primary);
    background: var(--color-surface-2);
    font-weight: 900;
  }

  @media (max-width: 920px) {
    .tags-view {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(220px, 42vh) minmax(0, 1fr);
    }

    .tags-list {
      border-right: 0;
      border-bottom: 1px solid var(--color-border-default);
    }
  }
</style>
