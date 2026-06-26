# Parser Contract

Parsing is background enrichment. It must never be required for note capture and must never replace the saved raw note.

## Ownership

- `schemas/parse-note.schema.json`: checked-in JSON schema for provider output.
- `src-tauri/src/parser/types.rs`: parser result, parser output, provider trait, parser errors.
- `src-tauri/src/parser/prompt.rs`: parse and reparse prompt text.
- `src-tauri/src/parser/codex_provider.rs`: the only module that executes local `codex`.
- `src-tauri/src/parser/validate.rs`: schema validation.
- `src-tauri/src/parser/result_applier.rs`: trust rules for applying parser output.
- `src-tauri/src/services/parse_queue.rs`: queue lifecycle, retries, transactions, parse runs, background worker.

## Codex Invocation

The provider invokes local Codex CLI with this shape:

```powershell
codex exec `
  --ephemeral `
  --skip-git-repo-check `
  --output-schema schemas/parse-note.schema.json `
  -o <temporary parse-result.json> `
  -
```

On Windows the default configured executable is `codex.cmd`. The app writes the parser prompt and raw note to stdin, reads the output file, validates JSON against `schemas/parse-note.schema.json`, and deserializes the result before applying it.

Settings that affect later jobs:

- `codexCommandPath`
- `parserTimeoutSeconds`
- `parserMaxRetries`
- `linkedWorkspacePaths`

The background worker reads persisted settings as it processes jobs, so settings changes do not require restarting the app.

When `linkedWorkspacePaths` contains existing directories, the parser prompt lists them as optional repo/directory context and the Codex process runs from the first existing linked directory. The parser must inspect linked context only when useful and must not claim repo facts unless the raw note states them or the linked context was inspected.

## Note Directives

A note author can embed instructions for the parser inline using the `@codex:`
marker. Before the prompt is built, the provider deterministically splits the
raw note into a body (content to clean) and an ordered list of directives via
`parser/directives.rs`; the directives are rendered in a separate "Note
instructions" prompt section and the body is what the agent cleans.

- A directive starts at a line whose first non-whitespace text is `@codex:`
  (ASCII case-insensitive).
- A directive is paragraph-bounded: it continues across following lines until a
  blank line, the start of the next `@codex:` line, or end of note. Continuation
  lines are joined into one instruction with single spaces.
- Because a directive runs until a blank line, leave a blank line before
  resuming note content after a directive, or that content is absorbed into the
  directive.
- A bare `@codex:` with no text is ignored.
- Directive text is never written into cleaned output, and the raw note is never
  modified — the `@codex:` lines remain in the stored raw text.
- Directives guide how the note is cleaned, tagged, and enriched, but are
  subordinate to the safety rules: they cannot make the parser invent facts or
  claim repo facts it did not inspect.

## Output Shape

Parser output must be JSON with camelCase fields:

```json
{
  "title": "Verify QA Config Flag",
  "cleanedText": "## Context\n\nMike said the deploy moved to Friday.\n\n## Follow-up\n\n- Check the `config` flag before QA gets the build.",
  "summary": "Deploy moved to Friday; verify config flag before QA build.",
  "tags": [
    { "name": "Mike", "kind": "person", "confidence": 0.92 }
  ],
  "actionItems": [
    {
      "text": "Check the config flag before QA gets the build.",
      "owner": null,
      "dueDate": null,
      "confidence": 0.83,
      "requiresReview": true
    }
  ]
}
```

`title` should be a short task-oriented label, usually 3-8 words, suitable for the inbox row.
`cleanedText` should be Markdown that cleans the note without erasing useful source structure. Use headings and bullets for ordinary shorthand notes. Preserve multi-line code, config, enum, command, stack trace, and log snippets mostly verbatim in fenced code blocks. Keep identifiers, methods, file names, fields, and enum values as inline code. For copied tickets, preserve labels and order such as Actual, Expected, Steps, Environment, and Notes instead of rewriting the ticket into unrelated categories. Do not repeat the title as the first Markdown heading.

Allowed tag kinds:

```text
person
project
topic
urgency
category
custom
```

Confidence values must be numbers from `0` through `1`. Additional properties are rejected.

## Trust Rules

- Raw note text is never overwritten by parser output.
- Title, cleaned text, and summary may be auto-applied after schema validation.
- Parser tags are applied with source `ai`.
- Parser actions are created as `suggested`.
- Dates, owners, commitments, and inferred obligations require review.
- If any parsed action has `requiresReview: true`, the note review status becomes `needs_review`.
- Parser failure leaves raw text intact and records the failure state.
- Invalid JSON or schema-invalid JSON marks the job failed.
- Parse runs keep the raw provider response separately from normalized parsed JSON.
- Retry may replace unreviewed parser-suggested tags/actions, but accepted/dismissed/done actions must be preserved.

## Queue Rules

- Capture creates the raw note and queued parse job through the capture persistence path.
- Active queued/parsing jobs for the same note should be reused instead of duplicated.
- The v1 worker processes one parse job at a time.
- Successful parse application should be transactional.
- A failed job stores `last_error` and can be retried.
- Reparse feedback is stored with the parse job/run so prompt iteration is auditable.

## Failure Modes To Preserve

The provider and queue should distinguish these cases where practical:

- `codex` command missing.
- Codex CLI not logged in.
- Timeout.
- Non-zero exit.
- Missing output file.
- Invalid JSON.
- JSON schema validation failure.
- SQLite write failure while applying output.

