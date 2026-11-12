<script lang="ts">
  import ArchiveIcon from "@lucide/svelte/icons/archive";
  import CalendarDays from "@lucide/svelte/icons/calendar-days";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import Flag from "@lucide/svelte/icons/flag";
  import Hash from "@lucide/svelte/icons/hash";
  import InboxIcon from "@lucide/svelte/icons/inbox";
  import ListChecks from "@lucide/svelte/icons/list-checks";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import Plus from "@lucide/svelte/icons/plus";
  import Settings from "@lucide/svelte/icons/settings";
  import Ticket from "@lucide/svelte/icons/ticket";
  import Users from "@lucide/svelte/icons/users";
  import { createEventDispatcher } from "svelte";
  import type { Component, Snippet } from "svelte";
  import type { InboxViewMode } from "$lib/stores/inbox";
  import { readFlag, writeFlag } from "$lib/ui-prefs";

  type ShellMetric = {
    label: string;
    value: string;
  };

  type NavItem = {
    view: InboxViewMode;
    label: string;
    icon: Component<{ size?: number; strokeWidth?: number }>;
    /** Label of the metric shown as this item's count. */
    metric?: string;
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
    parserActive?: boolean;
    themeId?: string;
    themeStyle?: string;
    children?: Snippet;
    quickCapture?: Snippet;
  };

  const SIDEBAR_COLLAPSED_KEY = "work-notes.sidebar-collapsed";
  const NAV_ITEMS: NavItem[] = [
    { view: "inbox", label: "Inbox", icon: InboxIcon, metric: "Inbox" },
    { view: "today", label: "Calendar", icon: CalendarDays },
    { view: "actions", label: "Actions", icon: ListChecks, metric: "Needs review" },
    { view: "followups", label: "Follow-ups", icon: Flag, metric: "Follow-ups" },
    { view: "jira", label: "Jira", icon: Ticket },
    { view: "tags", label: "Tags", icon: Hash },
    { view: "people", label: "People", icon: Users },
    { view: "done", label: "Done", icon: CircleCheck },
    { view: "archive", label: "Archive", icon: ArchiveIcon },
  ];

  let {
    title,
    subtitle,
    workspace,
    metrics,
    activeView = "inbox",
    tags = [],
    parserCommand = "codex.cmd",
    parserQueueCount = 0,
    parserActive = false,
    themeId = "dark-compact",
    themeStyle = "",
    children,
    quickCapture,
  }: Props = $props();
  let collapsed = $state(readFlag(SIDEBAR_COLLAPSED_KEY, false));

  const dispatch = createEventDispatcher<{
    newNote: void;
    settings: void;
    navigate: InboxViewMode;
  }>();

  const visibleTags = $derived(tags.slice(0, 5));
  const hiddenTagCount = $derived(Math.max(0, tags.length - visibleTags.length));
  const parserBusy = $derived(parserActive || parserQueueCount > 0);

  function metricValue(label: string | undefined): string | null {
    if (!label) {
      return null;
    }
    return metrics.find((metric) => metric.label === label)?.value ?? "0";
  }

  function toggleSidebar() {
    collapsed = !collapsed;
    writeFlag(SIDEBAR_COLLAPSED_KEY, collapsed);
  }

  function handleShortcut(event: KeyboardEvent) {
    if (event.ctrlKey && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "b") {
      event.preventDefault();
      toggleSidebar();
    }
  }
</script>

<svelte:window onkeydown={handleShortcut} />

