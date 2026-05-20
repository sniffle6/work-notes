<script lang="ts">
  import StatusBadge from "./StatusBadge.svelte";

  type InboxStatusTone = "neutral" | "accent" | "hot" | "success" | "warning" | "error";

  type InboxItem = {
    id: string;
    source: string;
    title: string;
    body: string;
    capturedAt: string;
    statusLabel: string;
    statusTone?: InboxStatusTone;
  };

  type Props = {
    items: InboxItem[];
    selectedId?: string;
  };

  let { items, selectedId }: Props = $props();
</script>

<section class="inbox-list" aria-label="Inbox notes">
  <div class="list-header">
    <div>
      <p class="eyebrow">Inbox</p>
      <h2>Drive-by captures</h2>
    </div>
    <StatusBadge label={`${items.length} open`} tone="accent" />
  </div>

  <div class="items">
    {#each items as item}
      <article class:selected={item.id === selectedId} class="inbox-item">
        <div class="item-topline">
          <span class="source">{item.source}</span>
          <span class="captured-at">{item.capturedAt}</span>
        </div>
        <div class="item-title-row">
          <h3>{item.title}</h3>
          <StatusBadge label={item.statusLabel} tone={item.statusTone ?? "neutral"} />
        </div>
        <p>{item.body}</p>
      </article>
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
    padding: 12px 14px;
    background: var(--color-surface-1);
    border-left: 3px solid transparent;
  }

  .inbox-item.selected {
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
