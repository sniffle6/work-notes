<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { AppSettings } from "$lib/types";
  import { themes } from "$lib/theme/themes";

  type Props = {
    settings: AppSettings | null;
    saving?: boolean;
    open?: boolean;
    error?: string | null;
  };

  let { settings, saving = false, open = true, error = null }: Props = $props();
  let hotkey = $state("");
  let parserTimeoutSeconds = $state(90);
  let parserMaxRetries = $state(3);
  let codexCommandPath = $state("");
  let selectedTheme = $state("dark-compact");
  let launchAtStartup = $state(false);
  let minimizeToTray = $state(true);

  const dispatch = createEventDispatcher<{
    save: AppSettings;
    close: void;
  }>();

  $effect(() => {
    hotkey = settings?.hotkey ?? "";
    parserTimeoutSeconds = settings?.parserTimeoutSeconds ?? 90;
    parserMaxRetries = settings?.parserMaxRetries ?? 3;
    codexCommandPath = settings?.codexCommandPath ?? "";
    selectedTheme = settings?.selectedTheme ?? "dark-compact";
    launchAtStartup = settings?.launchAtStartup ?? false;
    minimizeToTray = settings?.minimizeToTray ?? true;
  });

  function handleSubmit() {
    dispatch("save", {
      hotkey: hotkey.trim(),
      parserTimeoutSeconds: Math.max(1, Number(parserTimeoutSeconds) || 90),
      parserMaxRetries: Math.max(0, Number(parserMaxRetries) || 0),
      codexCommandPath: codexCommandPath.trim(),
      selectedTheme,
      launchAtStartup,
      minimizeToTray,
    });
  }
</script>

