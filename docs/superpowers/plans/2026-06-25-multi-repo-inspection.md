# Multi-Repo Inspection for the Note Parser — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the note parser inspect all linked repos read-only (not just the first) by passing `-s read-only` to `codex exec` and reframing the linked-context prompt block.

**Architecture:** The Rust Codex provider builds the `codex exec` argument list and the parse prompt. Adding `-s read-only` to the args grants full-filesystem read (so every linked repo path already listed in the prompt is readable) while forbidding writes. A prompt wording change tells the agent each listed path is an independently-readable absolute root with no primary repo. No data-flow, settings, or frontend changes.

**Tech Stack:** Rust (rusqlite/Tauri backend), `codex exec` CLI, `cargo test` with inline `#[cfg(test)]` modules.

## Global Constraints

- Sandbox flag value is exactly `-s read-only` (two separate args: `"-s"`, `"read-only"`), placed after `--skip-git-repo-check` and before `--output-schema`.
- `read-only` applies to **every** parse, unconditionally — the parser never writes, and the flag must not depend on whether repos are linked.
- Do not add `--add-dir`, common-parent cwd logic, or `sandbox_permissions` overrides — `read-only` alone provides full read. (`sandbox_permissions` is only a documented fallback for the manual verification step, not part of the code.)
- Keep the two existing inspection rules in the prompt rules array verbatim; only the `linked_context` block wording changes.
- cwd handling is unchanged (first existing linked directory remains the anchor).
- Platform note: the codex program is `codex.cmd` on Windows; tests assert argument strings only and do not invoke codex.

---

### Task 1: Add `-s read-only` to the codex exec command

