<script lang="ts">
  import Bot from "@lucide/svelte/icons/bot";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import Plus from "@lucide/svelte/icons/plus";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";
  import { createEventDispatcher } from "svelte";
  import { filterIssues, groupIssuesByStatus, priorityTone } from "$lib/jira-list";
  import type { JiraStore } from "$lib/stores/jira";
  import { isTypingTarget, readFlag, writeFlag } from "$lib/ui-prefs";

  import JiraChat from "./JiraChat.svelte";
  import JiraIssueDetail from "./JiraIssueDetail.svelte";
  import JiraIssueForm from "./JiraIssueForm.svelte";

  type Props = {
    jira: JiraStore;
  };

  // The view is rebuilt on every visit, so panel choices live in the browser.
  const AGENT_OPEN_KEY = "work-notes.jira.agent-open";
  const LIST_HIDDEN_KEY = "work-notes.jira.list-hidden";
  const TOGGLE_LIST_KEY = "[";
  const TOGGLE_AGENT_KEY = "]";

  let { jira }: Props = $props();
  let agentOpen = $state(readFlag(AGENT_OPEN_KEY, false));
  let listHidden = $state(readFlag(LIST_HIDDEN_KEY, false));
  let searchQuery = $state("");

  // The store instance is fixed for the lifetime of the view.
  // svelte-ignore state_referenced_locally
  const {
    status,
    activeFilter,
    agentFilter,
    issues,
    selectedIssue,
    issueTypes,
    draft,
    draftAgentOpen,
    loadingIssues,
    loadingIssue,
    savingIssue,
    error,
    attachmentDownload,
    attachmentThumbnails,
    attachmentViewer,
    openingAttachmentId,
    downloadingAttachments,
    threads,
    activeThreadId,
    messages,
    agentBusy,
    busyProposalId,
    chatError,
  } = jira;

  const dispatch = createEventDispatcher<{
    openSettings: void;
    /** A ticket was created in Jira; `noteId` is the note it was drafted from. */
    ticketCreated: { issueKey: string; noteId: string | null };
  }>();

  const visibleIssues = $derived(filterIssues($issues, searchQuery));
  const groups = $derived(groupIssuesByStatus(visibleIssues));
  const countLabel = $derived.by(() => {
    const total = `${$issues.length} ${$issues.length === 1 ? "ticket" : "tickets"}`;
    const project = $status?.projectKey ? ` · ${$status.projectKey}` : "";
    return searchQuery.trim() ? `${visibleIssues.length} of ${total}${project}` : `${total}${project}`;
  });

  // A draft started from a note brings the agent along, whatever the saved choice.
  const agentVisible = $derived(agentOpen || ($draftAgentOpen && $draft !== null));

  function toggleAgent() {
    agentOpen = !agentVisible;
    writeFlag(AGENT_OPEN_KEY, agentOpen);
    if (!agentOpen) {
      jira.closeDraftAgent();
    }
  }

  function toggleList() {
    listHidden = !listHidden;
    writeFlag(LIST_HIDDEN_KEY, listHidden);
  }

  function handleShortcut(event: KeyboardEvent) {
    if (event.ctrlKey || event.altKey || event.metaKey || isTypingTarget(event.target)) {
      return;
    }
    if (!$status?.configured) {
      return;
    }
    if (event.key === TOGGLE_LIST_KEY) {
      event.preventDefault();
      toggleList();
    } else if (event.key === TOGGLE_AGENT_KEY) {
      event.preventDefault();
      toggleAgent();
    }
  }

  function groupLabel(statusName: string, count: number): string {
    return `${statusName}, ${count} ${count === 1 ? "ticket" : "tickets"}`;
  }
</script>

<svelte:window onkeydown={handleShortcut} />

