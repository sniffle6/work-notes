<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { JiraIssueDraft } from "$lib/types";

  type Props = {
    draft: JiraIssueDraft;
    issueTypes: string[];
    projectKey?: string;
    saving?: boolean;
    /** The agent is working and may rewrite the fields. */
    agentBusy?: boolean;
  };

  let { draft, issueTypes, projectKey = "", saving = false, agentBusy = false }: Props = $props();
  let issueType = $state("");
  let summary = $state("");
  let description = $state("");
  // Plain variable on purpose: it only remembers which draft seeded the fields.
  let seededDraft: JiraIssueDraft | null = null;

  const dispatch = createEventDispatcher<{
    submit: JiraIssueDraft;
    /** The user changed a field. */
    edit: Pick<JiraIssueDraft, "issueType" | "summary" | "description">;
    cancel: void;
  }>();

  $effect(() => {
    if (draft !== seededDraft) {
      seededDraft = draft;
      issueType = draft.issueType;
      summary = draft.summary;
      description = draft.description;
    }
  });

  // Text fields report on input; the issue type select updates on change.
  function reportEdit() {
    dispatch("edit", { issueType, summary, description });
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!summary.trim() || saving) {
      return;
    }
    dispatch("submit", {
      issueType,
      summary: summary.trim(),
      description,
      noteId: draft.noteId ?? null,
    });
  }
</script>

<form
  class="jira-form"
  aria-label="New ticket"
  onsubmit={submit}
  oninput={reportEdit}
  onchange={reportEdit}
>
  <header>
    <h1>New ticket{projectKey ? ` in ${projectKey}` : ""}</h1>
    {#if draft.noteId}
      <p>The ticket key will be linked to the note it was created from.</p>
    {/if}
    <p role="status">
      {agentBusy
        ? "The agent is working on this draft."
        : "Nothing is in Jira until you create the ticket. Ask the agent to rework the draft."}
    </p>
  </header>

  <label>
    <span>Issue type</span>
    <select bind:value={issueType} aria-label="Issue type" disabled={saving}>
      {#each issueTypes as type}
        <option value={type}>{type}</option>
      {/each}
    </select>
  </label>

  <label>
    <span>Summary</span>
    <input bind:value={summary} aria-label="Summary" maxlength="255" disabled={saving} />
  </label>

  <label>
    <span>Description</span>
    <textarea bind:value={description} aria-label="Description" rows="14" disabled={saving}></textarea>
    <small>Blank lines separate paragraphs. Start lines with "- " for bullets or "## " for headings.</small>
  </label>

  <div class="form-actions">
    <button type="button" disabled={saving} onclick={() => dispatch("cancel")}>Cancel</button>
    <button class="primary-button" type="submit" disabled={saving || !summary.trim()}>
      {saving ? "Creating" : "Create ticket"}
    </button>
  </div>
</form>

<style>
  .jira-form {
    display: grid;
    align-content: start;
    gap: 12px;
    min-width: 0;
    min-height: 0;
    height: 100%;
    padding: 16px;
    color: var(--color-text-primary);
    background: var(--color-app-bg-paint, var(--color-app-bg));
    overflow: auto;
  }

  h1,
  p {
    margin: 0;
  }

  h1 {
    font-size: 16px;
    font-weight: var(--weight-strong);
  }

  p,
  small,
  label > span {
    color: var(--color-text-muted);
    font-size: 11.5px;
    font-weight: var(--weight-medium);
  }

  label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  select,
  input,
  textarea {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 7px 9px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 13px;
    line-height: 1.45;
    outline: none;
  }

  select {
    width: auto;
    min-width: 160px;
    justify-self: start;
  }

  textarea {
    font-family: var(--font-mono);
    font-size: 12.5px;
    resize: vertical;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  button {
    min-height: 28px;
    padding: 0 10px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    font: inherit;
    font-size: 12px;
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .primary-button {
    border-color: var(--color-accent-primary);
    background: color-mix(in srgb, var(--color-accent-primary) 18%, var(--color-surface-1));
  }

  select:focus-visible,
  input:focus-visible,
  textarea:focus-visible,
  button:focus-visible {
    border-color: var(--color-accent-primary);
    outline: 1px solid var(--color-accent-primary);
    outline-offset: 1px;
  }

  button:disabled,
  select:disabled,
  input:disabled,
  textarea:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
</style>