{#if open}
  <div class="settings-backdrop" aria-hidden="true"></div>
  <section class="settings-view" aria-label="Settings">
    <header class="settings-header">
      <div>
        <h2>Settings</h2>
        <p>Tune capture, parsing, and how Work Notes looks on your machine.</p>
      </div>
      <button class="close-button" type="button" aria-label="Close settings" onclick={() => dispatch("close")}>x</button>
    </header>

    <div class="settings-body">
      <aside class="settings-nav" aria-label="Settings sections">
        <button class="nav-item" type="button">General</button>
        <button class="nav-item active" type="button">Appearance</button>
        <button class="nav-item" type="button">Capture and hotkey</button>
        <button class="nav-item" type="button">Parser</button>
        <button class="nav-item" type="button">Tags and people</button>
        <button class="nav-item" type="button">Data and backup</button>
      </aside>

      <div class="settings-content">
        {#if error}
          <p class="settings-error">{error}</p>
        {/if}

        <section class="settings-section">
          <div class="section-head">
            <h3>Appearance</h3>
            <p>Themes are full token swaps. Every surface, status color, and shadow follows the choice.</p>
          </div>

          <div class="theme-grid" aria-label="Theme">
            {#each themes as theme}
              <button
                class:active={selectedTheme === theme.id}
                class={`theme-card theme-${theme.id}`}
                type="button"
                aria-label={theme.label}
                onclick={() => (selectedTheme = theme.id)}
              >
                <span class="theme-preview">
                  <span class="preview-sidebar"></span>
                  <span class="preview-content">
                    <span></span>
                    <span></span>
                    <span class="short"></span>
                  </span>
                </span>
                <span class="theme-meta">
                  <strong>{theme.label}</strong>
                  <span class="theme-palette" aria-hidden="true">
                    <i></i><i></i><i></i><i></i>
                  </span>
                </span>
              </button>
            {/each}
          </div>

          <div class="toggle-row">
            <div>
              <span class="toggle-label">Match system color scheme</span>
              <p>Keep the chosen theme unless this is enabled later.</p>
            </div>
            <button class="toggle" type="button" aria-label="Match system color scheme" aria-pressed="false"><span></span></button>
          </div>
        </section>

        <section class="settings-section">
          <div class="section-head">
            <h3>Capture and hotkey</h3>
            <p>The hotkey opens a small capture window from the tray.</p>
          </div>

          <div class="field-row">
            <label class="field">
              <span>Global hotkey</span>
              <input bind:value={hotkey} disabled={!settings || saving} placeholder="Ctrl+Shift+Space" />
            </label>

            <label class="field">
              <span>Capture window position</span>
              <select disabled={!settings || saving}>
                <option value="bottom-right">Bottom right</option>
              </select>
            </label>
          </div>

          <div class="toggle-row">
            <div>
              <span class="toggle-label">Launch at startup</span>
              <p>Start Work Notes when Windows boots and keep it in the tray.</p>
            </div>
            <button
              class:on={launchAtStartup}
              class="toggle"
              type="button"
              aria-label="Launch at startup"
              aria-pressed={launchAtStartup}
              disabled={!settings || saving}
              onclick={() => (launchAtStartup = !launchAtStartup)}
            ><span></span></button>
          </div>

          <div class="toggle-row">
            <div>
              <span class="toggle-label">Minimize to tray on close</span>
              <p>Closing the window keeps capture alive in the background.</p>
            </div>
            <button
              class:on={minimizeToTray}
              class="toggle"
              type="button"
              aria-label="Minimize to tray on close"
              aria-pressed={minimizeToTray}
              disabled={!settings || saving}
              onclick={() => (minimizeToTray = !minimizeToTray)}
            ><span></span></button>
          </div>
        </section>

        <section class="settings-section">
          <div class="section-head">
            <h3>Parser</h3>
            <p>Cleaning, summarizing, and tagging run locally through codex exec.</p>
          </div>

          <label class="field span2">
            <span>Codex command</span>
            <input bind:value={codexCommandPath} disabled={!settings || saving} placeholder="codex.cmd" />
          </label>

          <div class="field-row">
            <label class="field">
              <span>Parser timeout</span>
              <input
                type="number"
                min="1"
                step="1"
                bind:value={parserTimeoutSeconds}
                disabled={!settings || saving}
              />
            </label>

            <label class="field">
              <span>Max retries</span>
              <input
                type="number"
                min="0"
                step="1"
                bind:value={parserMaxRetries}
                disabled={!settings || saving}
              />
            </label>
          </div>

          <p class="parser-health">
            <span class="parser-dot" aria-hidden="true"></span>
            {codexCommandPath.trim() ? "Codex command configured" : "Codex command missing"}
          </p>
        </section>

        <div class="settings-actions">
          <button class="secondary" type="button" onclick={() => dispatch("close")}>Cancel</button>
          <button class="primary" type="button" disabled={saving || !settings} onclick={handleSubmit}>
            {saving ? "Saving" : "Save settings"}
          </button>
        </div>
      </div>
    </div>
  </section>
{/if}

<style>
  .settings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    background: color-mix(in srgb, var(--color-app-bg) 74%, rgba(0, 0, 0, 0.68));
    backdrop-filter: blur(2px);
  }

  .settings-view {
    position: fixed;
    inset: 24px;
    z-index: 31;
    display: flex;
    overflow: hidden;
    flex-direction: column;
    border: 1px solid var(--color-border-default);
    border-radius: 14px;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    box-shadow: 0 24px 48px -16px rgba(0, 0, 0, 0.55);
  }

  :global([data-theme="memphis"]) .settings-backdrop {
    background: color-mix(in srgb, var(--color-app-bg) 66%, transparent);
  }

  :global([data-theme="memphis"]) .settings-view {
    border: 2px solid var(--color-border-default);
    box-shadow: 6px 6px 0 var(--color-border-default);
  }

  .settings-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 22px 24px 18px;
    border-bottom: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  :global([data-theme="memphis"]) .settings-header {
    border-bottom-width: 2px;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 19px;
    font-weight: 850;
    line-height: 1.15;
  }

  .settings-header p,
  .section-head p,
  .toggle-row p {
    margin-top: 4px;
    color: var(--color-text-muted);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .close-button,
  .nav-item,
  .theme-card,
  .toggle,
  input,
  select,
  .primary,
  .secondary {
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
  }

  .close-button {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }

  .settings-body {
    display: grid;
    flex: 1;
    grid-template-columns: 190px minmax(0, 1fr);
    min-height: 0;
  }

  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 10px;
    border-right: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 86%, transparent);
  }

  :global([data-theme="memphis"]) .settings-nav {
    border-right-width: 2px;
    background: var(--color-surface-1);
  }

  .nav-item {
    min-height: 30px;
    padding: 0 10px;
    border-color: transparent;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 13px;
    font-weight: 750;
    text-align: left;
    cursor: pointer;
  }

  .nav-item:hover,
  .nav-item.active {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .nav-item.active {
    border-color: var(--color-border-default);
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .settings-content {
    min-width: 0;
    overflow-y: auto;
    padding: 24px 32px 28px;
  }

  .settings-error {
    margin: 0 0 14px;
    padding: 10px 12px;
    border: 1px solid var(--color-status-error);
    border-radius: 7px;
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 11%, var(--color-surface-1));
    font-size: 12px;
    line-height: 1.35;
  }

  .settings-section + .settings-section {
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--color-border-default);
  }

  .section-head {
    margin-bottom: 18px;
  }

  h3 {
    color: var(--color-text-primary);
    font-size: 15px;
    font-weight: 850;
    line-height: 1.2;
  }

  .theme-grid {
    display: grid;
    max-width: 560px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px;
    background: var(--color-surface-1);
    cursor: pointer;
    text-align: left;
  }

  .theme-card.active,
  .theme-card:hover,
  .theme-card:focus-visible {
    border-color: var(--color-accent-primary);
    outline: none;
  }

  :global([data-theme="memphis"]) .theme-card {
    border-width: 2px;
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  :global([data-theme="memphis"]) .theme-card.active {
    box-shadow: 3px 3px 0 var(--color-border-default);
  }

  .theme-preview {
    display: grid;
    grid-template-columns: 50px minmax(0, 1fr);
    height: 96px;
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-app-bg);
  }

  .preview-sidebar {
    border-right: 1px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  .preview-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 8px;
    padding: 10px;
  }

  .preview-content span {
    display: block;
    height: 5px;
    border-radius: 999px;
    background: var(--color-text-muted);
  }

  .preview-content .short {
    width: 58%;
  }

  .theme-card.theme-memphis .theme-preview {
    background: #f6f0d8;
  }

  .theme-card.theme-memphis .preview-sidebar {
    background: #fffdf7;
  }

  .theme-card.theme-memphis .preview-content span {
    background: #121315;
    opacity: 0.45;
  }

  .theme-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .theme-meta strong {
    font-size: 12.5px;
  }

  .theme-palette {
    display: inline-flex;
    gap: 3px;
  }

  .theme-palette i {
    width: 14px;
    height: 14px;
    border: 1px solid var(--color-border-default);
    border-radius: 4px;
    background: var(--color-accent-primary);
  }

  .theme-palette i:nth-child(2) {
    background: var(--color-accent-hot);
  }

  .theme-palette i:nth-child(3) {
    background: var(--color-status-warning);
  }

  .theme-palette i:nth-child(4) {
    background: var(--color-border-default);
  }

  .field-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
    margin-bottom: 16px;
  }

  .field {
    display: grid;
    gap: 6px;
    min-width: 0;
    margin-bottom: 16px;
  }

  .field.span2 {
    grid-column: 1 / -1;
  }

  .field span,
  .toggle-label {
    display: block;
    color: var(--color-text-primary);
    font-size: 12px;
    font-weight: 850;
  }

  input,
  select {
    width: 100%;
    min-height: 32px;
    padding: 0 10px;
    font-size: 13px;
  }

  input:focus,
  select:focus {
    border-color: var(--color-accent-primary);
    outline: none;
  }

  .toggle-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-top: 1px solid var(--color-border-default);
  }

  .toggle-row:first-of-type {
    border-top: 0;
  }

  .toggle {
    position: relative;
    width: 34px;
    height: 20px;
    border: 0;
    border-radius: 999px;
    background: var(--color-surface-2);
    cursor: pointer;
  }

  .toggle span {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: var(--color-text-muted);
    transition: transform 0.15s;
  }

  .toggle.on {
    background: var(--color-accent-primary);
  }

  .toggle.on span {
    transform: translateX(14px);
    background: var(--color-surface-1);
  }

  .parser-health {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--color-status-success);
    font-size: 12px;
    font-weight: 750;
  }

  .parser-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-status-success);
  }

  .settings-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }

  .primary,
  .secondary {
    min-height: 32px;
    padding: 0 14px;
    font-size: 12px;
    font-weight: 850;
    cursor: pointer;
  }

  .primary {
    color: var(--color-surface-1);
    border-color: var(--color-accent-primary);
    background: var(--color-accent-primary);
  }

  .secondary {
    background: var(--color-surface-1);
  }

  .primary:disabled,
  .toggle:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }

  @media (max-width: 760px) {
    .settings-view {
      inset: 12px;
    }

    .settings-body {
      grid-template-columns: 1fr;
    }

    .settings-nav {
      display: none;
    }

    .settings-content {
      padding: 18px;
    }

    .theme-grid,
    .field-row {
      grid-template-columns: 1fr;
    }
  }
</style>
