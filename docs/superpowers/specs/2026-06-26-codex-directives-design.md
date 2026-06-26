# Inline `@codex:` Directives for the Note Parser — Design

## Purpose

A user wants to give the parser an instruction about how to clean/enrich a
raw note — e.g. "tag this urgent", "rewrite as a checklist", "pull the pinned
version from repo-b" — by typing it inline in the note. Today they type a bare
`codex: …` line, which is unreliable: it sits inside the `Raw note:` blob the
agent is told to preserve and not invent from, so the agent sometimes follows
it and sometimes transcribes it verbatim into the cleaned note.

This work makes inline instructions reliable by **deterministically separating
them from note content in Rust before the prompt is built**, then injecting
them into their own prompt section — exactly the pattern the existing reparse
`feedback` channel already uses successfully.

## Scope

In scope:

- A new pure, unit-tested extraction module that splits a raw note into a
  `body` (content to clean) and an ordered list of `directives` (instructions),
  using a paragraph-bounded `@codex:` marker.
- Wiring the extraction into the parse-prompt builder so the agent cleans only
  the body and receives directives as a separate "Note instructions" section.
- One backup prompt rule.
- Updating `docs/parser-contract.md` to document directive extraction.

Out of scope:

- A dedicated capture-UI instructions field (rejected in favor of the inline
  marker; would require a DB/domain change).
- Any database, domain model, settings, or frontend change. The raw note is
  stored and displayed unchanged; directives are derived at parse time.
- A configurable marker string — the marker is fixed at `@codex:` for v1.
- Multi-line directives with preserved internal structure (lists, code) —
  continuation lines are joined into a single instruction string.

## The Marker and Bounding Rule

A **directive block**:

- **Starts** at a line whose first non-whitespace characters are `@codex:`
  (ASCII case-insensitive; leading whitespace before `@codex:` is allowed).
- **Continues** across the following lines until the first of:
  - a **blank line** (empty or whitespace-only), or
  - the **start of another `@codex:` line** (stacked directives stay separate,
    never glued together), or
  - the **end of the note**.
- **Directive text** = the marker line's remainder after `@codex:`, plus each
  continuation line, each trimmed, joined with a single space into one
  instruction string.
- A directive whose text is empty after trimming (a bare `@codex:`) is ignored.

Everything not inside a directive block is the **body**: the note content the
agent cleans. Directive blocks are removed from the body verbatim; the body's
leading and trailing blank lines are trimmed.

### Accepted tradeoff

Because a block runs until a blank line, the author **must leave a blank line
before resuming note content** after a directive; otherwise that content is
absorbed into the directive. This is the deliberate cost of paragraph
bounding and is documented in the parser contract.

### Worked example

Raw note:

```text
@codex: rewrite as a checklist,
tag urgent, and pull the version
from repo-b

Met with Bob about the rollout.
@codex: keep it terse
```

Extraction:

- `directives` = `["rewrite as a checklist, tag urgent, and pull the version from repo-b", "keep it terse"]`
- `body` = `"Met with Bob about the rollout."`

## Architecture

Mirrors the existing `feedback` channel: instructions live in their own clearly
labeled prompt section, never inside the cleaned content.

### New module: `src-tauri/src/parser/directives.rs`

Pure and framework-free:

```text
pub struct ExtractedNote {
    pub body: String,
    pub directives: Vec<String>,
}

pub fn extract_directives(raw_note: &str) -> ExtractedNote
```

Registered in `src-tauri/src/parser/mod.rs`.

### Prompt builder: `src-tauri/src/parser/prompt.rs`

`build_parse_prompt_with_context` calls `extract_directives` on the incoming
raw note, then:

- Uses `extracted.body` (not the original raw note) for the `Raw note:`
  section, so directive lines are never presented as content.
- When `extracted.directives` is non-empty, renders a new section before the
  raw note:

  ```text
  Note instructions (the author's directives for how to process this note — follow them; never copy the @codex: text into the cleaned note):
  - <directive 1>
  - <directive 2>
  ```

