<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { AppSettings } from "$lib/types";

  type Props = {
    settings: AppSettings | null;
    saving?: boolean;
  };

  let { settings, saving = false }: Props = $props();
  let hotkey = $state("");
  let parserTimeoutSeconds = $state(45);
  let codexCommandPath = $state("");
  let selectedTheme = $state("dark-compact");
  let launchAtStartup = $state(false);
  let minimizeToTray = $state(true);

  const dispatch = createEventDispatcher<{
    save: AppSettings;
  }>();

  $effect(() => {
    hotkey = settings?.hotkey ?? "";
    parserTimeoutSeconds = settings?.parserTimeoutSeconds ?? 45;
    codexCommandPath = settings?.codexCommandPath ?? "";
    selectedTheme = settings?.selectedTheme ?? "dark-compact";
    launchAtStartup = settings?.launchAtStartup ?? false;
    minimizeToTray = settings?.minimizeToTray ?? true;
  });

  function handleSubmit() {
    dispatch("save", {
      hotkey: hotkey.trim(),
      parserTimeoutSeconds: Math.max(1, Number(parserTimeoutSeconds) || 45),
      parserMaxRetries: settings?.parserMaxRetries,
      codexCommandPath: codexCommandPath.trim(),
      selectedTheme,
      launchAtStartup,
      minimizeToTray,
    });
  }
</script>

<section class="settings-view" aria-label="Settings">
  <header class="settings-header">
    <div>
      <p class="eyebrow">Settings</p>
      <h2>Capture and parser</h2>
    </div>
    <button type="button" disabled={saving || !settings} onclick={handleSubmit}>
      {saving ? "Saving" : "Save"}
    </button>
  </header>

  <div class="settings-grid">
    <label>
      <span>Hotkey</span>
      <input bind:value={hotkey} disabled={!settings || saving} placeholder="Ctrl+Shift+Space" />
    </label>

    <label>
      <span>Parser timeout</span>
      <input
        type="number"
        min="1"
        step="1"
        bind:value={parserTimeoutSeconds}
        disabled={!settings || saving}
      />
    </label>

    <label>
      <span>Codex command</span>
      <input bind:value={codexCommandPath} disabled={!settings || saving} placeholder="codex" />
    </label>

    <label>
      <span>Theme</span>
      <select bind:value={selectedTheme} disabled={!settings || saving}>
        <option value="dark-compact">Dark Compact</option>
      </select>
    </label>

    <label class="checkbox-row">
      <input type="checkbox" bind:checked={launchAtStartup} disabled={!settings || saving} />
      <span>Launch at startup</span>
    </label>

    <label class="checkbox-row">
      <input type="checkbox" bind:checked={minimizeToTray} disabled={!settings || saving} />
      <span>Minimize to tray</span>
    </label>

    <div class="parser-health" aria-label="Parser health">
      <span>Parser health</span>
      <strong>{codexCommandPath.trim() ? "Codex command configured" : "Codex command missing"}</strong>
    </div>
  </div>
</section>

<style>
  .settings-view {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .settings-header {
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
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 16px;
    line-height: 1.15;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    padding: 12px;
  }

  label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  label span,
  .parser-health span {
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 800;
    line-height: 1;
    text-transform: uppercase;
  }

  input,
  select,
  button {
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
  }

  input,
  select {
    width: 100%;
    min-height: 32px;
    padding: 0 9px;
    font-size: 13px;
  }

  input[type="checkbox"] {
    width: 16px;
    min-height: 16px;
    padding: 0;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .parser-health {
    display: grid;
    gap: 7px;
    min-width: 0;
    padding: 9px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    background: var(--color-surface-input);
  }

  .parser-health strong {
    color: var(--color-text-primary);
    font-size: 12px;
    line-height: 1.2;
  }

  button {
    min-height: 30px;
    padding: 0 12px;
    border-color: var(--color-accent-primary);
    background: var(--color-accent-primary);
    font-size: 12px;
    font-weight: 800;
    cursor: pointer;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }

  input:focus,
  select:focus,
  button:focus-visible {
    border-color: var(--color-accent-primary);
    outline: none;
  }

  @media (max-width: 640px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
