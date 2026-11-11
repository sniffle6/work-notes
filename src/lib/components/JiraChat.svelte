<script lang="ts">
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import PanelRightClose from "@lucide/svelte/icons/panel-right-close";
  import Plus from "@lucide/svelte/icons/plus";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import { createEventDispatcher, tick } from "svelte";
  import type { JiraChatMessage, JiraChatThread } from "$lib/types";

  type Props = {
    threads: JiraChatThread[];
    activeThreadId: string | null;
    messages: JiraChatMessage[];
    busy?: boolean;
    busyProposalId?: string | null;
    error?: string | null;
    /** Ticket open beside the chat; the agent receives it as context. */
    focusIssueKey?: string | null;
  };

  const SUGGESTIONS = [
    "Show me the important tickets I need to take care of today",
    "Show everything that changed in the last week",
    "Move this ticket to Dev Complete and add a QA note",
  ];

  let {
    threads,
    activeThreadId,
    messages,
    busy = false,
    busyProposalId = null,
    error = null,
    focusIssueKey = null,
  }: Props = $props();
  let draft = $state("");
  let log = $state<HTMLDivElement | null>(null);
  let box = $state<HTMLTextAreaElement | null>(null);

  const dispatch = createEventDispatcher<{
    send: string;
    resolve: { messageId: string; approve: boolean };
    openThread: string;
    newThread: void;
    deleteThread: string;
    close: void;
  }>();

  $effect(() => {
    // Keep the newest message in view as the conversation grows.
    void messages.length;
    void busy;
    void tick().then(() => {
      if (log) {
        log.scrollTop = log.scrollHeight;
      }
    });
  });

  function send() {
    const text = draft.trim();
    if (!text || busy) {
      return;
    }
    draft = "";
    dispatch("send", text);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  }

  function useSuggestion(text: string) {
    draft = text;
    box?.focus();
  }

  function selectThread(event: Event) {
    const threadId = (event.currentTarget as HTMLSelectElement).value;
    if (threadId) {
      dispatch("openThread", threadId);
    } else {
      dispatch("newThread");
    }
  }

  function proposalOutcome(message: JiraChatMessage): string {
    switch (message.status) {
      case "approved":
        return "Applied";
      case "rejected":
        return "Rejected";
      case "failed":
        return "Failed";
      default:
        return "Needs your approval";
    }
  }
</script>

