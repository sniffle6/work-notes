<script lang="ts">
  import Bot from "@lucide/svelte/icons/bot";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import Plus from "@lucide/svelte/icons/plus";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";
  import { createEventDispatcher } from "svelte";
  import { filterIssues, groupIssuesByStatus, issueKeyInQuery, jiraTextSearch, priorityTone } from "$lib/jira-list";
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
  let workspaceWidth = $state(0);
  let collapsedStatuses = $state<string[]>([]);
  let sortOrder = $state("jira");
  let draftsOnly = $state(false);

  // The store instance is fixed for the lifetime of the view.
  // svelte-ignore state_referenced_locally
  const {
    status,
    activeFilter,
    agentFilter,
    searchFilter,
    issues,
    selectedIssue,
    selectedWriting,
    ticketDrafts,
    pendingProposals,
    pendingProposalsError,
    pendingProposalsLoading,
    focusedProposalId,
    issueTypes,
    draft,
    draftVisible,
    draftBeforeAgent,
    draftAgentOpen,
    loadingIssues,
    loadingIssue,
    issuesSavedAt,
    issueSavedAt,
    savingIssue,
    error,
    writingError,
    writingSaving,
    writingReady,
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

  const visibleIssues = $derived(filterIssues($issues, searchQuery).filter((issue) => !draftsOnly || $ticketDrafts[issue.key]));
  const outsideDrafts = $derived(Object.entries($ticketDrafts).filter(([key, writing]) =>
    !$issues.some((issue) => issue.key === key) &&
    `${key} ${writing.summary}`.toLowerCase().includes(searchQuery.trim().toLowerCase()),
  ));
  const sharedAssignee = $derived(
    visibleIssues.length > 1 && visibleIssues[0].assignee &&
    visibleIssues.every((issue) => issue.assignee === visibleIssues[0].assignee)
      ? visibleIssues[0].assignee : null,
  );
  const groups = $derived(groupIssuesByStatus(sortOrder === "title"
    ? [...visibleIssues].sort((left, right) => left.summary.localeCompare(right.summary))
    : visibleIssues));
  const countLabel = $derived.by(() => {
    const total = `${$issues.length} ${$issues.length === 1 ? "ticket" : "tickets"}`;
    const project = $status?.projectKey ? ` · ${$status.projectKey}` : "";
    if (draftsOnly) {
      const count = visibleIssues.length + outsideDrafts.length;
      return `${count} ${count === 1 ? "ticket" : "tickets"} with drafts${project}`;
    }
    return searchQuery.trim() ? `${visibleIssues.length} of ${total}${project}` : `${total}${project}`;
  });

  // Looking beyond the loaded tickets: a typed key opens that ticket, other text searches Jira.
  const typedKey = $derived(issueKeyInQuery(searchQuery));
  const jiraSearchLabel = $derived(
    typedKey ? `Open ${typedKey}` :
      jiraTextSearch(searchQuery, $status?.projectKey ?? "") ? `Search Jira for “${searchQuery.trim()}”` : null,
  );

  function searchJira() {
    const query = searchQuery;
    // Jira matched the whole ticket text, so the box must not narrow its results by title.
    if (!typedKey) {
      searchQuery = "";
    }
    void jira.searchJira(query);
  }

  function searchJiraOnEnter(event: KeyboardEvent) {
    if (event.key === "Enter" && jiraSearchLabel) {
      event.preventDefault();
      searchJira();
    }
  }

  // A draft started from a note brings the agent along, whatever the saved choice.
  const agentVisible = $derived(agentOpen || ($draftAgentOpen && $draft !== null));
  const listShown = $derived(!listHidden && !(workspaceWidth > 0 && (
    (workspaceWidth <= 1100 && agentVisible) || (workspaceWidth < 760 && $draftVisible)
  )));

  $effect(() => {
    if ($draftVisible && workspaceWidth > 0 && workspaceWidth < 760) {
      agentOpen = false;
      jira.closeDraftAgent();
      listHidden = true;
    }
  });

  function toggleAgent() {
    agentOpen = !agentVisible;
    writeFlag(AGENT_OPEN_KEY, agentOpen);
    if (!agentOpen) {
      jira.closeDraftAgent();
    }
  }

  function toggleList() {
    const showList = !listShown;
    if (showList) {
      if (workspaceWidth > 0 && workspaceWidth <= 1100 && agentVisible) {
        toggleAgent();
      }
      if (workspaceWidth > 0 && workspaceWidth < 760) {
        jira.hideDraft();
      }
    }
    listHidden = !showList;
    writeFlag(LIST_HIDDEN_KEY, listHidden);
  }

  function openTicket(key: string) {
    if (workspaceWidth > 0 && workspaceWidth < 760) {
      listHidden = true;
    }
    void jira.selectIssue(key);
  }

  function openDraft() {
    if (workspaceWidth > 0 && workspaceWidth < 760) {
      listHidden = true;
      agentOpen = false;
      jira.closeDraftAgent();
    }
    void jira.startDraft();
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

<section class="jira-view" aria-label="Jira" bind:clientWidth={workspaceWidth}>
  <header class="jira-head">
    {#if $status?.configured}
      <button
        class="icon-button"
        class:active={listShown}
        type="button"
        aria-label={listShown ? "Hide ticket list" : "Show ticket list"}
        title={listShown ? "Hide ticket list ( [ )" : "Show ticket list ( [ )"}
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
        {:else if $writingSaving}
          Saving drafts…
        {:else if $loadingIssues}
          {$issues.length > 0 ? "Refreshing tickets" : "Loading tickets"}
        {:else}
          {countLabel}
          {#if $issuesSavedAt}
            <span title={`Saved ${new Date($issuesSavedAt).toLocaleString()}. Refresh to ask Jira again.`}>· saved copy</span>
          {/if}
        {/if}
      </p>
    </div>

    {#if $status?.configured}
      <div class="filter-row" role="group" aria-label="Ticket filters">
        {#each $status.filters as filter (filter.name)}
          <button
            class="filter-tab"
            class:active={$activeFilter?.name === filter.name && $activeFilter !== $agentFilter && $activeFilter !== $searchFilter}
            type="button"
            aria-pressed={$activeFilter?.name === filter.name && $activeFilter !== $agentFilter && $activeFilter !== $searchFilter}
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
        {#if $searchFilter}
          {@const jiraSearch = $searchFilter}
          <span class="agent-tab" class:active={$activeFilter === jiraSearch}>
            <button
              class="filter-tab"
              class:active={$activeFilter === jiraSearch}
              type="button"
              aria-pressed={$activeFilter === jiraSearch}
              title={`Jira search: ${jiraSearch.jql}`}
              onclick={() => void jira.selectFilter(jiraSearch)}
            >
              <Search size={13} strokeWidth={1.8} aria-hidden="true" />
              {jiraSearch.name}
            </button>
            <button
              class="tab-remove"
              type="button"
              aria-label={`Remove ${jiraSearch.name}`}
              title="Remove this search"
              onclick={() => void jira.clearSearchFilter()}
            >
              <X size={12} strokeWidth={2} aria-hidden="true" />
            </button>
          </span>
        {/if}
      </div>

      <div class="head-actions">
        <button class="quiet-button" type="button" aria-label="Refresh" title="Refresh tickets" disabled={$loadingIssues} onclick={() => void jira.refreshIssues()}>
          <RefreshCw size={14} strokeWidth={1.8} aria-hidden="true" />
          <span class="action-label">Refresh</span>
        </button>
        <button class="quiet-button" type="button" aria-label={$draft ? "Resume draft" : "New ticket"} title={$draft ? "Resume draft" : "New ticket"} onclick={openDraft}>
          <Plus size={14} strokeWidth={1.8} aria-hidden="true" />
          <span class="action-label">{$draft ? "Resume draft" : "New ticket"}</span>
        </button>
        <button
          class="quiet-button"
          class:active={agentVisible}
          type="button"
          aria-pressed={agentVisible}
          aria-label="Agent"
          aria-describedby="jira-pending-count"
          title={agentVisible ? "Hide the agent ( ] )" : "Ask the agent ( ] )"}
          onclick={toggleAgent}
        >
          <Bot size={14} strokeWidth={1.8} aria-hidden="true" />
          <span class="action-label">Agent</span>
          {#if $pendingProposals.length > 0}<span id="jira-pending-count" class="pending-count" aria-label={`${$pendingProposals.length} pending approvals`}>{$pendingProposals.length}</span>{/if}
          {#if $pendingProposalsError}<span title="Pending approvals could not be refreshed" aria-label="Pending approvals unavailable">!</span>{/if}
        </button>
      </div>
    {/if}
  </header>

  {#if $error}
    <p class="jira-error" role="alert">{$error}</p>
  {/if}
  {#if $writingError}
    <p class="jira-error" role="alert">
      {$writingError}
      <button type="button" disabled={$writingSaving} onclick={() => jira.retryWriting()}>Retry saving drafts</button>
    </p>
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
  {:else if $status && $writingReady}
    <div class="jira-body" class:with-agent={agentVisible} class:without-list={listHidden} class:has-draft={$draftVisible}>
      {#if !listHidden}
        <section class="list-pane" aria-label="Ticket list">
          <label class="list-search">
            <Search size={14} strokeWidth={1.8} aria-hidden="true" />
            <input
              type="search"
              bind:value={searchQuery}
              onkeydown={searchJiraOnEnter}
              aria-label="Search tickets"
              placeholder="Search tickets"
              spellcheck="false"
            />
          </label>

          <div class="list-options">
            <button class="draft-filter" class:active={draftsOnly} type="button" aria-pressed={draftsOnly}
              onclick={() => { draftsOnly = !draftsOnly; }}>Has drafts <span>{Object.keys($ticketDrafts).length}</span></button>
            <select bind:value={sortOrder} aria-label="Sort tickets within each status" title="Sort tickets within each status">
              <option value="jira">Jira order</option>
              <option value="title">Title A–Z</option>
            </select>
          </div>

          <div class="issue-list" aria-label="Tickets">
            {#if sharedAssignee}<p class="outside-label" title="All shown tickets are assigned to this person">{sharedAssignee}</p>{/if}
            {#if draftsOnly && visibleIssues.length === 0 && outsideDrafts.length === 0}
              <p class="list-empty">{searchQuery.trim() ? "No drafts match the search" : "No ticket drafts"}</p>
            {:else if $issues.length === 0 && !draftsOnly}
              <p class="list-empty">{$loadingIssues ? "Loading tickets" : "No tickets match this filter"}</p>
            {:else if visibleIssues.length === 0 && !draftsOnly}
              <p class="list-empty">No tickets match the search</p>
            {:else}
              {#each groups as group (group.status)}
                {@const expanded = draftsOnly || Boolean(searchQuery.trim()) || !collapsedStatuses.includes(group.status)}
                <div class="issue-group" role="group" aria-label={groupLabel(group.status, group.issues.length)}>
                  <button class="group-head" type="button"
                    aria-label={`${expanded ? "Collapse" : "Expand"} ${group.status}`}
                    aria-expanded={expanded}
                    disabled={draftsOnly || Boolean(searchQuery.trim())}
                    onclick={() => { collapsedStatuses = expanded
                      ? [...collapsedStatuses, group.status]
                      : collapsedStatuses.filter((status) => status !== group.status); }}>
                    {#if expanded}<ChevronDown size={12} aria-hidden="true" />{:else}<ChevronRight size={12} aria-hidden="true" />{/if}
                    <span class={`group-dot ${group.statusCategory}`}></span>
                    <span class="group-name">{group.status}</span>
                    <span class="group-count">{group.issues.length}</span>
                  </button>
                  {#if expanded}
                  {#each group.issues as issue (issue.key)}
                    <button
                      class="issue-row"
                      class:selected={$selectedIssue?.key === issue.key && !$draftVisible}
                      type="button"
                      aria-label={`${issue.key}: ${issue.summary}`}
                      aria-describedby={$ticketDrafts[issue.key] ? `draft-${issue.key}` : undefined}
                      title={issue.summary}
                      aria-current={$selectedIssue?.key === issue.key && !$draftVisible ? "true" : undefined}
                      onclick={() => openTicket(issue.key)}
                    >
                      <span class="row-summary">{issue.summary}</span>
                      <span class="row-meta">
                        <span class="row-key">{issue.key}</span>
                        <span>{issue.issueType}</span>
                        {#if !sharedAssignee}
                          <span class="row-assignee" class:unassigned={!issue.assignee}>{issue.assignee ?? "Unassigned"}</span>
                        {/if}
                        {#if issue.priority && priorityTone(issue.priority) !== "normal"}
                          <span class={`row-priority ${priorityTone(issue.priority)}`}>{issue.priority}</span>
                        {/if}
                      </span>
                      {#if $ticketDrafts[issue.key]}
                        <span class="row-draft" id={`draft-${issue.key}`}>{[$ticketDrafts[issue.key].comment.trim() ? "Unsent comment" : "", $ticketDrafts[issue.key].editing ? "Edit in progress" : ""].filter(Boolean).join(" · ")}</span>
                      {/if}
                    </button>
                  {/each}
                  {/if}
                </div>
              {/each}
            {/if}
            {#if jiraSearchLabel && !draftsOnly}
              <button class="search-jira" type="button" title="Look beyond the loaded tickets (Enter)" onclick={searchJira}>
                <Search size={13} strokeWidth={1.8} aria-hidden="true" />
                {jiraSearchLabel}
              </button>
            {/if}
            {#if draftsOnly && outsideDrafts.length > 0}
              <div class="issue-group" role="group" aria-label="Drafts outside this ticket list">
                <p class="outside-label">Outside this ticket list</p>
                {#each outsideDrafts as [key, writing] (key)}
                  <button class="issue-row" class:selected={$selectedIssue?.key === key && !$draftVisible}
                    type="button" aria-label={`${key}: ${writing.summary || "Saved draft"}`} aria-describedby={`draft-${key}`} onclick={() => openTicket(key)}>
                    <span class="row-summary">{writing.summary || key}</span>
                    <span class="row-key">{key}</span>
                    <span class="row-draft" id={`draft-${key}`}>{[writing.comment.trim() ? "Unsent comment" : "", writing.editing ? "Edit in progress" : ""].filter(Boolean).join(" · ")}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      {/if}

      <div class="issue-pane">
        {#if $draft && $draftVisible}
          <JiraIssueForm
            draft={$draft}
            issueTypes={$issueTypes}
            projectKey={$status.projectKey}
            saving={$savingIssue}
            agentBusy={$agentBusy}
            canUndo={Boolean($draftBeforeAgent)}
            on:undo={() => jira.undoDraftRewrite()}
            on:edit={(event) => jira.editDraft(event.detail)}
            on:submit={async (event) => {
              const issueKey = await jira.submitDraft(event.detail);
              if (issueKey) {
                dispatch("ticketCreated", { issueKey, noteId: event.detail.noteId ?? null });
              }
            }}
            on:cancel={() => jira.cancelDraft()}
            on:close={() => jira.hideDraft()}
          />
        {:else}
          <JiraIssueDetail
            issue={$selectedIssue}
            writing={$selectedWriting}
            on:writing={(event) => jira.rememberWriting(event.detail.key, event.detail.writing)}
            loading={$loadingIssue}
            saving={$savingIssue}
            savedAt={$issueSavedAt}
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
          pendingProposals={$pendingProposals}
          pendingError={$pendingProposalsError}
          pendingLoading={$pendingProposalsLoading}
          focusedProposalId={$focusedProposalId}
          on:openProposal={(event) => void jira.openProposal(event.detail)}
          on:retryPending={() => void jira.refreshPendingProposals()}
          busy={$agentBusy}
          busyProposalId={$busyProposalId}
          error={$chatError}
          focusIssueKey={$draftVisible ? null : ($selectedIssue?.key ?? null)}
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
  .search-jira {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    border-top: 1px solid var(--rule);
    border-radius: 0;
    background: transparent;
    color: var(--color-accent-hot);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .search-jira:hover {
    background: var(--color-surface-2);
  }

  .draft-filter {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    border: 0;
    background: transparent;
    color: var(--color-text-secondary);
    font: inherit;
    cursor: pointer;
    padding: 2px 0;
  }

  .draft-filter.active, .row-draft, .pending-count {
    color: var(--color-accent-hot);
  }

  .draft-filter:focus-visible {
    outline: 1px solid var(--color-accent-hot);
    outline-offset: 3px;
  }

  .draft-filter.active {
    text-decoration: underline;
    text-underline-offset: 4px;
  }

  .row-draft {
    font-size: 11px;
  }

  .outside-label {
    margin: 10px 12px 4px;
    font-size: 11px;
    color: var(--color-text-secondary);
  }

  .jira-view {
    container: jira / inline-size;
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
    flex: 0 0 auto;
    flex-wrap: nowrap;
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
    flex: 0 0 auto;
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
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }

  /* Tabs stretch inside the header rule; extending below it creates vertical overflow. */
  .filter-row {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: thin;
    white-space: nowrap;
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
    flex-shrink: 0;
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
    grid-template-columns: 300px minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .jira-body.with-agent {
    grid-template-columns: 300px minmax(0, 1fr) 340px;
  }

  .jira-body.without-list {
    grid-template-columns: minmax(0, 1fr);
  }

  .jira-body.without-list.with-agent {
    grid-template-columns: minmax(0, 1fr) 340px;
  }

  .list-pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: 1px solid var(--rule);
    overflow: hidden;
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

  .issue-group + .issue-group {
    margin-top: 8px;
    border-top: 1px solid var(--color-border-default);
  }

  .group-head {
    width: 100%;
    border-radius: 0;
    text-align: left;
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 8px;
    color: var(--color-text-muted);
    background: var(--color-app-bg-paint, var(--color-app-bg));
    font-size: 11.5px;
    font-weight: var(--weight-medium);
  }

  .group-head:disabled {
    opacity: 1;
    cursor: default;
  }

  .group-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .issue-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-items: start;
    gap: 5px;
    width: 100%;
    min-height: 0;
    padding: 12px 14px 12px 12px;
    border-left: 2px solid transparent;
    border-radius: 0;
    text-align: left;
  }

  .issue-row:hover {
    background: color-mix(in srgb, var(--color-surface-2) 60%, transparent);
  }

  .issue-row + .issue-row {
    border-top: 1px solid var(--rule);
  }

  .issue-row.selected {
    border-left-color: var(--color-accent-hot);
    background: var(--color-surface-2);
  }

  .row-key {
    flex: 0 0 auto;
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
    gap: 8px;
    min-width: 0;
    color: var(--color-text-muted);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
  }

  .row-assignee {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .list-owner {
    min-width: 0;
    color: var(--color-text-muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .list-options {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--rule);
  }

  .list-options select {
    max-width: 120px;
    min-height: 28px;
    border: 0;
    border-radius: var(--radius-s);
    background: var(--color-app-bg);
    color: var(--color-text-muted);
    font: inherit;
    font-size: 12px;
  }

  .list-options select:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: 1px;
  }

  .row-meta .unassigned {
    opacity: 0.7;
  }

  .row-priority {
    flex: 0 0 auto;
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

  @container jira (max-width: 1100px) {
    .jira-body.with-agent {
      grid-template-columns: minmax(0, 1fr) 320px;
    }

    .jira-body.with-agent .list-pane {
      display: none;
    }

    .head-actions .action-label {
      display: none;
    }

    .head-title p {
      display: none;
    }
  }

  @container jira (max-width: 759px) {
    .jira-body,
    .jira-body.with-agent,
    .jira-body.without-list.with-agent {
      grid-template-columns: 1fr;
    }

    .jira-body:not(.without-list):not(.has-draft) .issue-pane,
    .jira-body.with-agent .issue-pane,
    .jira-body.has-draft .list-pane {
      display: none;
    }

    .jira-head {
      gap: 0 6px;
    }
  }
</style>
