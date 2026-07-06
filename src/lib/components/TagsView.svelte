<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { buildTagDetail, buildTags, matchesTagSearch, type TagSummary } from "$lib/tags";
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
</script>

<section class="tags-view" aria-label="Tags">
  <section class="tags-list" aria-label="Tag list">
    <header class="tags-list-head">
      <div class="head-row">
        <h1>Tags</h1>
        <span class="load-state">{loadingNotes ? "Loading" : `${tags.length} tags`}</span>
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
            onclick={() => (selectedKey = tag.key)}
          >
            <span class={`kind-badge kind-${tag.kind}`}>{tag.kind}</span>
            <span class="tag-name">{tag.name}</span>
            <span class="tag-count">{tag.noteCount}</span>
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
          <span class={`kind-badge kind-${detail.tag.kind}`}>{detail.tag.kind}</span>
          <h2>{detail.tag.name}</h2>
        </div>
        <span class="load-state">{loadingNotes ? "Loading" : noteCountLabel(detail.tag.noteCount)}</span>
      </header>

      <div class="detail-scroll">
        {#if detail.notes.length === 0}
          <div class="tag-empty">No notes with this tag yet.</div>
        {:else}
          <div class="tag-notes">
            {#each detail.notes as note}
              <button
                class="tag-note"
                type="button"
                aria-label={`Open note: ${note.title}`}
                onclick={() => dispatch("openNote", note.id)}
              >
                <span class="note-title">{note.title}</span>
              </button>
            {/each}
          </div>
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

  .load-state {
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11.5px;
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
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 40px;
    padding: 8px 14px;
    border: 0;
    border-left: 2px solid transparent;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
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
  .tag-note:focus-visible {
    outline: 1px solid var(--color-accent-primary);
    outline-offset: -2px;
  }

  .tag-name {
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
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
    text-align: center;
  }

  .kind-badge {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 7px;
    border-radius: 999px;
    border: 1px solid var(--color-border-default);
    color: var(--color-text-muted);
    background: var(--color-surface-1);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: lowercase;
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

  .detail-scroll {
    flex: 1;
    max-width: 820px;
    min-height: 0;
    padding: 18px 24px 60px;
    overflow: auto;
  }

  .tag-notes {
    display: grid;
    gap: 1px;
  }

  .tag-note {
    display: grid;
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

  .note-title {
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag-empty,
  .empty-state span,
  .detail-empty p {
    color: var(--color-text-muted);
    font-size: 11.5px;
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