<section class="jira-chat" aria-label="Jira agent">
  <header class="chat-head">
    <select aria-label="Conversation" value={activeThreadId ?? ""} disabled={busy} onchange={selectThread}>
      <option value="">New conversation</option>
      {#each threads as thread (thread.id)}
        <option value={thread.id}>{thread.title}</option>
      {/each}
    </select>
    <button
      class="icon-button"
      type="button"
      aria-label="New chat"
      title="New chat"
      disabled={busy}
      onclick={() => dispatch("newThread")}
    >
      <Plus size={15} strokeWidth={1.8} aria-hidden="true" />
    </button>
    {#if activeThreadId}
      <button
        class="icon-button"
        type="button"
        aria-label="Delete conversation"
        title="Delete conversation"
        disabled={busy}
        onclick={() => activeThreadId && dispatch("deleteThread", activeThreadId)}
      >
        <Trash2 size={14} strokeWidth={1.8} aria-hidden="true" />
      </button>
    {/if}
    <button
      class="icon-button"
      type="button"
      aria-label="Hide agent"
      title="Hide agent ( ] )"
      onclick={() => dispatch("close")}
    >
      <PanelRightClose size={15} strokeWidth={1.8} aria-hidden="true" />
    </button>
  </header>

  <div class="chat-log" role="log" aria-live="polite" bind:this={log}>
    {#if messages.length === 0 && !busy}
      <div class="chat-hint">
        <p>
          Ask in plain words. The agent can put tickets in the list, open one, and look things up. It asks before it
          changes anything in Jira.
        </p>
        {#each SUGGESTIONS as suggestion}
          <button type="button" onclick={() => useSuggestion(suggestion)}>{suggestion}</button>
        {/each}
      </div>
    {/if}

    {#each messages as message (message.id)}
      {#if message.role === "user"}
        <div class="bubble user">{message.content}</div>
      {:else if message.role === "assistant"}
        <div class="bubble assistant">{message.content}</div>
      {:else if message.role === "tool"}
        <details class="lookup" class:failed={message.status === "error"}>
          <summary>{message.content}</summary>
          <pre>{message.result ?? ""}</pre>
        </details>
      {:else}
        <div
          class={`proposal ${message.status ?? "pending"}`}
          role="group"
          aria-label="Proposed change"
        >
          <div class="proposal-head">
            <strong>Proposed change</strong>
            <span>{proposalOutcome(message)}</span>
          </div>
          <pre>{message.content}</pre>
          {#if message.result}
            <p class="proposal-result">{message.result}</p>
          {/if}
          {#if message.status === "pending"}
            <div class="proposal-actions">
              <button
                class="text-button"
                type="button"
                disabled={busyProposalId !== null}
                onclick={() => dispatch("resolve", { messageId: message.id, approve: false })}
              >
                Reject
              </button>
              <button
                class="primary-button"
                type="button"
                disabled={busyProposalId !== null}
                onclick={() => dispatch("resolve", { messageId: message.id, approve: true })}
              >
                Approve
              </button>
            </div>
          {/if}
        </div>
      {/if}
    {/each}

    {#if busy}
      <div class="working" role="status">
        <span class="working-dot" aria-hidden="true"></span>
        <span>Agent is working</span>
      </div>
    {/if}
  </div>

  {#if error}
    <p class="chat-error" role="alert">{error}</p>
  {/if}

  <form
    class="chat-form"
    onsubmit={(event) => {
      event.preventDefault();
      send();
    }}
  >
    <textarea
      bind:this={box}
      bind:value={draft}
      aria-label="Message the Jira agent"
      rows="3"
      placeholder="Ask about tickets, or say what to show or change"
      onkeydown={handleKeydown}
    ></textarea>
    <div class="chat-actions">
      <small>
        {#if focusIssueKey}
          <span class="context-key">{focusIssueKey}</span> is open ·
        {/if}
        Enter sends
      </small>
      <button class="send-button" type="submit" aria-label="Send" title="Send" disabled={busy || !draft.trim()}>
        <ArrowUp size={15} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  </form>
</section>

<style>
  .jira-chat {
    display: flex;
    min-width: 0;
    min-height: 0;
    height: 100%;
    flex-direction: column;
    border-left: 1px solid var(--rule);
    color: var(--color-text-primary);
    background: color-mix(in srgb, var(--color-surface-1) 40%, var(--color-app-bg));
    overflow: hidden;
  }

  .chat-head {
    display: flex;
    align-items: center;
    gap: 2px;
    min-height: 44px;
    padding: 0 8px 0 10px;
    border-bottom: 1px solid var(--rule);
  }

  p,
  pre {
    margin: 0;
  }

  select {
    flex: 1 1 auto;
    min-width: 0;
    height: 28px;
    margin-right: 4px;
    padding: 0 4px;
    border: 0;
    border-radius: var(--radius-s);
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 13px;
    font-weight: var(--weight-medium);
    text-overflow: ellipsis;
    cursor: pointer;
  }

  select:hover:not(:disabled) {
    background: var(--color-surface-2);
  }

  select option {
    color: var(--color-text-primary);
    background: var(--color-surface-1);
  }

  .chat-log {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    padding: 14px 14px 16px;
    overflow-y: auto;
  }

  .chat-hint {
    display: grid;
    justify-items: start;
    gap: 4px;
    color: var(--color-text-muted);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .chat-hint p {
    margin-bottom: 8px;
  }

  .chat-hint button {
    min-height: 0;
    padding: 4px 0;
    color: var(--color-text-primary);
    font-size: 12.5px;
    line-height: 1.4;
    text-align: left;
  }

  .chat-hint button::before {
    margin-right: 8px;
    color: var(--color-text-muted);
    content: "›";
  }

  .chat-hint button:hover {
    color: var(--color-accent-hot);
  }

  .bubble {
    font-size: 13px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .bubble.user {
    align-self: flex-end;
    max-width: 88%;
    padding: 7px 11px;
    border-radius: 10px 10px 2px 10px;
    background: var(--color-surface-2);
  }

  .bubble.assistant {
    align-self: stretch;
  }

  .lookup {
    align-self: stretch;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 11.5px;
  }

  .lookup summary {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .lookup summary:hover {
    color: var(--color-text-primary);
  }

  .lookup.failed summary {
    color: var(--color-status-error);
  }

  pre {
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .lookup pre {
    max-height: 220px;
    margin-top: 6px;
    padding: 8px 10px;
    border-radius: var(--radius-s);
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    overflow-y: auto;
  }

  /* A change waiting for a decision is the one loud thing in the conversation. */
  .proposal {
    display: grid;
    gap: 8px;
    padding: 10px 12px;
    border-left: 2px solid var(--color-status-warning);
    border-radius: 0 var(--radius-m) var(--radius-m) 0;
    background: color-mix(in srgb, var(--color-status-warning) 8%, transparent);
  }

  .proposal.approved {
    border-left-color: var(--color-status-success);
    background: color-mix(in srgb, var(--color-status-success) 7%, transparent);
  }

  .proposal.rejected {
    border-left-color: var(--color-border-strong);
    background: transparent;
    opacity: 0.7;
  }

  .proposal.failed {
    border-left-color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 8%, transparent);
  }

  .proposal-head,
  .proposal-actions,
  .chat-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .proposal-head strong {
    font-size: 12.5px;
    font-weight: var(--weight-medium);
  }

  .proposal-head span,
  small {
    color: var(--color-text-muted);
    font-size: 11.5px;
  }

  .proposal-result {
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }

  .proposal-actions {
    justify-content: flex-end;
  }

  .working {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-text-muted);
    font-size: 12.5px;
  }

  .working-dot {
    width: 11px;
    height: 11px;
    border: 2px solid color-mix(in srgb, var(--color-accent-hot) 24%, transparent);
    border-top-color: var(--color-accent-hot);
    border-radius: 50%;
    animation: working-spin 0.8s linear infinite;
  }

  @keyframes working-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .chat-error {
    margin: 0 14px 8px;
    padding: 7px 10px;
    border-left: 2px solid var(--color-status-error);
    color: var(--color-status-error);
    background: color-mix(in srgb, var(--color-status-error) 8%, transparent);
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }

  .chat-form {
    display: grid;
    gap: 4px;
    margin: 0 10px 10px;
    padding: 8px 8px 6px 10px;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-m);
    background: var(--color-surface-input);
  }

  .chat-form:focus-within {
    border-color: var(--color-accent-hot);
  }

  textarea {
    width: 100%;
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

  .context-key {
    font-family: var(--font-mono);
  }

  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-s);
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }

  .icon-button {
    flex: 0 0 auto;
    width: 28px;
    height: 28px;
    padding: 0;
    color: var(--color-text-muted);
  }

  .icon-button:hover:not(:disabled),
  .text-button:hover:not(:disabled) {
    color: var(--color-text-primary);
    background: var(--color-surface-2);
  }

  .text-button {
    color: var(--color-text-muted);
  }

  .primary-button {
    border: 1px solid color-mix(in srgb, var(--color-accent-primary) 60%, transparent);
    background: color-mix(in srgb, var(--color-accent-primary) 22%, transparent);
    font-weight: var(--weight-medium);
  }

  .send-button {
    width: 26px;
    padding: 0;
    border-radius: 50%;
    color: var(--color-app-bg);
    background: var(--color-accent-hot);
  }

  .send-button:disabled {
    color: var(--color-text-muted);
    background: var(--color-surface-2);
  }

  select:focus-visible,
  button:focus-visible {
    outline: 2px solid var(--color-accent-hot);
    outline-offset: 1px;
  }

  button:disabled,
  select:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .send-button:disabled {
    opacity: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    .working-dot {
      animation: none;
    }
  }
</style>
