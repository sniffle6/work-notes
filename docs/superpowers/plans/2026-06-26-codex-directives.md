# Inline `@codex:` Directives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a note author embed `@codex:` instructions in a raw note that the parser reliably follows during cleaning instead of transcribing.

**Architecture:** A new pure module deterministically splits a raw note into a `body` (content to clean) and an ordered list of `directives`, using a paragraph-bounded `@codex:` marker. The provider extracts before building the prompt and passes `body` + `directives` into the prompt builder, which renders the directives in their own "Note instructions" section (mirroring the existing reparse-feedback section). All marker knowledge stays inside the extraction module.

**Tech Stack:** Rust (Tauri backend), `cargo test` with inline `#[cfg(test)]` modules.

## Global Constraints

- The marker is exactly `@codex:` (ASCII case-insensitive), recognized only at the start of a line after optional leading whitespace.
- A directive block is paragraph-bounded: it ends at the first blank line (empty or whitespace-only), the start of the next `@codex:` line, or end of note.
- Directive text = the marker line's remainder plus continuation lines, each trimmed, joined with a single space. A directive whose text is empty after trimming is ignored.
- The body is the note with directive blocks removed; its leading and trailing whitespace is trimmed.
- Directives are never written into cleaned text, and the raw note is never modified.
- Marker knowledge lives only in `directives.rs`; the prompt builder takes directives as a `&[String]` and does not parse the note.
- No database, domain, settings, or frontend change.

---

### Task 1: Pure `directives.rs` extraction module

**Files:**
- Create: `src-tauri/src/parser/directives.rs`
- Modify: `src-tauri/src/parser/mod.rs:1-21` (register and re-export the module)
- Test: inline `#[cfg(test)]` module in `src-tauri/src/parser/directives.rs`

**Interfaces:**
- Produces:
  - `pub struct ExtractedNote { pub body: String, pub directives: Vec<String> }`
  - `pub fn extract_directives(raw_note: &str) -> ExtractedNote`

- [ ] **Step 1: Create the module with types, tests, and a stub**

Create `src-tauri/src/parser/directives.rs`:

```rust
/// A raw note split into the content to clean and the author's `@codex:`
/// directives, in the order they appear.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedNote {
    pub body: String,
    pub directives: Vec<String>,
}

/// Split a raw note into its body (content to clean) and an ordered list of
/// `@codex:` directives. See `docs/parser-contract.md` for the bounding rules.
pub fn extract_directives(_raw_note: &str) -> ExtractedNote {
    ExtractedNote::default()
}

#[cfg(test)]
mod tests {
    use super::extract_directives;

    #[test]
    fn no_directive_returns_body_unchanged() {
        let note = "Met with Bob.\n\nShip date moved to Friday.";
        let extracted = extract_directives(note);
        assert_eq!(extracted.body, "Met with Bob.\n\nShip date moved to Friday.");
        assert!(extracted.directives.is_empty());
    }

    #[test]
    fn single_line_directive_is_extracted_and_removed_from_body() {
        let note = "@codex: tag urgent\n\nMet with Bob about the rollout.";
        let extracted = extract_directives(note);
        assert_eq!(extracted.directives, vec!["tag urgent".to_string()]);
        assert_eq!(extracted.body, "Met with Bob about the rollout.");
    }

    #[test]
    fn multi_line_directive_joins_continuation_lines_with_spaces() {
        let note = "@codex: rewrite as a checklist,\ntag urgent, and pull the version\nfrom repo-b\n\nMet with Bob about the rollout.";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["rewrite as a checklist, tag urgent, and pull the version from repo-b".to_string()]
        );
        assert_eq!(extracted.body, "Met with Bob about the rollout.");
    }

    #[test]
    fn stacked_directive_lines_stay_separate() {
        let note = "@codex: tag urgent\n@codex: keep it terse\n\nMet with Bob.";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["tag urgent".to_string(), "keep it terse".to_string()]
        );
        assert_eq!(extracted.body, "Met with Bob.");
    }

    #[test]
    fn marker_is_case_insensitive_and_allows_leading_whitespace() {
        let note = "   @CoDeX:   tag urgent  \n\nBody.";
        let extracted = extract_directives(note);
        assert_eq!(extracted.directives, vec!["tag urgent".to_string()]);
        assert_eq!(extracted.body, "Body.");
    }

    #[test]
    fn bare_marker_with_no_text_is_ignored() {
        let note = "@codex:\n\nMet with Bob.";
        let extracted = extract_directives(note);
        assert!(extracted.directives.is_empty());
        assert_eq!(extracted.body, "Met with Bob.");
    }

    #[test]
    fn content_without_a_blank_line_is_absorbed_into_the_directive() {
        let note = "@codex: tag urgent\nMet with Bob.";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["tag urgent Met with Bob.".to_string()]
        );
        assert_eq!(extracted.body, "");
    }

    #[test]
    fn directive_only_note_yields_empty_body() {
        let note = "@codex: summarize the standup as bullets";
        let extracted = extract_directives(note);
        assert_eq!(
            extracted.directives,
            vec!["summarize the standup as bullets".to_string()]
        );
        assert_eq!(extracted.body, "");
    }

    #[test]
    fn marker_not_at_line_start_is_left_in_the_body() {
        let note = "Discuss the @codex: integration with Bob.";
        let extracted = extract_directives(note);
        assert!(extracted.directives.is_empty());
        assert_eq!(extracted.body, "Discuss the @codex: integration with Bob.");
    }
}
```

