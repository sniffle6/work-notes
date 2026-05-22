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
  <div class="panel-titlebar">
    <div class="title-left">
      <span class="capture-dot" aria-hidden="true"></span>
      <span>Quick capture</span>
    </div>
    <div class="title-right">
      <span>Tray</span>
      <button class="icon-button" type="button" aria-label="Close quick capture" onclick={() => dispatch("close")}>
        x
      </button>
    </div>
  </div>

  <div class="panel-body">
    <textarea
      bind:this={noteInput}
      aria-label="Note text"
      placeholder="Name, ask, deadline, next step..."
      value={draft}
      disabled={saving}
      oninput={handleInput}
      onkeydown={handleKeydown}
    ></textarea>

    <div class="hint-row">
      <strong>{saving ? "Saving" : "Ready"}</strong>
      <span>Enter saves. Shift+Enter makes a new line.</span>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  <div class="panel-actions">
    <div class="key-hints">
      <span><kbd>Esc</kbd> close</span>
      <span><kbd>Shift</kbd>+<kbd>Enter</kbd> newline</span>
    </div>
    <button class="save-button" type="button" aria-label="Save" disabled={saving || !draft.trim()} onclick={dispatchSave}>
      Save <kbd>Enter</kbd>
    </button>
  </div>
</section>

<style>
  .quick-capture {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 20;
    display: flex;
    width: min(380px, calc(100vw - 28px));
    overflow: hidden;
    flex-direction: column;
    border: 1px solid var(--color-border-strong);
    border-radius: 10px;
    color: var(--color-text-primary);
    background: var(--color-surface-1);
    box-shadow: 0 24px 48px -16px rgba(0, 0, 0, 0.55);
  }

  :global([data-theme="memphis"]) .quick-capture {
    border: 2px solid var(--color-border-default);
    border-radius: 12px;
    box-shadow: 6px 6px 0 var(--color-border-default);
  }

  .panel-titlebar,
  .panel-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .panel-titlebar {
    padding: 8px 10px 8px 12px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .panel-titlebar {
    border-bottom-width: 2px;
    color: var(--color-surface-input);
    background: var(--color-accent-primary);
  }

  .title-left,
  .title-right,
  .key-hints {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .title-left {
    font-size: 12px;
    font-weight: 800;
  }

  .title-right {
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 10.5px;
  }

  :global([data-theme="memphis"]) .title-right {
    color: color-mix(in srgb, var(--color-surface-input) 82%, transparent);
  }

  .capture-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-accent-primary);
  }

  :global([data-theme="memphis"]) .capture-dot {
    background: var(--color-surface-input);
  }

  .icon-button {
    display: grid;
    width: 22px;
    height: 22px;
    place-items: center;
    border: 0;
    border-radius: 5px;
    color: inherit;
    background: transparent;
    font: inherit;
    cursor: pointer;
  }

  .icon-button:hover,
  .icon-button:focus-visible {
    background: color-mix(in srgb, var(--color-surface-input) 24%, transparent);
    outline: none;
  }

  .panel-body {
    display: grid;
    gap: 8px;
    padding: 12px 12px 8px;
  }

  textarea {
    width: 100%;
    min-height: 104px;
    resize: none;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    outline: none;
    padding: 9px 11px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
    line-height: 1.5;
  }

  :global([data-theme="memphis"]) textarea {
    border-width: 2px;
    border-radius: 8px;
  }

  textarea:focus {
    border-color: var(--color-accent-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent-primary) 22%, transparent);
  }

  :global([data-theme="memphis"]) textarea:focus {
    box-shadow: 3px 3px 0 var(--color-border-default);
  }

  textarea::placeholder {
    color: var(--color-text-muted);
  }

  .hint-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 9px;
    border-radius: 6px;
    color: var(--color-text-muted);
    background: color-mix(in srgb, var(--color-accent-primary) 13%, var(--color-surface-1));
    font-size: 11.5px;
  }

  :global([data-theme="memphis"]) .hint-row {
    border: 2px solid var(--color-border-default);
    border-radius: 8px;
    color: var(--color-border-strong);
    background: var(--color-accent-hot);
  }

  .hint-row strong {
    color: var(--color-text-primary);
  }

  .error {
    margin: 0;
    color: var(--color-status-error);
    font-size: 12px;
    line-height: 1.25;
  }

  .panel-actions {
    padding: 9px 12px 11px;
    border-top: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-2) 55%, transparent);
  }

  :global([data-theme="memphis"]) .panel-actions {
    border-top-width: 2px;
    background: var(--color-surface-2);
  }

  .key-hints {
    flex-wrap: wrap;
    color: var(--color-text-muted);
    font-size: 11px;
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

  .save-button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 28px;
    padding: 0 11px;
    border: 0;
    border-radius: 6px;
    color: var(--color-surface-1);
    background: var(--color-accent-primary);
    font: inherit;
    font-size: 12px;
    font-weight: 800;
    cursor: pointer;
  }

  :global([data-theme="memphis"]) .save-button {
    border: 2px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-surface-input);
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .save-button kbd {
    color: currentColor;
    background: color-mix(in srgb, var(--color-surface-input) 20%, transparent);
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

    .key-hints {
      display: none;
    }

    textarea {
      min-height: 86px;
    }
  }
</style>
