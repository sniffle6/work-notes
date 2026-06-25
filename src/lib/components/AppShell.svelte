<script lang="ts">
  import ArchiveIcon from "@lucide/svelte/icons/archive";
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import Flag from "@lucide/svelte/icons/flag";
  import Hash from "@lucide/svelte/icons/hash";
  import InboxIcon from "@lucide/svelte/icons/inbox";
  import ListChecks from "@lucide/svelte/icons/list-checks";
  import Users from "@lucide/svelte/icons/users";
  import { createEventDispatcher } from "svelte";
  import type { Snippet } from "svelte";
  import type { InboxViewMode } from "$lib/stores/inbox";

  type ShellMetric = {
    label: string;
    value: string;
  };

  type Props = {
    title: string;
    subtitle: string;
    workspace: string;
    metrics: ShellMetric[];
    activeView?: InboxViewMode;
    tags?: string[];
    parserCommand?: string;
    parserQueueCount?: number;
    themeId?: string;
    themeStyle?: string;
    children?: Snippet;
    quickCapture?: Snippet;
  };

  let {
    title,
    subtitle,
    workspace,
    metrics,
    activeView = "inbox",
    tags = [],
    parserCommand = "codex.cmd",
    parserQueueCount = 0,
    themeId = "dark-compact",
    themeStyle = "",
    children,
    quickCapture,
  }: Props = $props();

  const dispatch = createEventDispatcher<{
    newNote: void;
    settings: void;
    navigate: InboxViewMode;
  }>();

  const visibleTags = $derived(tags.slice(0, 5));
  const hiddenTagCount = $derived(Math.max(0, tags.length - visibleTags.length));
  const inboxMetric = $derived(metrics.find((metric) => metric.label === "Inbox")?.value ?? "0");
  const actionsMetric = $derived(metrics.find((metric) => metric.label === "Needs review")?.value ?? "0");
  const followupsMetric = $derived(metrics.find((metric) => metric.label === "Follow-ups")?.value ?? "0");

  function navigate(view: InboxViewMode) {
    dispatch("navigate", view);
  }
</script>