Register the module in `src-tauri/src/parser/mod.rs`. After the existing `pub mod codex_provider;` line, add `pub mod directives;` (keep alphabetical: it goes between `codex_provider` and `fake_provider`). After the `pub use fake_provider::FakeParserProvider;` line, add:

```rust
pub use directives::{extract_directives, ExtractedNote};
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml directives::`
Expected: FAIL — the stub returns an empty `ExtractedNote`, so the body/directive assertions fail (e.g. `no_directive_returns_body_unchanged` sees an empty body).

- [ ] **Step 3: Implement the extraction**

Replace the stub `extract_directives` (and add the `MARKER` constant and the private `directive_start` helper) in `src-tauri/src/parser/directives.rs`. Add the constant just below the `ExtractedNote` struct:

```rust
const MARKER: &str = "@codex:";
```

Then replace the stub function body and add the helper:

```rust
/// Split a raw note into its body (content to clean) and an ordered list of
/// `@codex:` directives. See `docs/parser-contract.md` for the bounding rules.
pub fn extract_directives(raw_note: &str) -> ExtractedNote {
    let mut directives = Vec::new();
    let mut body_lines: Vec<&str> = Vec::new();
    let mut lines = raw_note.lines().peekable();

    while let Some(line) = lines.next() {
        let Some(remainder) = directive_start(line) else {
            body_lines.push(line);
            continue;
        };

        let mut parts: Vec<&str> = Vec::new();
        let remainder = remainder.trim();
        if !remainder.is_empty() {
            parts.push(remainder);
        }
        while let Some(&next) = lines.peek() {
            if next.trim().is_empty() || directive_start(next).is_some() {
                break;
            }
            parts.push(next.trim());
            lines.next();
        }

        if !parts.is_empty() {
            directives.push(parts.join(" "));
        }
    }

    ExtractedNote {
        body: body_lines.join("\n").trim().to_string(),
        directives,
    }
}

/// If `line` starts a directive — the case-insensitive `@codex:` marker after
/// optional leading whitespace — return the text after the marker.
fn directive_start(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let head = trimmed.get(..MARKER.len())?;
    head.eq_ignore_ascii_case(MARKER)
        .then(|| &trimmed[MARKER.len()..])
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml directives::`
Expected: PASS — all 9 tests green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/parser/directives.rs src-tauri/src/parser/mod.rs
git commit -m "feat: extract @codex: directives from raw notes"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

