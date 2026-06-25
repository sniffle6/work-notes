<script lang="ts">
  import Bot from "@lucide/svelte/icons/bot";
  import Check from "@lucide/svelte/icons/check";
  import FolderPlus from "@lucide/svelte/icons/folder-plus";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import Palette from "@lucide/svelte/icons/palette";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import X from "@lucide/svelte/icons/x";
  import { createEventDispatcher } from "svelte";
  import { selectLinkedWorkspaceDirectory } from "$lib/api";
  import type { AppSettings } from "$lib/types";
  import { themes } from "$lib/theme/themes";

  type Props = {
    settings: AppSettings | null;
    saving?: boolean;
    open?: boolean;
    error?: string | null;
  };

  type SettingsSection = "appearance" | "capture" | "parser";
  type ThemeOption = (typeof themes)[number];

  let { settings, saving = false, open = true, error = null }: Props = $props();
  let activeSection = $state<SettingsSection>("appearance");
  let hotkey = $state("");
  let parserTimeoutSeconds = $state(90);
  let parserMaxRetries = $state(3);
  let codexCommandPath = $state("");
  let linkedWorkspacePaths = $state<string[]>([]);
  let folderPickerError = $state<string | null>(null);
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
    linkedWorkspacePaths = normalizeLinkedWorkspacePaths(settings?.linkedWorkspacePaths ?? []);
    folderPickerError = null;
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
      linkedWorkspacePaths,
      selectedTheme,
      launchAtStartup,
      minimizeToTray,
    });
  }

  async function addLinkedWorkspacePath() {
    folderPickerError = null;

    try {
      const selectedPath = await selectLinkedWorkspaceDirectory();
      if (selectedPath) {
        linkedWorkspacePaths = normalizeLinkedWorkspacePaths([...linkedWorkspacePaths, selectedPath]);
      }
    } catch {
      folderPickerError = "Could not open the folder picker.";
    }
  }

  function removeLinkedWorkspacePath(path: string) {
    linkedWorkspacePaths = linkedWorkspacePaths.filter((workspacePath) => workspacePath !== path);
  }

  function normalizeLinkedWorkspacePaths(values: string[]): string[] {
    const seen = new Set<string>();
    const paths: string[] = [];

    for (const value of values) {
      const path = value.trim();
      if (!path || seen.has(path)) {
        continue;
      }

      seen.add(path);
      paths.push(path);
    }

    return paths;
  }

  function themePreviewStyle(theme: ThemeOption): string {
    const tokens = theme.tokens;

    return [
      `--preview-bg: ${tokens["app.bg"]}`,
      `--preview-surface: ${tokens["surface.1"]}`,
      `--preview-surface-2: ${tokens["surface.2"]}`,
      `--preview-border: ${tokens["border.default"]}`,
      `--preview-text: ${tokens["text.primary"]}`,
      `--preview-muted: ${tokens["text.muted"]}`,
      `--preview-accent: ${tokens["accent.primary"]}`,
      `--preview-hot: ${tokens["accent.hot"]}`,
    ].join("; ");
  }
</script>

