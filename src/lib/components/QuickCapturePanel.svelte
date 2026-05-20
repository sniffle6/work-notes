<script lang="ts">
  import { onMount, tick } from "svelte";
  import { createEventDispatcher } from "svelte";

  type Props = {
    value?: string;
    saving?: boolean;
    error?: string | null;
  };

  let { value = "", saving = false, error = null }: Props = $props();
  let draft = $state("");
  let noteInput: HTMLTextAreaElement;

  const dispatch = createEventDispatcher<{
    input: string;
    save: string;
    close: void;
  }>();

  $effect(() => {
    draft = value;
  });

  onMount(() => {
    void focusNoteInput();
  });

  export async function focusNoteInput() {
    await tick();
    noteInput?.focus();
    noteInput?.select();
  }

  function handleInput(event: Event) {
    const nextValue = (event.currentTarget as HTMLTextAreaElement).value;
    draft = nextValue;
    dispatch("input", nextValue);
  }

  function dispatchSave() {
    if (saving || !draft.trim()) {
      return;
    }

    dispatch("save", draft);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      dispatchSave();
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("close");
    }
  }
</script>

<section class="quick-capture" aria-label="Quick capture">
  <div class="panel-header">
    <div>
      <p>Quick capture</p>
      <h2>New note</h2>
    </div>
    <button class="icon-button" type="button" aria-label="Close quick capture" onclick={() => dispatch("close")}>
      x
    </button>
  </div>

  <textarea
    bind:this={noteInput}
    aria-label="Note text"
    placeholder="Name, ask, deadline, next step"
    value={draft}
    disabled={saving}
    oninput={handleInput}
    onkeydown={handleKeydown}
  ></textarea>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="panel-actions">
    <span class="save-state">{saving ? "Saving" : "Ready"}</span>
    <button class="save-button" type="button" disabled={saving || !draft.trim()} onclick={dispatchSave}>
      Save
    </button>
  </div>
</section>

<style>
  .quick-capture {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 20;
    display: grid;
    gap: 10px;
    width: min(360px, calc(100vw - 28px));
    padding: 12px;
    border: 1px solid var(--color-border-strong);
    border-radius: 8px;
    background: var(--color-surface-2);
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.38);
  }

  .panel-header,
  .panel-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  p,
  h2 {
    margin: 0;
  }

  .panel-header p {
    color: var(--color-accent-primary);
    font-size: 11px;
    font-weight: 800;
    line-height: 1;
    text-transform: uppercase;
  }

  h2 {
    margin-top: 3px;
    color: var(--color-text-primary);
    font-size: 16px;
    line-height: 1.1;
  }

  .icon-button,
  .save-button {
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    cursor: pointer;
  }

  .icon-button {
    display: grid;
    width: 28px;
    height: 28px;
    padding: 0;
    place-items: center;
    color: var(--color-text-muted);
    font-size: 14px;
    line-height: 1;
  }

  textarea {
    width: 100%;
    min-height: 104px;
    box-sizing: border-box;
    resize: vertical;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    padding: 9px 10px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
    line-height: 1.35;
    outline: none;
  }

  textarea:focus,
  .icon-button:focus-visible,
  .save-button:focus-visible {
    border-color: var(--color-accent-primary);
  }

  textarea::placeholder {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-status-error);
    font-size: 12px;
    line-height: 1.25;
  }

  .save-state {
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .save-button {
    min-height: 30px;
    padding: 0 12px;
    background: var(--color-accent-primary);
    font-size: 12px;
    font-weight: 800;
  }

  .save-button:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }

  @media (max-width: 640px) {
    .quick-capture {
      right: 14px;
      bottom: 14px;
      max-height: min(320px, calc(100vh - 28px));
    }

    textarea {
      min-height: 86px;
    }
  }
</style>
