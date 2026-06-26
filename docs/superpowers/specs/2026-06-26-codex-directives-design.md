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
- Wiring the extraction into the parse path: the provider extracts, then passes
  the body and directives to the prompt builder, so the agent cleans only the
  body and receives directives as a separate "Note instructions" section.
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

The prompt builder renders a prompt from parts it is handed; it does not parse
the note. `build_parse_prompt_with_context` gains an explicit `directives`
parameter:

```text
build_parse_prompt_with_context(body: &str, feedback: Option<&str>, linked_workspace_paths: &[String], directives: &[String]) -> String
```

- `body` is rendered verbatim under `Raw note:`.
- When `directives` is non-empty, a new section is rendered before the raw
  note:

  ```text
  Note instructions (the author's directives for how to process this note — follow them; never copy the @codex: text into the cleaned note):
  - <directive 1>
  - <directive 2>
  ```

- When `directives` is empty, no section is rendered and behavior is unchanged.

The existing `feedback` (reparse) section is unchanged and continues to render
independently, so note directives and reparse feedback coexist.

The thin convenience wrappers `build_parse_prompt(raw)` and
`build_parse_prompt_with_feedback(raw, feedback)` pass an empty directives
slice (`&[]`), so their existing behavior and tests are unchanged.

### Orchestration: `src-tauri/src/parser/codex_provider.rs`

The provider is the seam where note splitting and prompt rendering meet. In
`parse_output_with_feedback`, before building the prompt, it extracts once and
passes the orthogonal parts to the builder:

```text
let extracted = extract_directives(raw_note);
build_parse_prompt_with_context(&extracted.body, feedback, &prompt_workspace_paths, &extracted.directives)
```

This keeps all `@codex:` marker knowledge inside `directives.rs`: the prompt
builder only ever sees directive-shaped inputs, never raw marker syntax, so
changing the marker later touches only the extraction module and its tests.

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
     -> extract_directives(raw_text) => { body, directives }
     -> build_parse_prompt_with_context(body, feedback, linked_paths, directives)
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

- Given a non-empty `directives` slice: the prompt contains the "Note
  instructions" header and each directive as a `- ` line, and the `Raw note:`
  section contains the passed `body`.
- Given an empty `directives` slice: no "Note instructions" header; the
  `Raw note:` section contains the passed text unchanged (existing behavior).
- The backup rule string is present in the rules array.
- The two existing inspection rules and the reparse-feedback behavior are
  unchanged.

The provider's one-line glue (extract once, pass `body` and `directives` to the
builder) is left to the composition of the two tested seams above; it adds no
branching of its own.

## Documentation

Update `docs/parser-contract.md` with a "Note Directives" subsection: the
`@codex:` marker, the paragraph-bounded rule, the blank-line tradeoff, that
directives are extracted deterministically and never written into cleaned
text, that the raw note is preserved, and that directives are subordinate to
the no-invent/no-unverified-repo-claim rules.

## Implementation Order

1. Add the pure `directives.rs` extraction module with its unit tests; register
   it in `parser/mod.rs`.
2. Add the `directives: &[String]` parameter to
   `build_parse_prompt_with_context`, render the "Note instructions" section
   from it, add the backup rule, and have the `build_parse_prompt` /
   `build_parse_prompt_with_feedback` wrappers pass `&[]`; update prompt tests.
3. In `CodexParserProvider::parse_output_with_feedback`, call
   `extract_directives` on the raw note and pass `body` and `directives` into
   the prompt builder.
4. Update `docs/parser-contract.md`.