<div class="app-shell" class:theme-memphis={themeId === "memphis"} data-theme={themeId} style={themeStyle}>
  <aside class="sidebar" class:collapsed aria-label="Workspace">
    <div class="sidebar-scroll">
      <div class="brand-row">
        <div class="brand-copy">
          <h1>{title}</h1>
          <p>{workspace}</p>
        </div>
        <button
          class="icon-button"
          type="button"
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          aria-expanded={!collapsed}
          title={collapsed ? "Expand sidebar (Ctrl+B)" : "Collapse sidebar (Ctrl+B)"}
          onclick={toggleSidebar}
        >
          <PanelLeft size={15} strokeWidth={1.8} aria-hidden="true" />
        </button>
      </div>

      <button
        class="capture-button"
        type="button"
        title="New note (Ctrl+Shift+Space)"
        aria-label={subtitle ? "New note" : "Capture"}
        onclick={() => dispatch("newNote")}
      >
        <Plus size={15} strokeWidth={2} aria-hidden="true" />
        <span class="capture-label">{subtitle ? "New note" : "Capture"}</span>
        <span class="capture-keys" aria-hidden="true">Ctrl Shift Space</span>
      </button>

      <nav class="nav-stack" aria-label="Primary">
        {#each NAV_ITEMS as item (item.view)}
          {@const count = metricValue(item.metric)}
          {@const Icon = item.icon}
          <button
            class:active={activeView === item.view}
            type="button"
            aria-current={activeView === item.view ? "page" : undefined}
            aria-label={item.label}
            title={collapsed ? item.label : undefined}
            onclick={() => dispatch("navigate", item.view)}
          >
            <span class="nav-icon" aria-hidden="true"><Icon size={15} strokeWidth={1.8} /></span>
            <span class="nav-label">{item.label}</span>
            {#if count !== null}
              <span class="nav-count" class:empty={count === "0"} aria-hidden="true">{count}</span>
            {/if}
          </button>
        {/each}
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

    <div class="parser-footer" class:active={parserBusy} role="status" aria-label="Parser activity" aria-live="polite">
      <span class="parser-dot" class:spinning={parserBusy} aria-hidden="true"></span>
      <div class="parser-copy">
        <p>{parserBusy ? "Parsing notes" : "Parser ready"}</p>
        <small>
          <span>{parserCommand || "codex.cmd"}</span>
          <span>{parserQueueCount} in queue</span>
        </small>
      </div>
      <button class="icon-button" type="button" aria-label="Settings" title="Settings" onclick={() => dispatch("settings")}>
        <Settings size={15} strokeWidth={1.8} aria-hidden="true" />
      </button>
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
    background: var(--color-app-bg-paint, var(--color-app-bg));
    font-family: var(--font-ui);
    font-size: 13px;
    letter-spacing: 0;
  }

  .theme-memphis {
    --shadow-card: 3px 3px 0 var(--color-border-default);
    --shadow-pop: 6px 6px 0 var(--color-border-default);
    background-color: var(--color-app-bg);
    background-image: radial-gradient(circle at 1px 1px, color-mix(in srgb, var(--color-text-muted) 22%, transparent) 1px, transparent 0);
    background-size: 12px 12px;
    font-family: "Space Grotesk", var(--font-ui);
  }

  .sidebar {
    position: relative;
    z-index: 2;
    display: flex;
    flex: 0 0 208px;
    flex-direction: column;
    height: 100vh;
    min-height: 0;
    min-width: 0;
    border-right: 1px solid var(--rule);
    background: color-mix(in srgb, var(--color-surface-1) 55%, var(--color-app-bg));
    overflow: hidden;
    transition: flex-basis 140ms ease;
  }

  .sidebar.collapsed {
    flex-basis: 48px;
  }

  .theme-memphis .sidebar {
    border-right: 2px solid var(--color-border-default);
    background: var(--color-surface-1);
  }

  .sidebar-scroll {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    padding: 10px 8px;
    overflow-x: hidden;
    overflow-y: auto;
  }

  .brand-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 30px;
    padding-left: 8px;
  }

  .brand-copy {
    flex: 1 1 auto;
    min-width: 0;
    line-height: 1.25;
  }

  h1 {
    margin: 0;
    overflow: hidden;
    color: var(--color-text-primary);
    font-family: var(--font-display);
    font-size: 13.5px;
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .brand-copy p,
  .tag-block p {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .icon-button {
    display: grid;
    flex: 0 0 auto;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 0;
    border-radius: var(--radius-s);
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }

  .icon-button:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .capture-button {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 8px;
    border: 1px solid color-mix(in srgb, var(--color-accent-primary) 55%, transparent);
    border-radius: var(--radius-m);
    color: var(--color-text-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 14%, transparent);
    font: inherit;
    font-weight: var(--weight-medium);
    white-space: nowrap;
    cursor: pointer;
  }

  .capture-button:hover {
    background: color-mix(in srgb, var(--color-accent-primary) 24%, transparent);
  }

  .theme-memphis .capture-button {
    border: 2px solid var(--color-border-default);
    color: var(--color-surface-input);
    background: var(--color-accent-primary);
    box-shadow: var(--shadow-card);
  }

  .capture-keys {
    margin-left: auto;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .theme-memphis .capture-keys {
    color: currentColor;
  }

  .nav-stack {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .nav-stack button {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-s);
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }

  .nav-icon {
    display: grid;
    flex: 0 0 auto;
    width: 16px;
    height: 16px;
    place-items: center;
  }

  .nav-label {
    flex: 1 1 auto;
    min-width: 0;
  }

  .nav-stack button:hover {
    color: var(--color-text-primary);
    background: color-mix(in srgb, var(--color-surface-2) 70%, transparent);
  }

  .nav-stack button.active {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
    font-weight: var(--weight-medium);
  }

  .theme-memphis .nav-stack button.active {
    box-shadow: 2px 2px 0 var(--color-border-default);
  }

  .nav-count {
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .nav-count.empty {
    opacity: 0.45;
  }

  .tag-block {
    display: grid;
    gap: 6px;
    min-width: 0;
    padding: 0 8px;
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
  }

  .tag-chip,
  .quiet {
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.6;
  }

  .tag-chip::before {
    content: "#";
    opacity: 0.55;
  }

  .quiet {
    opacity: 0.75;
  }

  .parser-footer {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 9px;
    min-height: 44px;
    padding: 6px 8px 6px 14px;
    border-top: 1px solid var(--rule);
  }

  .parser-footer.active {
    background: color-mix(in srgb, var(--color-accent-hot) 12%, transparent);
  }

  .theme-memphis .parser-footer {
    border-top: 2px solid var(--color-border-default);
    background: var(--color-surface-2);
  }

  .parser-dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-status-success);
  }

  .parser-dot.spinning {
    width: 11px;
    height: 11px;
    border: 2px solid color-mix(in srgb, var(--color-accent-hot) 24%, transparent);
    border-top-color: var(--color-accent-hot);
    background: transparent;
    animation: parser-spin 0.8s linear infinite;
  }

  .parser-copy {
    flex: 1 1 auto;
    min-width: 0;
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
  }

  .parser-footer small {
    display: flex;
    gap: 6px;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 10.5px;
  }

  .parser-footer small span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Collapsed: an icon rail. Names stay available through aria-label and title. */
  .sidebar.collapsed .sidebar-scroll {
    padding: 10px 6px;
  }

  .sidebar.collapsed .brand-row {
    justify-content: center;
    padding-left: 0;
  }

  .sidebar.collapsed .brand-copy,
  .sidebar.collapsed .capture-label,
  .sidebar.collapsed .capture-keys,
  .sidebar.collapsed .nav-label,
  .sidebar.collapsed .tag-block,
  .sidebar.collapsed .parser-copy,
  .sidebar.collapsed .parser-dot:not(.spinning) {
    display: none;
  }

  .sidebar.collapsed .capture-button,
  .sidebar.collapsed .nav-stack button {
    position: relative;
    justify-content: center;
    padding: 0;
  }

  .sidebar.collapsed .nav-count {
    position: absolute;
    top: 3px;
    right: 3px;
    width: 6px;
    height: 6px;
    overflow: hidden;
    border-radius: 50%;
    color: transparent;
    background: var(--color-accent-hot);
  }

  .sidebar.collapsed .nav-count.empty {
    display: none;
  }

  .sidebar.collapsed .parser-footer {
    flex-direction: column;
    justify-content: center;
    padding: 6px;
  }

  @keyframes parser-spin {
    to {
      transform: rotate(360deg);
    }
  }

  button:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: 1px;
  }

  .content-region {
    flex: 1;
    height: 100vh;
    min-width: 0;
    min-height: 0;
    background: var(--color-app-bg-paint, var(--color-app-bg));
    overflow: hidden;
  }

  .theme-memphis .content-region {
    background-image: radial-gradient(circle at 1px 1px, color-mix(in srgb, var(--color-text-muted) 18%, transparent) 1px, transparent 0);
    background-size: 12px 12px;
  }

  @media (max-width: 860px) {
    .sidebar:not(.collapsed) {
      flex-basis: 180px;
    }

    .capture-keys {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sidebar {
      transition: none;
    }

    .parser-dot.spinning {
      animation: none;
    }
  }
</style>