- When there are no directives, no section is rendered and behavior is
  unchanged (body == original note).

The existing `feedback` (reparse) section is unchanged and continues to render
independently, so note directives and reparse feedback coexist.

Because extraction happens inside the prompt builder, the `CodexParserProvider`
call sites are unchanged — they still pass the raw note.

### New backup rule

Add one rule to the rules array (defense-in-depth for a marker that somehow
appears mid-line, which whole-line extraction will not catch):

```text
Treat any line beginning with @codex: as an instruction to you, not note content: follow it and never include the @codex: text in the cleaned output.
```

## Guardrails

- Directives guide **how to clean, organize, tag, and enrich** the note. They
  are **subordinate to the existing safety rules** and cannot override them:
  "Do not invent facts" and "Do not claim repo facts unless you inspected the
  linked context or the raw note explicitly states them" still bind. So
  `@codex: invent a version number` will not fabricate; `@codex: pull the
  version from repo-b` will (it inspects the linked repo, per the read-only
  multi-repo inspection already shipped).
- The raw note is never modified (existing Trust Rule): the `@codex:` lines
  remain in the stored raw text and are visible under the note's "Raw" toggle.

## Data Flow

```text
note.raw_text
  -> CodexParserProvider.parse_output_with_feedback(raw_text, feedback)
     -> build_parse_prompt_with_context(raw_text, feedback, linked_paths)
        -> extract_directives(raw_text) => { body, directives }
        -> prompt = rules
                  + linked_context (if any)
                  + Note instructions section (if directives non-empty)
                  + "Raw note:\n" + body
                  + feedback section (if any)
  -> codex exec -> JSON -> validate -> apply
```

## Error Handling

- A note with no `@codex:` line extracts to `{ body: <whole note>, directives: [] }`
  and renders the prompt exactly as before — zero behavior change for existing
  notes.
- A note that is only `@codex:` directive lines extracts to an empty body. The
  agent receives the instructions and an empty `Raw note:` section and does its
  best within the no-invent guardrail. Not an error.
- A bare `@codex:` with no text is ignored (removed from body, not added as a
  directive).
- Extraction is total (never panics, never errors); it always returns an
  `ExtractedNote`.

## Testing

Extraction unit tests (`directives.rs`):

- No directive: body equals the input (modulo end trim), directives empty.
- Single one-line directive: extracted; body excludes it.
- Multi-line (paragraph) directive: continuation lines joined with spaces into
  one directive; stops at the blank line.
- Two stacked `@codex:` lines with no blank line between them: two separate
  directives, not one glued directive.
- Case-insensitive marker (`@CODEX:`, `@Codex:`) and leading whitespace before
  the marker are recognized.
- Bare `@codex:` with empty text is ignored.
- Content immediately after a directive with no blank line is absorbed into the
  directive (documents the accepted tradeoff).
- Directive-only note: body empty, directive captured.

Prompt tests (`prompt.rs`):

- With directives: prompt contains the "Note instructions" header and each
  directive as a `- ` line, and the `Raw note:` section contains the body
  without the directive text.
- Without directives: no "Note instructions" header; `Raw note:` contains the
  full note (unchanged behavior).
- The backup rule string is present in the rules array.
- The two existing inspection rules and the reparse-feedback behavior are
  unchanged.

## Documentation

Update `docs/parser-contract.md` with a "Note Directives" subsection: the
`@codex:` marker, the paragraph-bounded rule, the blank-line tradeoff, that
directives are extracted deterministically and never written into cleaned
text, that the raw note is preserved, and that directives are subordinate to
the no-invent/no-unverified-repo-claim rules.

## Implementation Order

1. Add the pure `directives.rs` extraction module with its unit tests; register
   it in `parser/mod.rs`.
2. Wire `extract_directives` into `build_parse_prompt_with_context`: render the
   "Note instructions" section, clean only the body, add the backup rule;
   update prompt tests.
3. Update `docs/parser-contract.md`.