<section class="jira-view" aria-label="Jira">
  <header class="jira-head">
    {#if $status?.configured}
      <button
        class="icon-button"
        class:active={!listHidden}
        type="button"
        aria-label={listHidden ? "Show ticket list" : "Hide ticket list"}
        title={listHidden ? "Show ticket list ( [ )" : "Hide ticket list ( [ )"}
        onclick={toggleList}
      >
        <PanelLeft size={15} strokeWidth={1.8} aria-hidden="true" />
      </button>
    {/if}

    <div class="head-title">
      <h1>Jira</h1>
      <p>
        {#if !$status}
          Loading
        {:else if !$status.configured}
          Not connected
        {:else if $loadingIssues}
          Loading tickets
        {:else}
          {countLabel}
        {/if}
      </p>
    </div>

    {#if $status?.configured}
      <div class="filter-row" role="group" aria-label="Ticket filters">
        {#each $status.filters as filter (filter.name)}
          <button
            class="filter-tab"
            class:active={$activeFilter?.name === filter.name && $activeFilter !== $agentFilter}
            type="button"
            aria-pressed={$activeFilter?.name === filter.name && $activeFilter !== $agentFilter}
            title={filter.jql}
            onclick={() => void jira.selectFilter(filter)}
          >
            {filter.name}
          </button>
        {/each}
        {#if $agentFilter}
          {@const agentSearch = $agentFilter}
          <span class="agent-tab" class:active={$activeFilter === agentSearch}>
            <button
              class="filter-tab"
              class:active={$activeFilter === agentSearch}
              type="button"
              aria-pressed={$activeFilter === agentSearch}
              title={`From the agent: ${agentSearch.jql}`}
              onclick={() => void jira.selectFilter(agentSearch)}
            >
              <Bot size={13} strokeWidth={1.8} aria-hidden="true" />
              {agentSearch.name}
            </button>
            <button
              class="tab-remove"
              type="button"
              aria-label={`Remove ${agentSearch.name}`}
              title="Remove this search"
              onclick={() => void jira.clearAgentFilter()}
            >
              <X size={12} strokeWidth={2} aria-hidden="true" />
            </button>
          </span>
        {/if}
      </div>

      <div class="head-actions">
        <button class="quiet-button" type="button" disabled={$loadingIssues} onclick={() => void jira.refreshIssues()}>
          <RefreshCw size={14} strokeWidth={1.8} aria-hidden="true" />
          Refresh
        </button>
        <button class="quiet-button" type="button" onclick={() => void jira.startDraft()}>
          <Plus size={14} strokeWidth={1.8} aria-hidden="true" />
          New ticket
        </button>
        <button
          class="quiet-button"
          class:active={agentVisible}
          type="button"
          aria-pressed={agentVisible}
          title={agentVisible ? "Hide the agent ( ] )" : "Ask the agent ( ] )"}
          onclick={toggleAgent}
        >
          <Bot size={14} strokeWidth={1.8} aria-hidden="true" />
          Agent
        </button>
      </div>
    {/if}
  </header>

  {#if $error}
    <p class="jira-error" role="alert">{$error}</p>
  {/if}

  {#if $status && !$status.configured}
    <div class="setup-card">
      <h2>Connect Work Notes to Jira</h2>
      <ol>
        {#each $status.missing as step}
          <li>{step}</li>
        {/each}
      </ol>
      <p>
        The API token stays in your environment. It is never saved by Work Notes or shown to the agent.
      </p>
      <button class="primary-button" type="button" onclick={() => dispatch("openSettings")}>Open settings</button>
    </div>
  {:else if $status}
    <div class="jira-body" class:with-agent={agentVisible} class:without-list={listHidden}>
      {#if !listHidden}
        <div class="list-pane">
          <label class="list-search">
            <Search size={14} strokeWidth={1.8} aria-hidden="true" />
            <input
              type="search"
              bind:value={searchQuery}
              aria-label="Search tickets"
              placeholder="Search tickets"
              spellcheck="false"
            />
          </label>

          <div class="issue-list" aria-label="Tickets">
            {#if $issues.length === 0}
              <p class="list-empty">{$loadingIssues ? "Loading tickets" : "No tickets match this filter"}</p>
            {:else if visibleIssues.length === 0}
              <p class="list-empty">No tickets match the search</p>
            {:else}
              {#each groups as group (group.status)}
                <div class="issue-group" role="group" aria-label={groupLabel(group.status, group.issues.length)}>
                  <div class="group-head" aria-hidden="true">
                    <span class={`group-dot ${group.statusCategory}`}></span>
                    <span class="group-name">{group.status}</span>
                    <span class="group-count">{group.issues.length}</span>
                  </div>
                  {#each group.issues as issue (issue.key)}
                    <button
                      class="issue-row"
                      class:selected={$selectedIssue?.key === issue.key && !$draft}
                      type="button"
                      aria-label={`${issue.key}: ${issue.summary}`}
                      aria-current={$selectedIssue?.key === issue.key && !$draft ? "true" : undefined}
                      onclick={() => {
                        jira.cancelDraft();
                        void jira.selectIssue(issue.key);
                      }}
                    >
                      <span class="row-key">{issue.key}</span>
                      <span class="row-summary">{issue.summary}</span>
                      <span class="row-meta">
                        <span>{issue.issueType}</span>
                        <span class:unassigned={!issue.assignee}>{issue.assignee ?? "Unassigned"}</span>
                        {#if issue.priority}
                          <span class={`row-priority ${priorityTone(issue.priority)}`}>{issue.priority}</span>
                        {/if}
                      </span>
                    </button>
                  {/each}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}

      <div class="issue-pane">
        {#if $draft}
          <JiraIssueForm
            draft={$draft}
            issueTypes={$issueTypes}
            projectKey={$status.projectKey}
            saving={$savingIssue}
            agentBusy={$agentBusy}
            on:edit={(event) => jira.editDraft(event.detail)}
            on:submit={async (event) => {
              const issueKey = await jira.submitDraft(event.detail);
              if (issueKey) {
                dispatch("ticketCreated", { issueKey, noteId: event.detail.noteId ?? null });
              }
            }}
            on:cancel={() => jira.cancelDraft()}
          />
        {:else}
          <JiraIssueDetail
            issue={$selectedIssue}
            loading={$loadingIssue}
            saving={$savingIssue}
            attachmentDownload={$attachmentDownload}
            downloadingAttachments={$downloadingAttachments}
            attachmentThumbnails={$attachmentThumbnails}
            attachmentViewer={$attachmentViewer}
            openingAttachmentId={$openingAttachmentId}
            on:openAttachment={(event) => void jira.openAttachment(event.detail)}
            on:closeAttachmentViewer={() => jira.closeAttachmentViewer()}
            preparePrompt={(issue) => jira.ticketPrompt(issue)}
            on:downloadAttachments={(event) => {
              if ($selectedIssue) {
                void jira.downloadAttachments($selectedIssue.key, event.detail);
              }
            }}
            on:revealAttachment={(event) => void jira.revealAttachment(event.detail)}
            on:transition={(event) => void jira.transitionSelected(event.detail)}
            on:assignToMe={() => void jira.assignSelectedToMe()}
            on:comment={async (event) => {
              if (await jira.commentOnSelected(event.detail.text)) {
                event.detail.done();
              }
            }}
            on:saveEdit={async (event) => {
              if (await jira.saveSelectedEdit(event.detail.edit)) {
                event.detail.done();
              }
            }}
          />
        {/if}
      </div>

      {#if agentVisible}
        <JiraChat
          threads={$threads}
          activeThreadId={$activeThreadId}
          messages={$messages}
          busy={$agentBusy}
          busyProposalId={$busyProposalId}
          error={$chatError}
          focusIssueKey={$draft ? null : ($selectedIssue?.key ?? null)}
          on:send={(event) => void jira.sendChat(event.detail)}
          on:resolve={(event) => void jira.resolveProposal(event.detail.messageId, event.detail.approve)}
          on:openThread={(event) => void jira.openThread(event.detail)}
          on:newThread={() => jira.newThread()}
          on:deleteThread={(event) => void jira.deleteThread(event.detail)}
          on:close={toggleAgent}
        />
      {/if}
    </div>
  {/if}
</section>

<style>
  .jira-view {
    display: flex;
    height: 100%;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    color: var(--color-text-primary);
    background: var(--color-app-bg-paint, var(--color-app-bg));
    overflow: hidden;
  }

  .jira-head {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    gap: 0 14px;
    min-height: 44px;
    padding: 0 12px 0 8px;
    border-bottom: 1px solid var(--rule);
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  .head-title {
    display: flex;
    align-items: baseline;
    align-self: center;
    gap: 8px;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: var(--weight-strong);
  }

  .head-title p,
  .list-empty,
  .setup-card p {
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .head-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }

  /* Tabs sit on the header's bottom rule; the active one draws over it. */
  .filter-row {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    gap: 2px;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding: 0 9px;
    border: 0;
    border-radius: var(--radius-s);
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }

  .icon-button {
    align-self: center;
    width: 28px;
    padding: 0;
    justify-content: center;
    color: var(--color-text-muted);
  }

  .icon-button:hover,
  .icon-button.active {
    color: var(--color-text-primary);
  }

  .icon-button:hover {
    background: var(--color-surface-2);
  }

  .filter-tab {
    min-height: 44px;
    margin-bottom: -1px;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    color: var(--color-text-muted);
  }

  .filter-tab:hover {
    color: var(--color-text-primary);
  }

  .filter-tab.active {
    border-bottom-color: var(--color-accent-hot);
    color: var(--color-text-primary);
    font-weight: var(--weight-medium);
  }

  .agent-tab {
    display: inline-flex;
    align-items: stretch;
  }

  .agent-tab .filter-tab {
    padding-right: 4px;
  }

  .tab-remove {
    align-self: center;
    width: 20px;
    min-height: 20px;
    padding: 0;
    justify-content: center;
    color: var(--color-text-muted);
  }

  .tab-remove:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .quiet-button {
    color: var(--color-text-muted);
  }

  .quiet-button:hover:not(:disabled) {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .quiet-button.active {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .primary-button {
    border: 1px solid color-mix(in srgb, var(--color-accent-primary) 60%, transparent);
    background: color-mix(in srgb, var(--color-accent-primary) 16%, transparent);
    font-weight: var(--weight-medium);
  }

  button:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: -2px;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .jira-error {
    margin: 10px 16px 0;
    padding: 8px 10px;
    border-left: 2px solid var(--color-status-error);
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 8%, transparent);
    font-size: 12.5px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .setup-card {
    display: grid;
    gap: 12px;
    justify-items: start;
    max-width: 600px;
    margin: 32px 24px;
  }

  .setup-card h2 {
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: var(--weight-strong);
  }

  .setup-card ol {
    display: grid;
    gap: 6px;
    margin: 0;
    padding-left: 20px;
    font-size: 13px;
    line-height: 1.5;
  }

  .jira-body {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: 340px minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .jira-body.with-agent {
    grid-template-columns: 320px minmax(0, 1fr) 360px;
  }

  .jira-body.without-list {
    grid-template-columns: minmax(0, 1fr);
  }

  .jira-body.without-list.with-agent {
    grid-template-columns: minmax(0, 1fr) 380px;
  }

  .list-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: 1px solid var(--rule);
  }

  .list-search {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border-bottom: 1px solid var(--rule);
    color: var(--color-text-muted);
  }

  .list-search:focus-within {
    color: var(--color-text-primary);
    box-shadow: inset 0 -1px 0 var(--color-accent-hot);
  }

  .list-search input {
    flex: 1;
    min-width: 0;
    height: 36px;
    border: 0;
    padding: 0;
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 13px;
    outline: none;
  }

  /* Block layout: a flex column would shrink the rows to fit the pane. */
  .issue-list {
    flex: 1 1 auto;
    min-height: 0;
    padding-bottom: 12px;
    overflow-y: auto;
  }

  .list-empty {
    padding: 24px 14px;
  }

  .group-head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 6px;
    color: var(--color-text-muted);
    background: var(--color-app-bg-paint, var(--color-app-bg));
    font-size: 11.5px;
    font-weight: var(--weight-medium);
  }

  .group-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-muted);
  }

  .group-dot.indeterminate {
    background: var(--color-accent-hot);
  }

  .group-dot.done {
    background: var(--color-status-success);
  }

  .group-count {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: var(--weight-regular);
  }

  /* A log line: the ticket key sits in a fixed gutter beside its text. */
  .issue-row {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: start;
    gap: 3px 8px;
    width: 100%;
    min-height: 0;
    padding: 8px 14px 9px 12px;
    border-left: 2px solid transparent;
    border-radius: 0;
    text-align: left;
  }

  .issue-row:hover {
    background: color-mix(in srgb, var(--color-surface-2) 60%, transparent);
  }

  .issue-row.selected {
    border-left-color: var(--color-accent-hot);
    background: var(--color-surface-2);
  }

  .row-key {
    grid-row: span 2;
    padding-top: 1px;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: nowrap;
  }

  .issue-row.selected .row-key {
    color: var(--color-accent-hot);
  }

  .row-summary {
    display: -webkit-box;
    overflow: hidden;
    font-size: 13px;
    line-height: 1.4;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .row-meta {
    display: flex;
    gap: 10px;
    min-width: 0;
    color: var(--color-text-muted);
    font-size: 11.5px;
    white-space: nowrap;
  }

  .row-meta .unassigned {
    opacity: 0.7;
  }

  .row-priority {
    margin-left: auto;
  }

  .row-priority.high {
    color: var(--color-status-warning);
  }

  .row-priority.urgent {
    color: var(--color-status-error);
    font-weight: var(--weight-medium);
  }

  .issue-pane {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  @media (max-width: 1180px) {
    .jira-body.with-agent {
      grid-template-columns: 260px minmax(0, 1fr) 320px;
    }
  }

  @media (max-width: 920px) {
    .jira-body,
    .jira-body.with-agent,
    .jira-body.without-list.with-agent {
      grid-template-columns: 1fr;
      grid-auto-rows: minmax(260px, 1fr);
      overflow-y: auto;
    }

    .list-pane {
      border-right: 0;
      border-bottom: 1px solid var(--rule);
    }
  }
</style>
