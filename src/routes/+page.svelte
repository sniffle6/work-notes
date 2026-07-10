<script lang="ts">
  import { onMount, tick } from "svelte";
  import { get } from "svelte/store";
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { hideQuickCapture } from "$lib/api";
  import ActionsList from "$lib/components/ActionsList.svelte";
  import AppShell from "$lib/components/AppShell.svelte";
  import FollowupsView from "$lib/components/FollowupsView.svelte";
  import InboxList from "$lib/components/InboxList.svelte";
  import NoteDetail from "$lib/components/NoteDetail.svelte";
  import PeopleView from "$lib/components/PeopleView.svelte";
  import QuickCapturePanel from "$lib/components/QuickCapturePanel.svelte";
  import ReviewQueue from "$lib/components/ReviewQueue.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";
  import TagsView from "$lib/components/TagsView.svelte";
  import TodayView from "$lib/components/TodayView.svelte";
  import { runUpdateCheck, createTauriUpdaterPort } from "$lib/updater";
  import { NOTE_CAPTURED_EVENT, CHECK_FOR_UPDATES_EVENT, type NoteCapturedPayload } from "$lib/events";
  import { createWorkNotesStore, type InboxViewMode } from "$lib/stores/inbox";
  import type { AppSettings } from "$lib/types";
  import { toCssVariables } from "$lib/theme/applyTheme";
  import { getThemeById } from "$lib/theme/themes";
  import packageMetadata from "../../package.json";

  const workNotes = createWorkNotesStore();
  const appVersion = packageMetadata.version;
  const {
    inbox,
    filteredInbox,
    filters,
    viewMode,
    selectedNote,
    suggestedActions,
    followups,
    settings,
    loadingInbox,
    loadingNote,
    loadingSuggestedActions,
    loadingFollowups,
    savingCapture,
    savingSettings,
    busyActionId,
    error,
    parserNotification,
    navSummary,
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
  // Sidebar chrome reads from the view-independent nav summary so the badges,
  // tag chips, and parser-queue count stay put as the user changes views.
  const metrics = $derived([
    { label: "Inbox", value: String($navSummary.inbox) },
    { label: "Needs review", value: String($navSummary.needsReview) },
    { label: "Follow-ups", value: String($navSummary.followups) },
    { label: "Parse failed", value: String($navSummary.parseFailed) },
  ]);
  const topTags = $derived($navSummary.tags);
  const parserQueueCount = $derived($navSummary.parseQueue);
  const parserCommand = $derived($settings?.codexCommandPath || "codex.cmd");
  const parserActive = $derived(
    parserQueueCount > 0 ||
      $inbox.some((note) => isParserActive(note.parseStatus)) ||
      isParserActive($selectedNote?.parseStatus),
  );

  onMount(() => {
    if (!isTauriRuntime()) {
      void workNotes.loadInbox();
      void workNotes.loadSuggestedActions();
      void workNotes.loadSettings();
      void workNotes.refreshNavSummary();
      return;
    }

    currentWindowLabel = getCurrentWindow().label;
    void workNotes.loadSettings();
    if (currentWindowLabel !== "quick-capture") {
      void workNotes.loadInbox();
      void workNotes.loadSuggestedActions();
      void workNotes.refreshNavSummary();
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
      if (currentWindowLabel !== "quick-capture") {
        return;
      }

      quickCaptureOpen = true;
      await tick();
      await quickCapturePanel?.focusNoteInput();
    }).then(registerUnlisten);

    if (currentWindowLabel !== "quick-capture") {
      const parserRefreshTimer = window.setInterval(() => {
        void workNotes.refreshParserActivity();
      }, 5000);
      registerUnlisten(() => window.clearInterval(parserRefreshTimer));

      void runUpdateCheck(createTauriUpdaterPort(), { silent: true });

      void listen(CHECK_FOR_UPDATES_EVENT, () => {
        void runUpdateCheck(createTauriUpdaterPort(), { silent: false });
      }).then(registerUnlisten);

      void listen<NoteCapturedPayload>(NOTE_CAPTURED_EVENT, (event) => {
        // A note arrived from another window; keep the sidebar counts current
        // regardless of which view we reload below.
        void workNotes.refreshNavSummary();

        if (get(viewMode) === "today") {
          void workNotes.showToday();
          return;
        }

        if (get(viewMode) === "people") {
          void workNotes.showPeople();
          return;
        }

        if (get(viewMode) === "followups") {
          void workNotes.showFollowups();
          return;
        }

        if (get(viewMode) === "tags") {
          void workNotes.showTags();
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

  $effect(() => {
    const notification = $parserNotification;
    if (!notification || typeof window === "undefined") {
      return;
    }

    const timeoutId = window.setTimeout(() => {
      workNotes.clearParserNotification();
    }, 5000);

    return () => window.clearTimeout(timeoutId);
  });

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function isParserActive(status?: string): boolean {
    return status === "queued" || status === "parsing";
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

  function selectTheme(event: CustomEvent<string>) {
    void workNotes.setTheme(event.detail);
  }

  function checkForUpdatesFromSettings() {
    if (!isTauriRuntime()) {
      return;
    }

    void runUpdateCheck(createTauriUpdaterPort(), { silent: false });
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

    if (event.detail === "people") {
      await workNotes.showPeople();
      return;
    }

    if (event.detail === "followups") {
      await workNotes.showFollowups();
      return;
    }

    if (event.detail === "tags") {
      await workNotes.showTags();
      return;
    }

    await workNotes.showInbox();
  }

  async function openNoteFromToday(noteId: string) {
    await workNotes.showInbox();
    await workNotes.selectNote(noteId);
  }

  async function openNoteFromPeople(noteId: string) {
    await workNotes.showInbox();
    await workNotes.selectNote(noteId);
  }

  async function openNoteFromTags(noteId: string) {
    await workNotes.showInbox();
    await workNotes.selectNote(noteId);
  }

  async function openNoteFromFollowup(noteId: string) {
    await workNotes.showInbox();
    await workNotes.selectNote(noteId);
  }

  async function createFollowupFromNote(
    event: CustomEvent<{ text: string; lane: string | null; done: () => void }>,
  ) {
    try {
      await workNotes.createFollowupFromSelectedNote(event.detail.text, event.detail.lane);
      event.detail.done();
    } catch {
      // The store exposes the error; keep the form open so the user can retry.
    }
  }

  async function saveCleanedFromNote(
    event: CustomEvent<{ title: string; summary: string; cleanedText: string; done: () => void }>,
  ) {
    try {
      await workNotes.saveCleanedEdits({
        title: event.detail.title,
        summary: event.detail.summary,
        cleanedText: event.detail.cleanedText,
      });
      event.detail.done();
    } catch {
      // The store exposes the error; keep the editor open so the user can retry.
    }
  }

  async function saveRawFromNote(event: CustomEvent<{ rawText: string; done: () => void }>) {
    try {
      await workNotes.saveRawEdit(event.detail.rawText);
      event.detail.done();
    } catch {
      // The store exposes the error; keep the editor open so the user can retry.
    }
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
  <meta name="color-scheme" content={currentTheme.mode} />
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
    {parserActive}
    {themeId}
    {themeStyle}
    activeView={$viewMode}
    on:newNote={() => void openQuickCapture()}
    on:settings={() => (settingsOpen = true)}
    on:navigate={(event) => void navigatePrimary(event)}
  >
    {#if $parserNotification}
      <div class={`parser-toast ${$parserNotification.tone}`} role="status" aria-live="polite">
        <div>
          <strong>{$parserNotification.title}</strong>
          <span>{$parserNotification.message}</span>
        </div>
        <button type="button" aria-label="Dismiss parser notification" onclick={() => workNotes.clearParserNotification()}>
          x
        </button>
      </div>
    {/if}

    {#if $error}
      <p class="app-error">{$error}</p>
    {/if}

    {#if $viewMode === "followups"}
      <FollowupsView
        followups={$followups}
        loading={$loadingFollowups}
        busyActionId={$busyActionId}
        on:openNote={(event) => void openNoteFromFollowup(event.detail)}
        on:updateState={(event) => void workNotes.updateFollowupState(event.detail.id, event.detail.state)}
        on:updateLane={(event) => void workNotes.updateFollowupLane(event.detail.id, event.detail.lane)}
        on:complete={(event) => void workNotes.completeAction(event.detail)}
        on:reopen={(event) => void workNotes.reopenAction(event.detail)}
      />
    {:else if $viewMode === "today"}
      <TodayView
        actions={$suggestedActions}
        followups={$followups}
        loadingActions={$loadingSuggestedActions || $loadingFollowups}
        busyActionId={$busyActionId}
        on:openNote={(event) => void openNoteFromToday(event.detail)}
        on:accept={(event) => void workNotes.acceptSuggestedAction(event.detail)}
        on:complete={(event) => void workNotes.completeAction(event.detail)}
        on:reopen={(event) => void workNotes.reopenAction(event.detail)}
      />
    {:else if $viewMode === "people"}
      <PeopleView
        notes={$inbox}
        actions={$suggestedActions}
        loadingNotes={$loadingInbox}
        loadingActions={$loadingSuggestedActions}
        on:openNote={(event) => void openNoteFromPeople(event.detail)}
      />
    {:else if $viewMode === "tags"}
      <TagsView
        notes={$inbox}
        loadingNotes={$loadingInbox}
        on:openNote={(event) => void openNoteFromTags(event.detail)}
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
            on:createFollowup={(event) => void createFollowupFromNote(event)}
            on:saveCleaned={(event) => void saveCleanedFromNote(event)}
            on:saveRaw={(event) => void saveRawFromNote(event)}
          />
        </div>
      </div>
    {/if}

    <SettingsView
      settings={$settings}
      saving={$savingSettings}
      open={settingsOpen}
      error={settingsOpen ? $error : null}
      appVersion={appVersion}
      on:save={(event) => void saveSettings(event)}
      on:selectTheme={selectTheme}
      on:checkUpdates={checkForUpdatesFromSettings}
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
    height: 100%;
    min-height: 100%;
    overflow: hidden;
  }

  :global(body) {
    height: 100%;
    min-height: 100%;
    margin: 0;
    background: var(--color-app-bg);
    overflow: hidden;
  }

  :global(*) {
    box-sizing: border-box;
  }

  .workspace-grid {
    display: grid;
    grid-template-columns: 348px minmax(0, 1fr);
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .detail-stack {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    min-width: 0;
    min-height: 0;
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

  .parser-toast {
    position: fixed;
    top: 14px;
    right: 16px;
    z-index: 35;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    width: min(360px, calc(100vw - 32px));
    padding: 10px 11px 10px 12px;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    color: var(--color-text-primary);
    background: var(--color-surface-1);
    box-shadow: 0 18px 36px -18px rgba(0, 0, 0, 0.58);
  }

  .parser-toast.success {
    border-color: color-mix(in srgb, var(--color-status-success) 58%, var(--color-border-default));
    background: color-mix(in srgb, var(--color-status-success) 12%, var(--color-surface-1));
  }

  .parser-toast.error {
    border-color: color-mix(in srgb, var(--color-status-error) 58%, var(--color-border-default));
    background: color-mix(in srgb, var(--color-status-error) 12%, var(--color-surface-1));
  }

  .parser-toast div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .parser-toast strong,
  .parser-toast span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .parser-toast strong {
    font-size: 12.5px;
    font-weight: 850;
  }

  .parser-toast span {
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .parser-toast button {
    display: grid;
    width: 26px;
    height: 26px;
    place-items: center;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--color-text-muted);
    background: transparent;
    font: inherit;
    font-weight: 800;
    cursor: pointer;
  }

  .parser-toast button:hover,
  .parser-toast button:focus-visible {
    border-color: var(--color-border-default);
    color: var(--color-text-primary);
    background: var(--color-surface-2);
    outline: none;
  }

  @media (max-width: 980px) {
    .workspace-grid {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(220px, 42vh) minmax(0, 1fr);
    }
  }
</style>
