<script lang="ts">
  import { createEventDispatcher, untrack } from "svelte";

  type Props = {
    title: string;
    summary: string;
    cleanedText: string;
  };

  let { title, summary, cleanedText }: Props = $props();

  let titleValue = $state(untrack(() => title));
  let summaryValue = $state(untrack(() => summary));
  let bodyValue = $state(untrack(() => cleanedText));

  const dispatch = createEventDispatcher<{
    save: { title: string; summary: string; cleanedText: string };
    cancel: void;
  }>();

  function save() {
    dispatch("save", {
      title: titleValue,
      summary: summaryValue,
      cleanedText: bodyValue,
    });
  }
</script>

<div class="cleaned-editor">
  <label>
    <span>Title</span>
    <input aria-label="Edit title" bind:value={titleValue} />
  </label>
  <label>
    <span>Summary</span>
    <textarea aria-label="Edit summary" bind:value={summaryValue} rows="2"></textarea>
  </label>
  <label>
    <span>Cleaned note</span>
    <textarea aria-label="Edit cleaned note" bind:value={bodyValue} rows="12"></textarea>
  </label>
  <footer>
    <button class="secondary-action" type="button" onclick={() => dispatch("cancel")}>Cancel</button>
    <button class="primary-action" type="button" onclick={save}>Save</button>
  </footer>
</div>

<style>
  .cleaned-editor {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .cleaned-editor label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .cleaned-editor label span {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .cleaned-editor input,
  .cleaned-editor textarea {
    background: var(--surface-input);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    color: var(--text-primary);
    padding: 0.5rem;
    font: inherit;
  }

  .cleaned-editor textarea {
    resize: vertical;
  }

  .cleaned-editor footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