**Files:**
- Modify: `src-tauri/src/parser/codex_provider.rs:57-71` (the `CodexCommandBuilder::build` method)
- Test: `src-tauri/src/parser/codex_provider.rs:439-460` (the `builds_codex_exec_command_with_schema_and_output_file` test, same file's `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `CodexCommandBuilder::new(program)`, `.schema_path(...)`, `.output_path(...)`, `.build() -> CodexCommandSpec` where `CodexCommandSpec { program: String, args: Vec<String> }`.
- Produces: the exact `args` vector that `run_codex_command` passes to the process. Order is asserted by the test and must match the implementation.

- [ ] **Step 1: Update the failing test to expect the new args**

In `src-tauri/src/parser/codex_provider.rs`, replace the `assert_eq!` arg vector in `builds_codex_exec_command_with_schema_and_output_file` (currently lines 447-459) with:

```rust
        assert_eq!(
            command.args,
            vec![
                "exec",
                "--ephemeral",
                "--skip-git-repo-check",
                "-s",
                "read-only",
                "--output-schema",
                "schemas/parse-note.schema.json",
                "-o",
                "parse-result.json",
                "-",
            ]
        );
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml builds_codex_exec_command_with_schema_and_output_file`
Expected: FAIL — left vector lacks `"-s", "read-only"` (assertion mismatch).

- [ ] **Step 3: Add the flag to the builder**

In `CodexCommandBuilder::build` (currently lines 57-71), insert `"-s"` and `"read-only"` after `"--skip-git-repo-check"`:

```rust
    pub fn build(self) -> CodexCommandSpec {
        CodexCommandSpec {
            program: self.program,
            args: vec![
                "exec".to_string(),
                "--ephemeral".to_string(),
                "--skip-git-repo-check".to_string(),
                "-s".to_string(),
                "read-only".to_string(),
                "--output-schema".to_string(),
                self.schema_path,
                "-o".to_string(),
                self.output_path,
                "-".to_string(),
            ],
        }
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml builds_codex_exec_command_with_schema_and_output_file`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/parser/codex_provider.rs
git commit -m "feat: run codex parser sandboxed read-only"
```

---

### Task 2: Reframe the linked-context prompt block for multi-repo read-only inspection

**Files:**
- Modify: `src-tauri/src/parser/prompt.rs:35-46` (the `linked_context` `format!` inside `build_parse_prompt_with_context`)
- Test: `src-tauri/src/parser/prompt.rs:120-134` (the `build_parse_prompt_with_context_lists_linked_workspaces` test)

**Interfaces:**
- Consumes: `build_parse_prompt_with_context(raw_note: &str, feedback: Option<&str>, linked_workspace_paths: &[String]) -> String`.
- Produces: a prompt string. When `linked_workspace_paths` is non-empty, it contains the new header text and one `- <path>` line per path. When empty, the linked block is absent (unchanged).

- [ ] **Step 1: Update the test to assert the new wording**

In `src-tauri/src/parser/prompt.rs`, replace the assertions in `build_parse_prompt_with_context_lists_linked_workspaces` (currently lines 131-133) with:

```rust
        assert!(prompt.contains("available for optional read-only inspection"));
        assert!(prompt.contains("none is the primary repo"));
        assert!(prompt.contains("- C:\\code\\product"));
        assert!(prompt.contains("- D:\\scratch\\other"));
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml build_parse_prompt_with_context_lists_linked_workspaces`
Expected: FAIL — the current header is "Linked repos/directories available for optional inspection:", so `"available for optional read-only inspection"` and `"none is the primary repo"` are not present.

- [ ] **Step 3: Update the linked_context block**

In `build_parse_prompt_with_context`, replace the `format!` header string in the `else` branch (currently the string on line 39) so the block reads:

```rust
    let linked_context = if linked_workspace_paths.is_empty() {
        String::new()
    } else {
        format!(
            "\nLinked repos/directories available for optional read-only inspection. Each is an absolute directory root you may read from directly by its full path; they are equally available and none is the primary repo:\n{}\n",
            linked_workspace_paths
                .iter()
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
```

Leave the `rules` array (lines 14-33) unchanged — the two inspection rules stay verbatim.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml build_parse_prompt_with_context_lists_linked_workspaces`
Expected: PASS.

- [ ] **Step 5: Confirm the unchanged-rules test still passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml build_parse_prompt_includes_parser_rules_and_raw_note`
Expected: PASS (the rules array was not modified).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/parser/prompt.rs
git commit -m "feat: frame linked repos as read-only absolute roots in parse prompt"
```

---

### Task 3: Verify the full parser suite and the live multi-repo read

**Files:**
- None modified — verification only.

**Interfaces:**
- Consumes: the changes from Tasks 1 and 2.
- Produces: evidence that the parser module tests pass and that a real parse reads a non-cwd repo without modifying any repo.

- [ ] **Step 1: Run the parser module test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::`
Expected: PASS — all parser tests green, including the two updated above.

- [ ] **Step 2: Manual live two-repo check**

This proves the one assumption the unit tests cannot: that Codex `read-only` permits cross-tree reads on Windows (which lacks OS-level sandbox enforcement).

1. In the running app's Settings, link two real existing repositories as workspace paths (e.g. `C:\code\repo-a` and `C:\code\repo-b`).
2. Write a note whose meaning depends on a fact discoverable only in the **second** repo (e.g. "Confirm the crate version in repo-b's Cargo.toml").
3. Trigger a parse of that note.
4. Confirm the cleaned output reflects a fact read from the second repo (the agent inspected a non-cwd repo).
5. Run `git status` in **both** repos and confirm neither was modified.

Expected: the parse reflects repo-b content; both repos show a clean `git status`.

- [ ] **Step 3 (only if Step 2 fails the cross-tree read): apply the documented fallback**

If the agent cannot read the second repo under `read-only`, add the full-read override to the command builder args in `src-tauri/src/parser/codex_provider.rs` `build` — insert after `"read-only"`:

```rust
                "-c".to_string(),
                "sandbox_permissions=[\"disk-full-read-access\"]".to_string(),
```

Update the `builds_codex_exec_command_with_schema_and_output_file` arg vector to include `"-c"` and `"sandbox_permissions=[\"disk-full-read-access\"]"` in the same position, re-run `cargo test --manifest-path src-tauri/Cargo.toml builds_codex_exec_command_with_schema_and_output_file` (expect PASS), repeat Step 2, then commit:

```bash
git add src-tauri/src/parser/codex_provider.rs
git commit -m "fix: grant codex parser full read access for cross-repo inspection"
```

If Step 2 passed, skip this step entirely.
