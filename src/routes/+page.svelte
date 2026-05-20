<script lang="ts">
  import AppShell from "$lib/components/AppShell.svelte";
  import InboxList from "$lib/components/InboxList.svelte";
  import QuickCapturePanel from "$lib/components/QuickCapturePanel.svelte";
  import { toCssVariables } from "$lib/theme/applyTheme";
  import { darkCompactTheme } from "$lib/theme/themes";

  type InboxItem = {
    id: string;
    source: string;
    title: string;
    body: string;
    capturedAt: string;
    statusLabel: string;
    statusTone?: "neutral" | "accent" | "hot" | "success" | "warning" | "error";
  };

  const themeStyle = Object.entries(toCssVariables(darkCompactTheme))
    .map(([name, value]) => `${name}: ${value}`)
    .join("; ");

  const metrics = [
    { label: "Unsorted", value: "18" },
    { label: "Due today", value: "4" },
    { label: "People", value: "9" },
  ];

  const inboxItems: InboxItem[] = [
    {
      id: "n-1024",
      source: "Maya",
      title: "Kiosk 7 telemetry IDs",
      body: "Bring serial list into the Tuesday sync and flag missing asset tags before the vendor call.",
      capturedAt: "9:42 AM",
      statusLabel: "Today",
      statusTone: "hot",
    },
    {
      id: "n-1023",
      source: "Jordan",
      title: "Pricing export mismatch",
      body: "Finance sees different totals in the CSV than the dashboard. Needs the last filtered query and owner.",
      capturedAt: "9:18 AM",
      statusLabel: "Waiting",
      statusTone: "warning",
    },
    {
      id: "n-1022",
      source: "Rina",
      title: "Visitor badge printer",
      body: "Front desk can print test labels, but badge names are shifted one line on real entries.",
      capturedAt: "Yesterday",
      statusLabel: "Open",
      statusTone: "accent",
    },
    {
      id: "n-1021",
      source: "Alex",
      title: "Ops follow-up from standup",
      body: "Move the vendor renewal note out of chat and attach the current quote before end of week.",
      capturedAt: "Yesterday",
      statusLabel: "Logged",
      statusTone: "success",
    },
  ];

  let quickDraft = $state("Nora: capture the VPN renewal owner, quote link, and Friday deadline.");
  let quickCaptureOpen = $state(true);

  function updateQuickDraft(event: CustomEvent<string>) {
    quickDraft = event.detail;
  }

  function saveQuickDraft(event: CustomEvent<string>) {
    quickDraft = event.detail.trim();
  }

  function closeQuickCapture() {
    quickCaptureOpen = false;
  }
</script>

<svelte:head>
  <title>Work Notes</title>
  <meta name="color-scheme" content="dark" />
</svelte:head>

<AppShell
  title="Work Notes"
  subtitle="Fast capture for coworker drive-bys"
  workspace="Local workspace"
  {metrics}
  {themeStyle}
>
  <InboxList items={inboxItems} selectedId="n-1024" />

  {#snippet quickCapture()}
    {#if quickCaptureOpen}
      <QuickCapturePanel
        value={quickDraft}
        error={null}
        on:input={updateQuickDraft}
        on:save={saveQuickDraft}
        on:close={closeQuickCapture}
      />
    {/if}
  {/snippet}
</AppShell>

<style>
  :global(html) {
    min-height: 100%;
  }

  :global(body) {
    min-height: 100%;
    margin: 0;
    background: var(--color-app-bg, #11151c);
  }

  :global(*) {
    box-sizing: border-box;
  }
</style>