{#if open}
  <div class="settings-backdrop" aria-hidden="true"></div>
  <div
    class="settings-view"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    aria-describedby="settings-description"
  >
    <header class="settings-header">
      <div>
        <p class="eyebrow">Work Notes</p>
        <h2 id="settings-title">Settings</h2>
        <p id="settings-description">Configure capture, parser behavior, and local workspace context.</p>
      </div>
      <button class="icon-button" type="button" aria-label="Close settings" onclick={() => dispatch("close")}>
        <X size={16} strokeWidth={2.2} />
      </button>
    </header>

    <div class="settings-body">
      <aside class="settings-nav" aria-label="Settings sections">
        <button
          class:active={activeSection === "appearance"}
          type="button"
          aria-current={activeSection === "appearance" ? "page" : undefined}
          onclick={() => (activeSection = "appearance")}
        >
          <Palette size={16} strokeWidth={2.2} />
          <span>Appearance</span>
        </button>
        <button
          class:active={activeSection === "capture"}
          type="button"
          aria-current={activeSection === "capture" ? "page" : undefined}
          onclick={() => (activeSection = "capture")}
        >
          <Keyboard size={16} strokeWidth={2.2} />
          <span>Capture</span>
        </button>
        <button
          class:active={activeSection === "parser"}
          type="button"
          aria-current={activeSection === "parser" ? "page" : undefined}
          onclick={() => (activeSection = "parser")}
        >
          <Bot size={16} strokeWidth={2.2} />
          <span>Parser</span>
        </button>
      </aside>

      <div class="settings-panel">
        <div class="settings-scroll">
          {#if error}
            <p class="settings-error">{error}</p>
          {/if}

          {#if activeSection === "appearance"}
            <section class="settings-section" aria-labelledby="appearance-title">
              <div class="section-head">
                <span class="section-icon" aria-hidden="true"><Palette size={18} strokeWidth={2.2} /></span>
                <div>
                  <h3 id="appearance-title">Appearance</h3>
                  <p>Choose the theme used across the main window and quick capture.</p>
                </div>
              </div>

              <div class="theme-grid" aria-label="Theme">
                {#each themes as theme}
                  <button
                    class:active={selectedTheme === theme.id}
                    class="theme-card"
                    type="button"
                    aria-label={theme.label}
                    aria-pressed={selectedTheme === theme.id}
                    onclick={() => (selectedTheme = theme.id)}
                  >
                    <span class="theme-preview" style={themePreviewStyle(theme)}>
                      <span class="preview-sidebar"></span>
                      <span class="preview-content">
                        <span class="preview-line strong"></span>
                        <span class="preview-line"></span>
                        <span class="preview-line short"></span>
                        <span class="preview-chip"></span>
                      </span>
                    </span>
                    <span class="theme-meta">
                      <strong>{theme.label}</strong>
                      {#if selectedTheme === theme.id}
                        <span class="selected-badge"><Check size={13} strokeWidth={2.5} />Selected</span>
                      {/if}
                    </span>
                  </button>
                {/each}
              </div>
            </section>
          {:else if activeSection === "capture"}
            <section class="settings-section" aria-labelledby="capture-title">
              <div class="section-head">
                <span class="section-icon" aria-hidden="true"><Keyboard size={18} strokeWidth={2.2} /></span>
                <div>
                  <h3 id="capture-title">Capture</h3>
                  <p>Keep quick capture predictable from the keyboard and tray.</p>
                </div>
              </div>

              <label class="field">
                <span>Global hotkey</span>
                <input
                  bind:value={hotkey}
                  aria-label="Global hotkey"
                  disabled={!settings || saving}
                  placeholder="Ctrl+Shift+Space"
                />
                <small>Opens the bottom-corner quick capture window.</small>
              </label>

              <div class="toggle-row">
                <div>
                  <span class="toggle-label">Launch at startup</span>
                  <p>Start Work Notes when Windows starts.</p>
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
                  <p>Keep quick capture and parser work alive after closing the main window.</p>
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
          {:else}
            <section class="settings-section" aria-labelledby="parser-title">
              <div class="section-head">
                <span class="section-icon" aria-hidden="true"><Bot size={18} strokeWidth={2.2} /></span>
                <div>
                  <h3 id="parser-title">Parser</h3>
                  <p>Codex runs locally and can inspect linked folders when a note needs repo context.</p>
                </div>
              </div>

              <label class="field">
                <span>Codex command</span>
                <input
                  bind:value={codexCommandPath}
                  aria-label="Codex command"
                  disabled={!settings || saving}
                  placeholder="codex.cmd"
                />
                <small>Use a full path only if the command is not on PATH.</small>
              </label>

              <div class="field-row">
                <label class="field">
                  <span>Parser timeout</span>
                  <input
                    type="number"
                    min="1"
                    step="1"
                    bind:value={parserTimeoutSeconds}
                    aria-label="Parser timeout"
                    disabled={!settings || saving}
                  />
                  <small>Seconds before a parse job is marked failed.</small>
                </label>

                <label class="field">
                  <span>Max retries</span>
                  <input
                    type="number"
                    min="0"
                    step="1"
                    bind:value={parserMaxRetries}
                    aria-label="Max retries"
                    disabled={!settings || saving}
                  />
                  <small>Retry attempts after parser failures.</small>
                </label>
              </div>

              <div class="workspace-field">
                <div class="workspace-head">
                  <div>
                    <span>Linked repos and directories</span>
                    <small>Optional folders the parser can inspect for task context.</small>
                  </div>
                  <button class="secondary compact" type="button" disabled={!settings || saving} onclick={addLinkedWorkspacePath}>
                    <FolderPlus size={14} strokeWidth={2.2} />
                    Add folder
                  </button>
                </div>

                {#if linkedWorkspacePaths.length}
                  <ul class="workspace-list" aria-label="Linked repos or directories">
                    {#each linkedWorkspacePaths as path}
                      <li>
                        <span title={path}>{path}</span>
                        <button
                          class="remove-path"
                          type="button"
                          disabled={!settings || saving}
                          aria-label={`Remove ${path}`}
                          onclick={() => removeLinkedWorkspacePath(path)}
                        >
                          <Trash2 size={14} strokeWidth={2.2} />
                        </button>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="empty-workspaces">No linked folders yet.</p>
                {/if}

                {#if folderPickerError}
                  <p class="field-error">{folderPickerError}</p>
                {/if}
              </div>
            </section>
          {/if}
        </div>

        <footer class="settings-actions">
          <button class="secondary" type="button" onclick={() => dispatch("close")}>Cancel</button>
          <button class="primary" type="button" disabled={saving || !settings} onclick={handleSubmit}>
            {saving ? "Saving" : "Save settings"}
          </button>
        </footer>
      </div>
    </div>
  </div>
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
    top: 50%;
    left: 50%;
    z-index: 31;
    display: flex;
    width: min(940px, calc(100vw - 32px));
    height: min(720px, calc(100vh - 32px));
    min-height: 420px;
    overflow: hidden;
    flex-direction: column;
    transform: translate(-50%, -50%);
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    box-shadow: 0 24px 48px -16px rgba(0, 0, 0, 0.55);
  }

  :global([data-theme="memphis"]) .settings-backdrop {
    background: color-mix(in srgb, var(--color-app-bg) 66%, transparent);
  }

  :global([data-theme="memphis"]) .settings-view {
    border-width: 2px;
    box-shadow: 6px 6px 0 var(--color-border-default);
  }

  .settings-header {
    display: flex;
    flex: 0 0 auto;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px 16px;
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

  .eyebrow {
    margin-bottom: 5px;
    color: var(--color-accent-primary);
    font-size: 10.5px;
    font-weight: 850;
    letter-spacing: 0.06em;
    line-height: 1;
    text-transform: uppercase;
  }

  h2 {
    color: var(--color-text-primary);
    font-size: 19px;
    font-weight: 850;
    line-height: 1.15;
  }

  .settings-header p,
  .section-head p,
  .toggle-row p,
  .field small,
  .workspace-head small {
    margin-top: 4px;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .icon-button,
  .settings-nav button,
  .theme-card,
  .toggle,
  .remove-path,
  input,
  .primary,
  .secondary {
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
  }

  .icon-button {
    display: grid;
    width: 30px;
    height: 30px;
    flex: 0 0 auto;
    place-items: center;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }

  .icon-button:hover,
  .icon-button:focus-visible {
    border-color: var(--color-accent-primary);
    color: var(--color-text-primary);
    outline: none;
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
    min-height: 0;
    padding: 12px 10px;
    border-right: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 86%, transparent);
  }

  :global([data-theme="memphis"]) .settings-nav {
    border-right-width: 2px;
    background: var(--color-surface-1);
  }

  .settings-nav button {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 0 10px;
    border-color: transparent;
    color: var(--color-text-muted);
    background: transparent;
    font-size: 13px;
    font-weight: 750;
    text-align: left;
    cursor: pointer;
  }

  .settings-nav button:hover,
  .settings-nav button.active {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  :global([data-theme="memphis"]) .settings-nav button.active {
    border-color: var(--color-border-default);
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .settings-panel {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
  }

  .settings-scroll {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 24px 30px 28px;
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

  .settings-section {
    display: grid;
    gap: 18px;
    max-width: 680px;
  }

  .section-head {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    align-items: start;
    gap: 10px;
  }

  .section-icon {
    display: grid;
    width: 34px;
    height: 34px;
    place-items: center;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-accent-primary);
    background: var(--color-surface-1);
  }

  h3 {
    color: var(--color-text-primary);
    font-size: 16px;
    font-weight: 850;
    line-height: 1.2;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .theme-card {
    display: grid;
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

  .theme-preview {
    display: grid;
    grid-template-columns: 52px minmax(0, 1fr);
    height: 108px;
    overflow: hidden;
    border: 1px solid var(--preview-border);
    border-radius: 6px;
    background: var(--preview-bg);
  }

  .preview-sidebar {
    border-right: 1px solid var(--preview-border);
    background: var(--preview-surface);
  }

  .preview-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 8px;
    padding: 12px;
  }

  .preview-line {
    display: block;
    height: 6px;
    border-radius: 999px;
    background: var(--preview-muted);
  }

  .preview-line.strong {
    background: var(--preview-text);
  }

  .preview-line.short {
    width: 62%;
  }

  .preview-chip {
    width: 56px;
    height: 14px;
    margin-top: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--preview-accent) 80%, var(--preview-surface-2));
  }

  .theme-meta,
  .selected-badge {
    display: flex;
    align-items: center;
  }

  .theme-meta {
    justify-content: space-between;
    gap: 10px;
    min-height: 22px;
  }

  .theme-meta strong {
    font-size: 12.5px;
  }

  .selected-badge {
    gap: 4px;
    color: var(--color-accent-primary);
    font-size: 11px;
    font-weight: 800;
  }

  .field-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .field,
  .workspace-field {
    display: grid;
    gap: 7px;
    min-width: 0;
  }

  .field > span,
  .workspace-head span,
  .toggle-label {
    display: block;
    color: var(--color-text-primary);
    font-size: 12px;
    font-weight: 850;
  }

  input {
    width: 100%;
    min-height: 34px;
    padding: 0 10px;
    font-size: 13px;
  }

  input:focus {
    border-color: var(--color-accent-primary);
    outline: none;
  }

  .toggle-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 14px 0;
    border-top: 1px solid var(--color-border-default);
  }

  .toggle-row:first-of-type {
    border-top: 0;
  }

  .toggle {
    position: relative;
    width: 38px;
    height: 22px;
    border: 0;
    border-radius: 999px;
    background: var(--color-surface-2);
    cursor: pointer;
  }

  .toggle span {
    position: absolute;
    top: 3px;
    left: 3px;
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
    transform: translateX(16px);
    background: var(--color-surface-1);
  }

  .workspace-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .secondary.compact {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 30px;
    white-space: nowrap;
  }

  .workspace-list {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .workspace-list li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 30px;
    align-items: center;
    gap: 8px;
    min-height: 38px;
    padding: 4px 6px 4px 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-input);
  }

  .workspace-list li > span {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 12.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-path {
    display: grid;
    width: 30px;
    height: 30px;
    padding: 0;
    place-items: center;
    color: var(--color-text-muted);
    background: var(--color-surface-1);
    cursor: pointer;
  }

  .remove-path:hover,
  .remove-path:focus-visible {
    border-color: var(--color-status-error);
    color: var(--color-status-error);
    outline: none;
  }

  .empty-workspaces,
  .field-error {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.4;
  }

  .field-error {
    color: var(--color-status-error);
  }

  .settings-actions {
    display: flex;
    flex: 0 0 auto;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 94%, var(--color-app-bg));
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
  .secondary:disabled,
  .remove-path:disabled,
  .toggle:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }

  @media (max-width: 760px) {
    .settings-view {
      width: calc(100vw - 20px);
      height: calc(100vh - 20px);
    }

    .settings-header {
      padding: 16px;
    }

    .settings-body {
      grid-template-columns: 1fr;
    }

    .settings-nav {
      flex-direction: row;
      overflow-x: auto;
      border-right: 0;
      border-bottom: 1px solid var(--color-border-default);
    }

    .settings-nav button {
      min-width: max-content;
    }

    .settings-scroll {
      padding: 18px;
    }

    .theme-grid,
    .field-row {
      grid-template-columns: 1fr;
    }

    .workspace-head {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