### Task 2: Render directives in the parse prompt

**Files:**
- Modify: `src-tauri/src/parser/prompt.rs:1-66` (the three prompt builders and the rules array)
- Modify: `src-tauri/src/parser/codex_provider.rs:159-164` (update the existing call site to pass an empty directives slice so the crate still compiles — no behavior change yet)
- Test: inline `#[cfg(test)]` module in `src-tauri/src/parser/prompt.rs`

**Interfaces:**
- Produces: `build_parse_prompt_with_context(body: &str, feedback: Option<&str>, linked_workspace_paths: &[String], directives: &[String]) -> String`
- Consumes (call site, temporary): the existing `build_parse_prompt_with_context(raw_note, feedback, &prompt_workspace_paths)` becomes a 4-arg call passing `&[]` for directives. Task 3 replaces this with real extraction.

- [ ] **Step 1: Update the prompt tests for the new parameter and directive section**

In `src-tauri/src/parser/prompt.rs`, update the existing linked-workspaces test call to pass the new 4th argument, and add three new tests. In `build_parse_prompt_with_context_lists_linked_workspaces`, change the call from three arguments to:

```rust
        let prompt = build_parse_prompt_with_context(
            "Review this task against the local checkout.",
            None,
            &[
                "C:\\code\\product".to_string(),
                "D:\\scratch\\other".to_string(),
            ],
            &[],
        );
```

Then add these tests to the same `#[cfg(test)]` module (and add `build_parse_prompt` / `build_parse_prompt_with_context` to its `use super::{...}` import if not already present):

```rust
    #[test]
    fn build_parse_prompt_with_context_renders_note_instructions() {
        let prompt = build_parse_prompt_with_context(
            "Met with Bob about the rollout.",
            None,
            &[],
            &["tag urgent".to_string(), "keep it terse".to_string()],
        );

        assert!(prompt.contains(
            "Note instructions (the author's directives for how to process this note"
        ));
        assert!(prompt.contains("- tag urgent"));
        assert!(prompt.contains("- keep it terse"));
        assert!(prompt.contains("Raw note:\nMet with Bob about the rollout."));
    }

    #[test]
    fn build_parse_prompt_with_context_omits_note_instructions_when_empty() {
        let prompt = build_parse_prompt_with_context("Body only.", None, &[], &[]);

        assert!(!prompt.contains("Note instructions"));
        assert!(prompt.contains("Raw note:\nBody only."));
    }

    #[test]
    fn build_parse_prompt_includes_codex_directive_backup_rule() {
        let prompt = build_parse_prompt("Body.");

        assert!(prompt.contains(
            "Treat any line beginning with @codex: as an instruction to you"
        ));
    }
```

- [ ] **Step 2: Run the prompt tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml prompt::`
Expected: FAIL to compile — `build_parse_prompt_with_context` still takes three parameters, so the 4-arg test calls do not compile.

- [ ] **Step 3: Add the parameter, backup rule, and directive section**

In `src-tauri/src/parser/prompt.rs`, update the two wrappers to pass `&[]` for directives:

```rust
pub fn build_parse_prompt(raw_note: &str) -> String {
    build_parse_prompt_with_feedback(raw_note, None)
}

