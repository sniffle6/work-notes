<script lang="ts">
  import Bot from "@lucide/svelte/icons/bot";
  import Check from "@lucide/svelte/icons/check";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import FolderPlus from "@lucide/svelte/icons/folder-plus";
  import Info from "@lucide/svelte/icons/info";
  import Keyboard from "@lucide/svelte/icons/keyboard";
  import Palette from "@lucide/svelte/icons/palette";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Ticket from "@lucide/svelte/icons/ticket";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import X from "@lucide/svelte/icons/x";
  import { createEventDispatcher } from "svelte";
  import { selectLinkedWorkspaceDirectory } from "$lib/api";
  import type { AppSettings, GeneratedTheme, JiraFilter } from "$lib/types";
  import { allThemes, draftThemeDefinition } from "$lib/theme/custom";
  import type { ThemeDefinition } from "$lib/theme/tokens";

  type Props = {
    settings: AppSettings | null;
    saving?: boolean;
    open?: boolean;
    error?: string | null;
    appVersion?: string;
    /** Theme the theme agent built that is being previewed. */
    themeDraft?: GeneratedTheme | null;
    themeBusy?: boolean;
    themeError?: string | null;
  };

  const THEME_SUGGESTIONS = [
    "Amber phosphor terminal",
    "Paper logbook with blue ink",
    "Slot floor at night: green felt and brass",
  ];

  type SettingsSection = "appearance" | "capture" | "parser" | "jira" | "about";

  let {
    settings,
    saving = false,
    open = true,
    error = null,
    appVersion = "unknown",
    themeDraft = null,
    themeBusy = false,
    themeError = null,
  }: Props = $props();
  let themeRequest = $state("");
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
  let jiraSiteUrl = $state("");
  let jiraProjectKey = $state("");
  let jiraFilters = $state<JiraFilter[]>([]);

  const dispatch = createEventDispatcher<{
    save: AppSettings;
    close: void;
    selectTheme: string;
    checkUpdates: void;
    /** Ask the theme agent for a theme, or for a change to the one being previewed. */
    buildTheme: string;
    keepTheme: void;
    discardTheme: void;
    deleteTheme: string;
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
    jiraSiteUrl = settings?.jiraSiteUrl ?? "";
    jiraProjectKey = settings?.jiraProjectKey ?? "";
    jiraFilters = (settings?.jiraFilters ?? []).map((filter) => ({ ...filter }));
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
      jiraSiteUrl: jiraSiteUrl.trim(),
      jiraProjectKey: jiraProjectKey.trim(),
      // Resolved by the backend from the site URL; passed through untouched.
      jiraCloudId: settings?.jiraCloudId ?? "",
      jiraFilters: jiraFilters
        .map((filter) => ({ name: filter.name.trim(), jql: filter.jql.trim() }))
        .filter((filter) => filter.name || filter.jql),
      // Managed from the Appearance section; passed through untouched.
      customThemes: settings?.customThemes ?? [],
    });
  }

  function addJiraFilter() {
    jiraFilters = [...jiraFilters, { name: "", jql: "" }];
  }

  function removeJiraFilter(index: number) {
    jiraFilters = jiraFilters.filter((_, filterIndex) => filterIndex !== index);
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

  const themeList = $derived(allThemes(settings?.customThemes ?? []));
  const customThemeIds = $derived(new Set((settings?.customThemes ?? []).map((theme) => theme.id)));
  const draftDefinition = $derived(themeDraft ? draftThemeDefinition(themeDraft) : null);

  // A small block painted in the theme's own colours.
  function specimenStyle(theme: ThemeDefinition): string {
    const tokens = theme.tokens;

    return [
      `--specimen-bg: ${tokens["app.bg"]}`,
      `--specimen-surface: ${tokens["surface.2"]}`,
      `--specimen-border: ${tokens["border.default"]}`,
      `--specimen-text: ${tokens["text.primary"]}`,
      `--specimen-muted: ${tokens["text.muted"]}`,
      `--specimen-accent: ${tokens["accent.primary"]}`,
      `--specimen-hot: ${tokens["accent.hot"]}`,
    ].join("; ");
  }

  function applyTheme(themeId: string): void {
    if (themeId === selectedTheme) {
      return;
    }
    selectedTheme = themeId;
    dispatch("selectTheme", themeId);
  }

  function requestTheme(): void {
    const text = themeRequest.trim();
    if (!text || themeBusy) {
      return;
    }
    themeRequest = "";
    dispatch("buildTheme", text);
  }

  function handleThemeKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      requestTheme();
    }
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
        <button
          class:active={activeSection === "jira"}
          type="button"
          aria-current={activeSection === "jira" ? "page" : undefined}
          onclick={() => (activeSection = "jira")}
        >
          <Ticket size={16} strokeWidth={2.2} />
          <span>Jira</span>
        </button>
        <button
          class:active={activeSection === "about"}
          type="button"
          aria-current={activeSection === "about" ? "page" : undefined}
          onclick={() => (activeSection = "about")}
        >
          <Info size={16} strokeWidth={2.2} />
          <span>About</span>
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
                <div>
                  <h3 id="appearance-title">Appearance</h3>
                  <p>Pick a theme, or describe one and have it built. A theme applies to the main window and quick capture as soon as you pick it.</p>
                </div>
              </div>

              {#snippet specimen(theme: ThemeDefinition)}
                <span class="specimen" style={specimenStyle(theme)} aria-hidden="true">
                  <span class="specimen-text">Aa</span>
                  <span class="specimen-row"></span>
                  <span class="specimen-dot hot"></span>
                  <span class="specimen-dot"></span>
                </span>
              {/snippet}

              <div class="theme-list" role="group" aria-label="Theme">
                {#each themeList as theme (theme.id)}
                  <div class="theme-row" class:active={theme.id === selectedTheme}>
                    <button
                      class="theme-pick"
                      type="button"
                      aria-label={theme.label}
                      aria-pressed={theme.id === selectedTheme}
                      onclick={() => applyTheme(theme.id)}
                    >
                      {@render specimen(theme)}
                      <span class="theme-name">{theme.label}</span>
                      <span class="theme-mode">{theme.mode === "dark" ? "Dark" : "Light"}</span>
                      <span class="theme-in-use">
                        {#if theme.id === selectedTheme}
                          <Check size={13} strokeWidth={2.2} aria-hidden="true" />In use
                        {/if}
                      </span>
                    </button>
                    {#if customThemeIds.has(theme.id)}
                      <button
                        class="theme-delete"
                        type="button"
                        aria-label={`Delete ${theme.label}`}
                        title="Delete this theme"
                        onclick={() => dispatch("deleteTheme", theme.id)}
                      >
                        <Trash2 size={14} strokeWidth={1.8} aria-hidden="true" />
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>

              <div class="theme-builder">
                <div>
                  <h4>Build a theme</h4>
                  <p>Describe the look. The theme agent picks the colours and checks that text stays readable. It takes about half a minute.</p>
                </div>

                {#if draftDefinition && themeDraft}
                  <div class="theme-draft" role="group" aria-label={`Previewing ${themeDraft.name}`}>
                    {@render specimen(draftDefinition)}
                    <div class="draft-copy">
                      <strong>{themeDraft.name} <small>previewing</small></strong>
                      <span>{themeDraft.notes}</span>
                      {#if themeDraft.warnings.length > 0}
                        <ul class="draft-warnings">
                          {#each themeDraft.warnings as warning}
                            <li>{warning}</li>
                          {/each}
                        </ul>
                      {/if}
                    </div>
                    <div class="draft-actions">
                      <button class="quiet" type="button" disabled={themeBusy} onclick={() => dispatch("discardTheme")}>
                        Discard
                      </button>
                      <button class="primary" type="button" disabled={themeBusy} onclick={() => dispatch("keepTheme")}>
                        Keep theme
                      </button>
                    </div>
                  </div>
                {/if}

                <div class="theme-request">
                  <textarea
                    bind:value={themeRequest}
                    aria-label="Describe a theme"
                    rows="2"
                    placeholder={themeDraft
                      ? "Ask for a change, for example: warmer accent, lighter panels"
                      : "For example: amber phosphor terminal, low glare"}
                    onkeydown={handleThemeKeydown}
                  ></textarea>
                  <button
                    class="primary"
                    type="button"
                    aria-label="Build theme"
                    disabled={themeBusy || !themeRequest.trim()}
                    onclick={requestTheme}
                  >
                    {themeDraft ? "Change" : "Build"}
                  </button>
                </div>

                {#if themeBusy}
                  <p class="theme-status" role="status">
                    <span class="theme-spinner" aria-hidden="true"></span>
                    Building theme
                  </p>
                {:else if themeError}
                  <p class="theme-error" role="alert">{themeError}</p>
                {:else if !themeDraft}
                  <div class="theme-suggestions">
                    {#each THEME_SUGGESTIONS as suggestion}
                      <button type="button" onclick={() => (themeRequest = suggestion)}>{suggestion}</button>
                    {/each}
                  </div>
                {/if}
              </div>
            </section>
          {:else if activeSection === "capture"}
            <section class="settings-section" aria-labelledby="capture-title">
              <div class="section-head">
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
          {:else if activeSection === "parser"}
            <section class="settings-section" aria-labelledby="parser-title">
              <div class="section-head">
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
          {:else if activeSection === "jira"}
            <section class="settings-section" aria-labelledby="jira-title">
              <div class="section-head">
                <div>
                  <h3 id="jira-title">Jira</h3>
                  <p>
                    Sign-in comes from the JIRA_EMAIL (or BITBUCKET_EMAIL) and JIRA_API_TOKEN environment
                    variables. The token is never stored here.
                  </p>
                </div>
              </div>

              <div class="field-row">
                <label class="field">
                  <span>Jira site URL</span>
                  <input
                    bind:value={jiraSiteUrl}
                    aria-label="Jira site URL"
                    disabled={!settings || saving}
                    placeholder="https://your-team.atlassian.net"
                  />
                  <small>The address you open Jira at in the browser.</small>
                </label>

                <label class="field">
                  <span>Project key</span>
                  <input
                    bind:value={jiraProjectKey}
                    aria-label="Project key"
                    disabled={!settings || saving}
                    placeholder="ABC"
                  />
                  <small>New tickets are created here. It also scopes the built-in filters.</small>
                </label>
              </div>

              <div class="workspace-field">
                <div class="workspace-head">
                  <div>
                    <span>Ticket filters</span>
                    <small>
                      Leave empty for the built-in filters: my open tickets, all open, and unassigned.
                    </small>
                  </div>
                  <button class="secondary compact" type="button" disabled={!settings || saving} onclick={addJiraFilter}>
                    Add filter
                  </button>
                </div>

                {#if jiraFilters.length}
                  <ul class="jira-filter-list" aria-label="Jira ticket filters">
                    {#each jiraFilters as filter, index}
                      <li>
                        <input
                          bind:value={filter.name}
                          aria-label={`Filter ${index + 1} name`}
                          disabled={!settings || saving}
                          placeholder="Name"
                        />
                        <input
                          bind:value={filter.jql}
                          aria-label={`Filter ${index + 1} JQL`}
                          disabled={!settings || saving}
                          placeholder="project = ABC AND statusCategory != Done"
                        />
                        <button
                          class="remove-path"
                          type="button"
                          disabled={!settings || saving}
                          aria-label={`Remove filter ${filter.name || index + 1}`}
                          onclick={() => removeJiraFilter(index)}
                        >
                          <Trash2 size={14} strokeWidth={2.2} />
                        </button>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="empty-workspaces">Using the built-in filters.</p>
                {/if}
              </div>
            </section>
          {:else}
            <section class="settings-section" aria-labelledby="about-title">
              <div class="section-head">
                <div>
                  <h3 id="about-title">About</h3>
                  <p>Installed version and update controls for this app.</p>
                </div>
              </div>

              <div class="about-card">
                <div>
                  <span class="about-label">Installed version</span>
                  <strong>{appVersion}</strong>
                </div>
                <div>
                  <span class="about-label">Update source</span>
                  <p>GitHub releases</p>
                </div>
              </div>

              <div class="about-actions">
                <button class="primary" type="button" onclick={() => dispatch("checkUpdates")}>
                  <RefreshCw size={14} strokeWidth={2.2} />
                  Check for updates
                </button>
                <a
                  class="secondary link-button"
                  href="https://github.com/sniffle6/work-notes/releases/latest"
                  target="_blank"
                  rel="noreferrer"
                >
                  <ExternalLink size={14} strokeWidth={2.2} />
                  View release notes
                </a>
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
    border-bottom: 1px solid var(--rule);
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
    font-weight: var(--weight-strong);
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
    border-right: 1px solid var(--rule);
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
    font-weight: var(--weight-medium);
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
    gap: 10px;
  }

  h3 {
    color: var(--color-text-primary);
    font-size: 16px;
    font-weight: var(--weight-strong);
    line-height: 1.2;
  }

  h4 {
    margin: 0 0 3px;
    font-size: 13px;
    font-weight: var(--weight-strong);
  }

  /* Themes are a ruled list: one line each, painted with their own colours. */
  .theme-list {
    border-top: 1px solid var(--rule);
  }

  .theme-row {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--rule);
  }

  .theme-row.active {
    background: color-mix(in srgb, var(--color-surface-2) 60%, transparent);
  }

  .theme-pick {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: auto minmax(0, 1fr) 48px 72px;
    align-items: center;
    gap: 12px;
    min-width: 0;
    border: 0;
    padding: 8px 10px;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  /* Built-in themes have no delete button; keep their columns aligned with rows that do. */
  .theme-row:not(:has(.theme-delete)) .theme-pick {
    padding-right: 44px;
  }

  .theme-pick:hover {
    background: color-mix(in srgb, var(--color-surface-2) 60%, transparent);
  }

  .theme-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .theme-row.active .theme-name {
    font-weight: var(--weight-medium);
  }

  .theme-mode,
  .theme-in-use {
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .theme-in-use {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    color: var(--color-accent-hot);
  }

  .theme-delete {
    display: grid;
    flex: 0 0 auto;
    width: 28px;
    height: 28px;
    margin-right: 6px;
    place-items: center;
    border: 0;
    border-radius: var(--radius-s);
    padding: 0;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }

  .theme-delete:hover {
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 10%, transparent);
  }

  .specimen {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 5px;
    width: 92px;
    height: 28px;
    border: 1px solid var(--specimen-border);
    border-radius: var(--radius-s);
    padding: 0 7px;
    background: var(--specimen-bg);
  }

  .specimen-text {
    color: var(--specimen-text);
    font-size: 12px;
    font-weight: var(--weight-medium);
    line-height: 1;
  }

  .specimen-row {
    height: 10px;
    border-radius: 2px;
    background: var(--specimen-surface);
  }

  .specimen-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--specimen-accent);
  }

  .specimen-dot.hot {
    background: var(--specimen-hot);
  }

  .theme-builder {
    display: grid;
    gap: 10px;
    margin-top: 8px;
  }

  .theme-builder p {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .theme-request {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 8px;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-m);
    padding: 8px 8px 8px 10px;
    background: var(--color-surface-input);
  }

  .theme-request:focus-within {
    border-color: var(--color-accent-hot);
  }

  .theme-request textarea {
    min-width: 0;
    border: 0;
    padding: 0;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 13px;
    line-height: 1.5;
    outline: none;
    resize: none;
  }

  .theme-draft {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: start;
    gap: 12px;
    border-left: 2px solid var(--color-accent-hot);
    padding: 4px 0 4px 12px;
  }

  .draft-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
    color: var(--color-text-muted);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .draft-copy strong {
    color: var(--color-text-primary);
    font-size: 13px;
    font-weight: var(--weight-medium);
  }

  .draft-copy small {
    margin-left: 6px;
    color: var(--color-accent-hot);
    font-size: 11.5px;
    font-weight: var(--weight-regular);
  }

  .draft-warnings {
    margin: 2px 0 0;
    padding-left: 16px;
    color: var(--color-status-warning);
    font-size: 12px;
  }

  .draft-actions {
    display: flex;
    gap: 6px;
  }

  .quiet {
    min-height: 30px;
    border: 0;
    border-radius: var(--radius-s);
    padding: 0 10px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }

  .quiet:hover:not(:disabled) {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .theme-status {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .theme-spinner {
    width: 11px;
    height: 11px;
    border: 2px solid color-mix(in srgb, var(--color-accent-hot) 24%, transparent);
    border-top-color: var(--color-accent-hot);
    border-radius: 50%;
    animation: theme-spin 0.8s linear infinite;
  }

  @keyframes theme-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .theme-builder .theme-error {
    border-left: 2px solid var(--color-status-error);
    padding: 6px 10px;
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 8%, transparent);
    overflow-wrap: anywhere;
  }

  .theme-suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
  }

  .theme-suggestions button {
    border: 0;
    padding: 2px 0;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }

  .theme-suggestions button::before {
    margin-right: 6px;
    content: "›";
  }

  .theme-suggestions button:hover {
    color: var(--color-accent-hot);
  }

  @media (prefers-reduced-motion: reduce) {
    .theme-spinner {
      animation: none;
    }
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
    font-weight: var(--weight-strong);
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
    border-top: 1px solid var(--rule);
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

  .jira-filter-list {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .jira-filter-list li {
    display: grid;
    grid-template-columns: minmax(120px, 0.35fr) minmax(0, 1fr) 30px;
    align-items: center;
    gap: 8px;
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

  .about-card {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .about-card > div {
    display: grid;
    gap: 6px;
    min-width: 0;
    padding: 14px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-1);
  }

  .about-label {
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: var(--weight-strong);
    line-height: 1;
  }

  .about-card strong,
  .about-card p {
    min-width: 0;
    overflow-wrap: anywhere;
    color: var(--color-text-primary);
    font-size: 14px;
    line-height: 1.3;
  }

  .about-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .about-actions .primary,
  .about-actions .secondary {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    text-decoration: none;
  }

  .settings-actions {
    display: flex;
    flex: 0 0 auto;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--rule);
    background: color-mix(in srgb, var(--color-surface-1) 94%, var(--color-app-bg));
  }

  .primary,
  .secondary {
    min-height: 32px;
    padding: 0 14px;
    font-size: 12px;
    font-weight: var(--weight-strong);
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
      border-bottom: 1px solid var(--rule);
    }

    .settings-nav button {
      min-width: max-content;
    }

    .settings-scroll {
      padding: 18px;
    }

    .field-row,
    .about-card {
      grid-template-columns: 1fr;
    }

    .workspace-head {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
