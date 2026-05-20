<script lang="ts">
  import type { Snippet } from "svelte";

  type ShellMetric = {
    label: string;
    value: string;
  };

  type Props = {
    title: string;
    subtitle: string;
    workspace: string;
    metrics: ShellMetric[];
    themeStyle?: string;
    children?: Snippet;
    quickCapture?: Snippet;
  };

  let { title, subtitle, workspace, metrics, themeStyle = "", children, quickCapture }: Props = $props();
</script>

<div class="app-shell" style={themeStyle}>
  <aside class="sidebar" aria-label="Workspace">
    <div class="brand-block">
      <p>{workspace}</p>
      <h1>{title}</h1>
    </div>

    <nav class="nav-stack" aria-label="Primary">
      <a class="active" href="/">Inbox</a>
      <a href="/">Today</a>
      <a href="/">People</a>
      <a href="/">Archive</a>
    </nav>
  </aside>

  <main class="main-surface">
    <header class="topbar">
      <div>
        <p class="subtitle">{subtitle}</p>
        <h2>Inbox first</h2>
      </div>
      <div class="metrics" aria-label="Workspace metrics">
        {#each metrics as metric}
          <div class="metric">
            <span>{metric.value}</span>
            <p>{metric.label}</p>
          </div>
        {/each}
      </div>
    </header>

    <section class="content-region">
      {@render children?.()}
    </section>
  </main>

  {@render quickCapture?.()}
</div>

<style>
  .app-shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: 184px minmax(0, 1fr);
    color: var(--color-text-primary);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--color-accent-primary) 10%, transparent), transparent 210px),
      var(--color-app-bg);
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-width: 0;
    padding: 16px 12px;
    border-right: 1px solid var(--color-border-default);
    background: color-mix(in srgb, var(--color-surface-1) 76%, var(--color-app-bg));
  }

  .brand-block {
    padding: 9px 8px;
  }

  .brand-block p,
  .subtitle,
  .metric p {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 700;
    line-height: 1.2;
    text-transform: uppercase;
  }

  h1,
  h2 {
    margin: 0;
    color: var(--color-text-primary);
    line-height: 1.1;
  }

  h1 {
    margin-top: 4px;
    font-size: 20px;
  }

  h2 {
    margin-top: 4px;
    font-size: 21px;
  }

  .nav-stack {
    display: grid;
    gap: 4px;
  }

  .nav-stack a {
    display: flex;
    align-items: center;
    min-height: 30px;
    padding: 0 9px;
    border: 1px solid transparent;
    border-radius: 7px;
    color: var(--color-text-muted);
    font-size: 13px;
    font-weight: 750;
    text-decoration: none;
  }

  .nav-stack a.active,
  .nav-stack a:hover {
    border-color: var(--color-border-default);
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .main-surface {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 100vh;
    padding: 14px 16px 88px;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    margin-bottom: 12px;
  }

  .metrics {
    display: flex;
    align-items: stretch;
    gap: 8px;
  }

  .metric {
    min-width: 78px;
    padding: 8px 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    background: var(--color-surface-1);
  }

  .metric span {
    display: block;
    color: var(--color-text-primary);
    font-size: 17px;
    font-weight: 850;
    line-height: 1;
  }

  .metric p {
    margin-top: 5px;
    text-transform: none;
  }

  .content-region {
    min-width: 0;
    min-height: 0;
  }

  @media (max-width: 820px) {
    .app-shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      border-right: 0;
      border-bottom: 1px solid var(--color-border-default);
    }

    .nav-stack {
      display: flex;
      overflow-x: auto;
    }

    .nav-stack a {
      white-space: nowrap;
    }

    .main-surface {
      min-height: auto;
      padding: 12px 12px 360px;
    }
  }

  @media (max-width: 640px) {
    .topbar {
      align-items: flex-start;
      flex-direction: column;
    }

    .metrics {
      width: 100%;
      overflow-x: auto;
    }
  }
</style>