<div class="app-shell" class:theme-memphis={themeId === "memphis"} data-theme={themeId} style={themeStyle}>
  <aside class="sidebar" aria-label="Workspace">
    <div class="sidebar-scroll">
      <div class="brand-block">
        <div class="brand-mark" aria-hidden="true">WN</div>
        <div class="brand-copy">
          <p>{workspace}</p>
          <h1>{title}</h1>
        </div>
      </div>

      <button class="capture-button" type="button" onclick={() => dispatch("newNote")}>
        <span class="capture-icon" aria-hidden="true">+</span>
        <span>{subtitle ? "New note" : "Capture"}</span>
        <span class="key-row" aria-hidden="true"><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>Space</kbd></span>
      </button>

      <nav class="nav-stack" aria-label="Primary">
        <button
          class:active={activeView === "inbox"}
          type="button"
          aria-current={activeView === "inbox" ? "page" : undefined}
          aria-label="Inbox"
          onclick={() => navigate("inbox")}
        >
          <span class="nav-icon" aria-hidden="true"><InboxIcon size={15} strokeWidth={2.2} /></span>
          <span>Inbox</span>
          <strong aria-hidden="true">{inboxMetric}</strong>
        </button>
        <button
          class:active={activeView === "today"}
          type="button"
          aria-current={activeView === "today" ? "page" : undefined}
          aria-label="Today"
          onclick={() => navigate("today")}
        >
          <span class="nav-icon" aria-hidden="true"><CalendarDays size={15} strokeWidth={2.2} /></span>
          <span>Today</span>
        </button>
        <button
          class:active={activeView === "actions"}
          type="button"
          aria-current={activeView === "actions" ? "page" : undefined}
          aria-label="Actions"
          onclick={() => navigate("actions")}
        >
          <span class="nav-icon" aria-hidden="true"><ListChecks size={15} strokeWidth={2.2} /></span>
          <span>Actions</span>
          <strong aria-hidden="true">{actionsMetric}</strong>
        </button>
        <button
          class:active={activeView === "followups"}
          type="button"
          aria-current={activeView === "followups" ? "page" : undefined}
          aria-label="Follow-ups"
          onclick={() => navigate("followups")}
        >
          <span class="nav-icon" aria-hidden="true"><Flag size={15} strokeWidth={2.2} /></span>
          <span>Follow-ups</span>
          <strong aria-hidden="true">{followupsMetric}</strong>
        </button>
        <a href="/">
          <span class="nav-icon" aria-hidden="true"><Hash size={15} strokeWidth={2.2} /></span>
          <span>Tags</span>
        </a>
        <button
          class:active={activeView === "people"}
          type="button"
          aria-current={activeView === "people" ? "page" : undefined}
          aria-label="People"
          onclick={() => navigate("people")}
        >
          <span class="nav-icon" aria-hidden="true"><Users size={15} strokeWidth={2.2} /></span>
          <span>People</span>
        </button>
        <button
          class:active={activeView === "archive"}
          type="button"
          aria-current={activeView === "archive" ? "page" : undefined}
          aria-label="Archive"
          onclick={() => navigate("archive")}
        >
          <span class="nav-icon" aria-hidden="true"><ArchiveIcon size={15} strokeWidth={2.2} /></span>
          <span>Archive</span>
        </button>
      </nav>

      <div class="tag-block">
        <p>Tags</p>
        <div class="tag-list">
          {#if visibleTags.length === 0}
            <span class="quiet">No tags yet</span>
          {:else}
            {#each visibleTags as tag}
              <span class="tag-chip">{tag}</span>
            {/each}
            {#if hiddenTagCount > 0}
              <span class="quiet">+ {hiddenTagCount} more</span>
            {/if}
          {/if}
        </div>
      </div>
    </div>

    <div class="parser-footer">
      <span class="parser-dot" aria-hidden="true"></span>
      <div>
        <p>Parser ready</p>
        <small>{parserCommand || "codex.cmd"} - {parserQueueCount} in queue</small>
      </div>
      <button class="settings-button" type="button" aria-label="Settings" onclick={() => dispatch("settings")}>*</button>
    </div>
  </aside>

  <main class="content-region">
    {@render children?.()}
  </main>

  {@render quickCapture?.()}
</div>

<style>
  .app-shell {
    --shadow-card: 0 1px 0 color-mix(in srgb, white 5%, transparent) inset, 0 1px 3px rgba(0, 0, 0, 0.28);
    --shadow-pop: 0 24px 48px -16px rgba(0, 0, 0, 0.55);
    display: flex;
    height: 100vh;
    min-height: 0;
    overflow: hidden;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    font-family:
      "Geist", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 13px;
    letter-spacing: 0;
  }

  .theme-memphis {
    --shadow-card: 3px 3px 0 var(--color-border-default);
    --shadow-pop: 6px 6px 0 var(--color-border-default);
    background-color: var(--color-app-bg);
    background-image: radial-gradient(circle at 1px 1px, color-mix(in srgb, var(--color-text-muted) 22%, transparent) 1px, transparent 0);
    background-size: 12px 12px;
    font-family:
      "Space Grotesk", "Geist", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }

  .sidebar {
    position: relative;
    z-index: 2;
    display: flex;
    flex: 0 0 220px;
    flex-direction: column;
    height: 100vh;
    min-height: 0;
    min-width: 0;
    border-right: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 86%, var(--color-app-bg));
    overflow: hidden;
  }

  .theme-memphis .sidebar {
    border-right-width: 2px;
    background: var(--color-surface-1);
  }

  .theme-memphis .sidebar::after {
    position: absolute;
    top: 18px;
    right: -13px;
    width: 0;
    height: 0;
    border-color: transparent transparent transparent var(--color-accent-primary);
    border-style: solid;
    border-width: 8px 0 8px 12px;
    content: "";
  }

  .sidebar-scroll {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    padding: 14px 10px 10px;
    overflow-y: auto;
  }

  .brand-block {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 6px;
  }

  .brand-mark {
    display: grid;
    width: 26px;
    height: 26px;
    place-items: center;
    border-radius: 7px;
    color: var(--color-surface-1);
    background: var(--color-accent-primary);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
    font-weight: 800;
  }

  .theme-memphis .brand-mark {
    border: 2px solid var(--color-border-default);
    box-shadow: 2px 2px 0 var(--color-border-default);
    color: var(--color-surface-input);
  }

  .brand-copy {
    min-width: 0;
    line-height: 1.2;
  }

  .brand-copy p,
  .tag-block p {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 10.5px;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .theme-memphis .brand-copy p {
    color: var(--color-accent-primary);
  }

  h1 {
    margin: 1px 0 0;
    color: var(--color-text-primary);
    font-size: 13.5px;
    font-weight: 750;
    line-height: 1.2;
  }

  .capture-button {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 0 8px 0 9px;
    border: 1px solid var(--color-accent-primary);
    border-radius: 8px;
    color: var(--color-text-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 16%, var(--color-surface-1));
    font: inherit;
    font-weight: 750;
    cursor: pointer;
  }

  .theme-memphis .capture-button {
    border: 2px solid var(--color-border-default);
    border-radius: 10px;
    color: var(--color-surface-input);
    background: var(--color-accent-primary);
    box-shadow: var(--shadow-card);
  }

  .capture-icon {
    display: grid;
    width: 22px;
    height: 22px;
    place-items: center;
    border-radius: 5px;
    color: var(--color-surface-1);
    background: var(--color-accent-primary);
    font-weight: 900;
  }

  .theme-memphis .capture-icon {
    color: var(--color-accent-primary);
    background: var(--color-surface-input);
  }

  .key-row {
    display: inline-flex;
    gap: 2px;
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

  .theme-memphis kbd {
    border-width: 1.5px;
    color: var(--color-text-primary);
    background: var(--color-surface-1);
    box-shadow: 1px 1px 0 var(--color-border-default);
  }

  .capture-button kbd {
    color: currentColor;
    background: color-mix(in srgb, var(--color-surface-input) 20%, transparent);
  }

  .nav-stack {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .nav-stack a,
  .nav-stack button {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    padding: 0 9px;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-weight: 650;
    text-decoration: none;
    text-align: left;
    cursor: pointer;
  }

  .nav-icon {
    display: grid;
    width: 18px;
    height: 18px;
    place-items: center;
    color: var(--color-text-muted);
  }

  .nav-stack a:hover,
  .nav-stack button:hover,
  .nav-stack button.active {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .nav-stack a:hover .nav-icon,
  .nav-stack button:hover .nav-icon,
  .nav-stack button.active .nav-icon {
    color: var(--color-text-primary);
  }

  .theme-memphis .nav-stack button.active {
    border-color: var(--color-border-default);
    background: var(--color-surface-2);
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .nav-stack strong {
    min-width: 22px;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 13%, var(--color-surface-2));
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
    line-height: 1.2;
    text-align: center;
  }

  .theme-memphis .nav-stack strong {
    color: var(--color-surface-input);
    background: var(--color-accent-primary);
  }

  .tag-block {
    display: grid;
    gap: 7px;
    min-width: 0;
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 6px;
  }

  .tag-chip,
  .quiet {
    display: inline-flex;
    align-items: center;
    min-height: 22px;
    padding: 0 8px;
    border: 1px solid var(--color-border-default);
    border-radius: 999px;
    color: var(--color-text-muted);
    background: var(--color-surface-1);
    font-size: 11.5px;
    font-weight: 650;
  }

  .theme-memphis .tag-chip {
    border-width: 1.5px;
    color: var(--color-text-primary);
    background: var(--color-surface-1);
  }

  .quiet {
    border-color: transparent;
    background: transparent;
  }

  .parser-footer {
    display: grid;
    flex: 0 0 auto;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 9px;
    padding: 8px 12px;
    border-top: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-2) 72%, transparent);
  }

  .theme-memphis .parser-footer {
    border-top-width: 2px;
    background: var(--color-surface-2);
  }

  .parser-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-status-success);
  }

  .theme-memphis .parser-dot {
    border: 1.5px solid var(--color-border-default);
  }

  .parser-footer p,
  .parser-footer small {
    display: block;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .parser-footer p {
    font-size: 12px;
    font-weight: 700;
  }

  .parser-footer small {
    color: var(--color-text-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 11px;
  }

  .settings-button {
    display: grid;
    width: 26px;
    height: 26px;
    place-items: center;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }

  .settings-button:hover,
  .settings-button:focus-visible {
    border-color: var(--color-border-default);
    color: var(--color-text-primary);
    background: var(--color-surface-1);
    outline: none;
  }

  .content-region {
    flex: 1;
    height: 100vh;
    min-width: 0;
    min-height: 0;
    background: var(--color-app-bg);
    overflow: hidden;
  }

  .theme-memphis .content-region {
    background-image: radial-gradient(circle at 1px 1px, color-mix(in srgb, var(--color-text-muted) 18%, transparent) 1px, transparent 0);
    background-size: 12px 12px;
  }

  @media (max-width: 860px) {
    .sidebar {
      flex-basis: 190px;
    }

    .key-row {
      display: none;
    }
  }
</style>
