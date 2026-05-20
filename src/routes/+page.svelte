<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { hideQuickCapture } from "$lib/api";
  import AppShell from "$lib/components/AppShell.svelte";
  import InboxList from "$lib/components/InboxList.svelte";
  import NoteDetail from "$lib/components/NoteDetail.svelte";
  import QuickCapturePanel from "$lib/components/QuickCapturePanel.svelte";
  import ReviewQueue from "$lib/components/ReviewQueue.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";
  import { createWorkNotesStore } from "$lib/stores/inbox";
  import { toCssVariables } from "$lib/theme/applyTheme";
  import { darkCompactTheme } from "$lib/theme/themes";

  const themeStyle = Object.entries(toCssVariables(darkCompactTheme))
    .map(([name, value]) => `${name}: ${value}`)
    .join("; ");

  const workNotes = createWorkNotesStore();
  const {
    inbox,
    filteredInbox,
    filters,
    selectedNote,
    settings,
    loadingInbox,
    loadingNote,
    savingCapture,
    savingSettings,
    busyActionId,
    error,
  } = workNotes;

  let quickDraft = $state("");
  let quickCaptureOpen = $state(true);
  let quickCaptureError = $state<string | null>(null);
  let currentWindowLabel = $state("browser");
  let quickCapturePanel = $state<{ focusNoteInput: () => Promise<void> } | null>(null);

  const selectedActionItems = $derived($selectedNote?.actionItems ?? []);
  const suggestedActions = $derived(selectedActionItems.filter((action) => action.status === "suggested"));
  const selectedId = $derived($selectedNote?.id);
  const isQuickCaptureWindow = $derived(currentWindowLabel === "quick-capture");
  const metrics = $derived([
    { label: "Inbox", value: String($inbox.length) },
    { label: "Needs review", value: String($inbox.filter((note) => note.reviewStatus === "needs_review").length) },
    { label: "Parse failed", value: String($inbox.filter((note) => note.parseStatus === "failed").length) },
  ]);

  onMount(() => {
    void workNotes.loadInbox();
    void workNotes.loadSettings();

    if (!isTauriRuntime()) {
      return;
    }

    currentWindowLabel = getCurrentWindow().label;
    let unlisten: (() => void) | undefined;
    let disposed = false;

    void listen("quick-capture:focus-note-textarea", async () => {
      quickCaptureOpen = true;
      await tick();
      await quickCapturePanel?.focusNoteInput();
    }).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
      } else {
        unlisten = nextUnlisten;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function updateQuickDraft(event: CustomEvent<string>) {
    quickDraft = event.detail;
  }

  async function saveQuickDraft(event: CustomEvent<string>) {
    quickCaptureError = null;

    try {
      await workNotes.saveCapture(event.detail);
      quickDraft = "";
      await closeQuickCapture();
    } catch (unknownError) {
      quickCaptureError = unknownError instanceof Error ? unknownError.message : "Could not save note.";
    }
  }

  async function closeQuickCapture() {
    quickCaptureOpen = false;
    await hideQuickCapture();
  }
</script>

<svelte:head>
  <title>Work Notes</title>
  <meta name="color-scheme" content="dark" />
</svelte:head>

{#if isQuickCaptureWindow}
  <main class="quick-window" style={themeStyle}>
    <QuickCapturePanel
      bind:this={quickCapturePanel}
      value={quickDraft}
      saving={$savingCapture}
      error={quickCaptureError}
      on:input={updateQuickDraft}
      on:save={saveQuickDraft}
      on:close={() => void closeQuickCapture()}
    />
  </main>
{:else}
  <AppShell
    title="Work Notes"
    subtitle="Fast capture for coworker drive-bys"
    workspace="Local workspace"
    {metrics}
    {themeStyle}
  >
    {#if $error}
      <p class="app-error">{$error}</p>
    {/if}

    <div class="workspace-grid">
      <InboxList
        items={$filteredInbox}
        filters={$filters}
        {selectedId}
        loading={$loadingInbox}
        on:select={(event) => void workNotes.selectNote(event.detail)}
        on:filter={(event) => workNotes.updateFilters(event.detail)}
      />

      <div class="detail-column">
        <NoteDetail
          note={$selectedNote}
          loading={$loadingNote}
          on:retryParse={() => void workNotes.retrySelectedParse()}
        />

        <ReviewQueue
          actions={suggestedActions}
          busyActionId={$busyActionId}
          on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
          on:dismiss={(event) => void workNotes.dismissSuggestedAction(event.detail)}
        />

        <SettingsView
          settings={$settings}
          saving={$savingSettings}
          on:save={(event) => void workNotes.persistSettings(event.detail)}
        />
      </div>
    </div>

    {#snippet quickCapture()}
      {#if quickCaptureOpen}
        <QuickCapturePanel
          bind:this={quickCapturePanel}
          value={quickDraft}
          saving={$savingCapture}
          error={quickCaptureError}
          on:input={updateQuickDraft}
          on:save={saveQuickDraft}
          on:close={() => void closeQuickCapture()}
        />
      {/if}
    {/snippet}
  </AppShell>
{/if}

<style>
  :global(html) {
    min-height: 100%;
  }

  :global(body) {
    min-height: 100%;
    margin: 0;
    background: var(--color-app-bg);
  }

  :global(*) {
    box-sizing: border-box;
  }

  .workspace-grid {
    display: grid;
    grid-template-columns: minmax(260px, 360px) minmax(0, 1fr);
    gap: 12px;
    min-width: 0;
    min-height: 0;
  }

  .quick-window {
    min-height: 100vh;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
  }

  :global(.quick-window .quick-capture) {
    position: static;
    width: 100%;
    min-height: 100vh;
    border: 0;
    border-radius: 0;
    box-shadow: none;
  }

  .detail-column {
    display: grid;
    grid-template-rows: minmax(260px, 1fr) auto auto;
    gap: 12px;
    min-width: 0;
    min-height: 0;
  }

  .app-error {
    margin: 0 0 10px;
    padding: 9px 11px;
    border: 1px solid var(--color-status-error);
    border-radius: 7px;
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 10%, var(--color-surface-1));
    font-size: 12px;
    line-height: 1.3;
  }

  @media (max-width: 980px) {
    .workspace-grid {
      grid-template-columns: 1fr;
    }

    .detail-column {
      grid-template-rows: auto;
    }
  }
</style>