pub fn build_parse_prompt_with_feedback(raw_note: &str, feedback: Option<&str>) -> String {
    build_parse_prompt_with_context(raw_note, feedback, &[], &[])
}
```

Change the `build_parse_prompt_with_context` signature to take `body` and `directives`:

```rust
pub fn build_parse_prompt_with_context(
    body: &str,
    feedback: Option<&str>,
    linked_workspace_paths: &[String],
    directives: &[String],
) -> String {
```

Add the backup rule as the last entry of the `rules` array (after the `"Do not claim repo facts..."` line):

```rust
        "Treat any line beginning with @codex: as an instruction to you, not note content: follow it and never include the @codex: text in the cleaned output.",
```

Add the note-instructions block immediately after the `linked_context` block and before the `feedback` block:

```rust
    let note_instructions = if directives.is_empty() {
        String::new()
    } else {
        format!(
            "\nNote instructions (the author's directives for how to process this note — follow them; never copy the @codex: text into the cleaned note):\n{}\n",
            directives
                .iter()
                .map(|directive| format!("- {directive}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
```

Update the final `format!` to insert `note_instructions` and render `body`:

```rust
    format!(
        "{}\n{}{}\nRaw note:\n{}\n{}",
        rules.join("\n"),
        linked_context,
        note_instructions,
        body,
        feedback
    )
```

- [ ] **Step 4: Update the call site so the crate compiles**

In `src-tauri/src/parser/codex_provider.rs`, the existing call (currently `build_parse_prompt_with_context(raw_note, feedback, &prompt_workspace_paths)`) must pass the new argument. Change it to:

```rust
        run_codex_command(
            &command,
            &build_parse_prompt_with_context(raw_note, feedback, &prompt_workspace_paths, &[]),
            self.timeout,
            working_dir,
        )?;
```

This keeps behavior identical (empty directives render no section, and `raw_note` is still the body); Task 3 wires in real extraction.

- [ ] **Step 5: Run the prompt tests and confirm the crate builds**

Run: `cargo test --manifest-path src-tauri/Cargo.toml prompt::`
Expected: PASS — the three new tests and the updated linked-workspaces test pass; the crate compiles.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/parser/prompt.rs src-tauri/src/parser/codex_provider.rs
git commit -m "feat: render @codex: directives as a prompt section"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`

---

### Task 3: Wire extraction into the provider and document the contract

**Files:**
- Modify: `src-tauri/src/parser/codex_provider.rs:1-5` (import) and `:159-164` (call site)
- Modify: `docs/parser-contract.md`
- Test: full backend suite (the wiring is the composition of the two already-tested seams; it adds no branching)

**Interfaces:**
- Consumes: `extract_directives(raw_note) -> ExtractedNote { body, directives }` (Task 1) and `build_parse_prompt_with_context(body, feedback, linked_paths, directives)` (Task 2).

- [ ] **Step 1: Import the extractor**

In `src-tauri/src/parser/codex_provider.rs`, the top `use super::{...}` block currently imports `prompt::build_parse_prompt_with_context` among others. Add `directives::extract_directives` to that block, e.g.:

```rust
use super::{
    directives::extract_directives,
    prompt::build_parse_prompt_with_context,
    types::{ParserError, ParserOutput, ParserProvider, ParserResult},
    validate::validate_parser_json,
};
```

- [ ] **Step 2: Extract and pass body + directives at the call site**

Replace the temporary call from Task 2 (`build_parse_prompt_with_context(raw_note, feedback, &prompt_workspace_paths, &[])`) with real extraction. Just before the `run_codex_command(...)` call, add the extraction, then pass the parts:

```rust
        let extracted = extract_directives(raw_note);

        run_codex_command(
            &command,
            &build_parse_prompt_with_context(
                &extracted.body,
                feedback,
                &prompt_workspace_paths,
                &extracted.directives,
            ),
            self.timeout,
            working_dir,
        )?;
```

- [ ] **Step 3: Run the full backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — every backend test green, including Task 1 and Task 2 tests; the crate compiles with the new wiring.

- [ ] **Step 4: Document the contract**

In `docs/parser-contract.md`, add this section immediately after the `## Codex Invocation` section (before `## Output Shape`):

```markdown
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
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/parser/codex_provider.rs docs/parser-contract.md
git commit -m "feat: wire @codex: directive extraction into the parser"
```

End the commit message with the trailer:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`
