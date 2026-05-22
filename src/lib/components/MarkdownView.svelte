<script lang="ts">
  import { renderMarkdown } from "$lib/markdown";

  type Props = {
    markdown: string;
  };

  let { markdown }: Props = $props();
  const html = $derived(renderMarkdown(markdown));
</script>

<div class="markdown-view">
  {@html html}
</div>

<style>
  .markdown-view {
    color: var(--color-text-primary);
    font-size: 14px;
    line-height: 1.6;
  }

  .markdown-view :global(h1),
  .markdown-view :global(h2),
  .markdown-view :global(h3),
  .markdown-view :global(h4),
  .markdown-view :global(p),
  .markdown-view :global(ul),
  .markdown-view :global(ol),
  .markdown-view :global(pre) {
    margin: 0 0 12px;
  }

  .markdown-view :global(h1),
  .markdown-view :global(h2),
  .markdown-view :global(h3),
  .markdown-view :global(h4) {
    color: var(--color-text-primary);
    font-weight: 850;
    line-height: 1.25;
  }

  .markdown-view :global(h1) {
    font-size: 19px;
  }

  .markdown-view :global(h2) {
    font-size: 16px;
  }

  .markdown-view :global(h3),
  .markdown-view :global(h4) {
    font-size: 14px;
  }

  .markdown-view :global(ul),
  .markdown-view :global(ol) {
    padding-left: 22px;
  }

  .markdown-view :global(li + li) {
    margin-top: 4px;
  }

  .markdown-view :global(code) {
    border: 1px solid var(--color-border-default);
    border-radius: 5px;
    padding: 1px 5px;
    color: var(--color-accent-hot);
    background: var(--color-surface-input);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12px;
  }

  .markdown-view :global(pre) {
    overflow-x: auto;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    padding: 12px;
    background: var(--color-surface-input);
  }

  .markdown-view :global(pre code) {
    border: 0;
    padding: 0;
    color: var(--color-text-primary);
    background: transparent;
  }

  .markdown-view :global(:last-child) {
    margin-bottom: 0;
  }
</style>
