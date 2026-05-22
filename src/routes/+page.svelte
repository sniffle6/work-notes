<script lang="ts">
  import { onMount, tick } from "svelte";
  import { get } from "svelte/store";
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { hideQuickCapture } from "$lib/api";
  import ActionsList from "$lib/components/ActionsList.svelte";
  import AppShell from "$lib/components/AppShell.svelte";
  import InboxList from "$lib/components/InboxList.svelte";
  import NoteDetail from "$lib/components/NoteDetail.svelte";
  import QuickCapturePanel from "$lib/components/QuickCapturePanel.svelte";
  import ReviewQueue from "$lib/components/ReviewQueue.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";
  import TodayView from "$lib/components/TodayView.svelte";
  import { NOTE_CAPTURED_EVENT, type NoteCapturedPayload } from "$lib/events";
  import { createWorkNotesStore, type InboxViewMode } from "$lib/stores/inbox";
  import type { AppSettings } from "$lib/types";
  import { toCssVariables } from "$lib/theme/applyTheme";
  import { getThemeById } from "$lib/theme/themes";

  const workNotes = createWorkNotesStore();
  const {
    inbox,
    filteredInbox,
    filters,
    viewMode,
    selectedNote,
    suggestedActions,
    settings,
    loadingInbox,
    loadingNote,
    loadingSuggestedActions,
    savingCapture,
    savingSettings,
    busyActionId,
    error,
  } = workNotes;

  let quickDraft = $state("");
  let quickCaptureOpen = $state(false);
  let quickCaptureError = $state<string | null>(null);
  let currentWindowLabel = $state(initialWindowLabel());
  let quickCapturePanel = $state<{ focusNoteInput: () => Promise<void> } | null>(null);
  let settingsOpen = $state(false);

  const selectedId = $derived($selectedNote?.id);
  const isQuickCaptureWindow = $derived(currentWindowLabel === "quick-capture");
  const currentTheme = $derived(getThemeById($settings?.selectedTheme));
  const themeId = $derived(currentTheme.id);
  const themeStyle = $derived(
    Object.entries(toCssVariables(currentTheme))
      .map(([name, value]) => `${name}: ${value}`)
      .join("; "),
  );
  const metrics = $derived([
    { label: "Inbox", value: String($inbox.length) },
    { label: "Needs review", value: String($suggestedActions.length) },
    { label: "Parse failed", value: String($inbox.filter((note) => note.parseStatus === "failed").length) },
  ]);
  const topTags = $derived(
    Array.from(new Set($inbox.flatMap((note) => note.tags.map((tag) => tag.name)))).sort((left, right) =>
      left.localeCompare(right),
    ),
  );
  const parserQueueCount = $derived($inbox.filter((note) => note.parseStatus === "queued" || note.parseStatus === "parsing").length);
  const parserCommand = $derived($settings?.codexCommandPath || "codex.cmd");

  onMount(() => {
    if (!isTauriRuntime()) {
      void workNotes.loadInbox();
      void workNotes.loadSuggestedActions();
      void workNotes.loadSettings();
      return;
    }

    currentWindowLabel = getCurrentWindow().label;
    void workNotes.loadSettings();
    if (currentWindowLabel !== "quick-capture") {
      void workNotes.loadInbox();
      void workNotes.loadSuggestedActions();
    }
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const registerUnlisten = (nextUnlisten: () => void) => {
      if (disposed) {
        nextUnlisten();
      } else {
        unlisteners.push(nextUnlisten);
      }
    };

    void listen("quick-capture:focus-note-textarea", async () => {
      quickCaptureOpen = true;
      await tick();
      await quickCapturePanel?.focusNoteInput();
    }).then(registerUnlisten);

    if (currentWindowLabel !== "quick-capture") {
      void listen<NoteCapturedPayload>(NOTE_CAPTURED_EVENT, (event) => {
        if (get(viewMode) === "today") {
          void workNotes.showToday();
          return;
        }

        if (event.payload.noteId) {
          void workNotes.showCapturedNote(event.payload.noteId);
          void workNotes.loadSuggestedActions();
        } else {
          void workNotes.loadInbox();
          void workNotes.loadSuggestedActions();
        }
      }).then(registerUnlisten);
    }

    return () => {
      disposed = true;
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  $effect(() => {
    if (typeof document === "undefined") {
      return;
    }

    for (const [name, value] of Object.entries(toCssVariables(currentTheme))) {
      document.body.style.setProperty(name, value);
    }
    document.body.dataset.theme = themeId;
  });

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function initialWindowLabel(): string {
    return isTauriRuntime() ? getCurrentWindow().label : "browser";
  }

  function updateQuickDraft(event: CustomEvent<string>) {
    quickDraft = event.detail;
  }

  async function saveQuickDraft(event: CustomEvent<string>) {
    quickCaptureError = null;

    try {
      const noteId =
        currentWindowLabel === "quick-capture"
          ? await workNotes.captureRawNote(event.detail)
          : await workNotes.saveCapture(event.detail);
      if (noteId && currentWindowLabel === "quick-capture") {
        await emit(NOTE_CAPTURED_EVENT, { noteId } satisfies NoteCapturedPayload).catch(() => undefined);
      }
      quickDraft = "";
      quickCaptureOpen = false;
      await hideQuickCapture().catch((unknownError) => {
        quickCaptureError = unknownError instanceof Error ? unknownError.message : "Could not hide quick capture.";
      });
    } catch (unknownError) {
      quickCaptureError = unknownError instanceof Error ? unknownError.message : "Could not save note.";
    }
  }

  async function closeQuickCapture() {
    quickCaptureOpen = false;
    await hideQuickCapture();
  }

  async function openQuickCapture() {
    quickCaptureOpen = true;
    await tick();
    await quickCapturePanel?.focusNoteInput();
  }

  async function saveSettings(event: CustomEvent<AppSettings>) {
    try {
      await workNotes.persistSettings(event.detail);
      settingsOpen = false;
    } catch {
      settingsOpen = true;
    }
  }

  async function deleteSelectedNote() {
    if (typeof window !== "undefined" && !window.confirm("Archive this note?")) {
      return;
    }

    await workNotes.deleteSelectedNote();
  }

  async function navigatePrimary(event: CustomEvent<InboxViewMode>) {
    if (event.detail === "today") {
      await workNotes.showToday();
      return;
    }

    if (event.detail === "archive") {
      await workNotes.showArchive();
      return;
    }

    if (event.detail === "actions") {
      await workNotes.showActions();
      return;
    }

    await workNotes.showInbox();
  }

  async function openNoteFromToday(noteId: string) {
    await workNotes.showInbox();
    await workNotes.selectNote(noteId);
  }

  async function restoreSelectedNote() {
    await workNotes.restoreSelectedNote();
  }

  async function permanentlyDeleteSelectedNote() {
    const note = $selectedNote;

    if (!note) {
      return;
    }

    const title = note.title || note.summary || note.rawText.slice(0, 40);
    if (
      typeof window !== "undefined" &&
      !window.confirm(`Permanently delete "${title}"? This cannot be undone.`)
    ) {
      return;
    }

    await workNotes.permanentlyDeleteSelectedNote();
  }
</script>

<svelte:head>
  <title>Work Notes</title>
  <meta name="color-scheme" content={themeId === "memphis" ? "light" : "dark"} />
</svelte:head>

{#if isQuickCaptureWindow}
  <main class="quick-window" data-theme={themeId} style={themeStyle}>
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
    tags={topTags}
    {parserCommand}
    {parserQueueCount}
    {themeId}
    {themeStyle}
    activeView={$viewMode}
    on:newNote={() => void openQuickCapture()}
    on:settings={() => (settingsOpen = true)}
    on:navigate={(event) => void navigatePrimary(event)}
  >
    {#if $error}
      <p class="app-error">{$error}</p>
    {/if}

    {#if $viewMode === "today"}
      <TodayView
        notes={$inbox}
        actions={$suggestedActions}
        loadingNotes={$loadingInbox}
        loadingActions={$loadingSuggestedActions}
        busyActionId={$busyActionId}
        on:openNote={(event) => void openNoteFromToday(event.detail)}
        on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
      />
    {:else}
      <div class="workspace-grid">
        {#if $viewMode === "actions"}
          <ActionsList
            actions={$suggestedActions}
            selectedNoteId={$selectedNote?.id ?? null}
            busyActionId={$busyActionId}
            loading={$loadingSuggestedActions}
            on:select={(event) => void workNotes.selectNote(event.detail)}
            on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
            on:dismiss={(event) => void workNotes.dismissSuggestedAction(event.detail)}
          />
        {:else}
          <InboxList
            items={$filteredInbox}
            filters={$filters}
            {selectedId}
            loading={$loadingInbox}
            viewMode={$viewMode}
            on:select={(event) => void workNotes.selectNote(event.detail)}
            on:filter={(event) => void workNotes.updateFilters(event.detail)}
          />
        {/if}

        <div class="detail-stack">
          {#if $viewMode !== "actions" && ($suggestedActions.length > 0 || $loadingSuggestedActions)}
            <ReviewQueue
              actions={$suggestedActions}
              busyActionId={$busyActionId}
              loading={$loadingSuggestedActions}
              on:select={(event) => void workNotes.selectNote(event.detail)}
              on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
              on:dismiss={(event) => void workNotes.dismissSuggestedAction(event.detail)}
            />
          {/if}

          <NoteDetail
            note={$selectedNote}
            loading={$loadingNote}
            busyActionId={$busyActionId}
            on:retryParse={() => void workNotes.retrySelectedParse()}
            on:reparseWithFeedback={(event) => void workNotes.retrySelectedParseWithFeedback(event.detail)}
            on:deleteNote={() => void deleteSelectedNote()}
            on:restoreNote={() => void restoreSelectedNote()}
            on:permanentlyDeleteNote={() => void permanentlyDeleteSelectedNote()}
            on:acceptAction={(event) => void workNotes.acceptSuggestedAction(event.detail)}
            on:dismissAction={(event) => void workNotes.dismissSuggestedAction(event.detail)}
            on:completeAction={(event) => void workNotes.completeAction(event.detail)}
            on:reopenAction={(event) => void workNotes.reopenAction(event.detail)}
          />
        </div>
      </div>
    {/if}

    <SettingsView
      settings={$settings}
      saving={$savingSettings}
      open={settingsOpen}
      error={settingsOpen ? $error : null}
      on:save={(event) => void saveSettings(event)}
      on:close={() => (settingsOpen = false)}
    />

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
    grid-template-columns: 348px minmax(0, 1fr);
    min-width: 0;
    min-height: 100vh;
  }

  .detail-stack {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 100vh;
    overflow: hidden;
  }

  :global(.detail-stack .review-queue) {
    max-height: 240px;
    border-top: 0;
    border-right: 0;
    border-left: 0;
    border-radius: 0;
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

  :global(.quick-window) {
    display: block;
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
  }
</style>
