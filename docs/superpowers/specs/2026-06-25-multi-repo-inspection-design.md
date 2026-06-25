# Multi-Repo Inspection for the Note Parser — Design

## Purpose

The note-enhancement parser (Codex provider) can already be given a list of
linked workspace repos/directories, and the prompt lists them for "optional
inspection." But two things bias the agent toward only the first repo and
leave its access non-deterministic:

1. No `--sandbox` flag is passed to `codex exec`, so the access the agent has
   to the linked repos depends on whatever the user's `~/.codex/config.toml`
   default sandbox is — and could even allow the parser to write into the
   user's real repositories.
2. The working directory is set to the first linked path, and the prompt
   frames the repos as a flat list without making clear each one is an
   absolute root the agent can read directly. The agent gravitates to the
   cwd repo.

This work makes multi-repo inspection deterministic and read-only: the agent
can read across **all** linked repos by absolute path, can never modify them,
and treats no single repo as primary.

## Scope

In scope:

- Pass `-s read-only` to every `codex exec` parse invocation.
- Refine the linked-context prompt block so the agent treats every listed
  path as an absolute, independently-readable root with no primary repo.
- Update the affected unit tests (command args, prompt wording).
- A manual live verification across two real repos.

Out of scope:

- Per-note (vs. global) repo linking — linking stays a global setting.
- Letting the parser write to / build / test inside repos (the "full working
  access" model was explicitly rejected in favor of read-only).
- Changing how linked paths are configured, validated, or normalized in
  settings.
- `--add-dir`, common-parent cwd computation, or any per-directory grant —
  unnecessary once the sandbox is `read-only` (full-filesystem read).
- Frontend changes.

## Background: how Codex sandboxing applies here

`codex exec --sandbox <mode>` governs **model-generated shell commands only**
(confirmed from `codex exec --help`). The three modes:

- `read-only` — the agent may read the entire filesystem; it cannot write or
  use the network.
- `workspace-write` — broad read, write restricted to the workspace +
  `--add-dir` roots.
- `danger-full-access` — no restrictions.

Two consequences make `read-only` the right and minimal choice:

- Codex writes its own `-o` output file and reads the `--output-schema` file
  itself (not as model shell commands), so `read-only` does **not** break
  parser output.
- `read-only` grants full-filesystem **read**, so every linked repo path
  already listed in the prompt becomes readable with no extra per-directory
  flags.

Setting `read-only` explicitly also removes the current latent risk: with no
`--sandbox` flag, a user whose config defaults to `workspace-write` or
`danger-full-access` could have the parser modify files in their repos.

## Approach

Chosen: **`-s read-only` + a prompt refinement.** Rejected alternatives:

- `-c 'sandbox_permissions=["disk-full-read-access"]'` — verbose and tied to a
  specific config knob; `read-only` already grants full read. Kept only as a
  fallback if live testing shows `read-only`'s read scope is narrower than
  documented.
- Common-parent cwd — fragile: repos on different Windows drives have no
  common ancestor, or it resolves to a drive root and exposes everything.
- `--add-dir` per repo — grants *write* access, which is the rejected access
  model.

## Components and Changes

### 1. Codex command — `src-tauri/src/parser/codex_provider.rs`

`CodexCommandBuilder::build()` adds `-s read-only` to the args, placed after
`--skip-git-repo-check` and before `--output-schema`. Applied to every parse
(the parser never needs to write), so no conditional logic.

Resulting args:

```text
exec
--ephemeral
--skip-git-repo-check
-s
read-only
--output-schema
<schema>
-o
<output>
-
```

cwd handling is unchanged: it remains the first existing linked directory as
a harmless starting anchor. Under `read-only` full-read, cwd is only a
starting point; the prompt removes the repo-#1 bias.

### 2. Prompt — `src-tauri/src/parser/prompt.rs`

Keep the existing inspection rules:

- "If linked repo or directory context is provided, inspect it only when it
  is useful for understanding the note or checking a task."
- "Do not claim repo facts unless you inspected the linked context or the raw
  note explicitly states them."

Change the linked-context block so it (a) states the inspection is read-only
and (b) frames each path as an absolute root with no primary. New block when
paths are present:

```text
Linked repos/directories available for optional read-only inspection. Each is
an absolute directory root you may read from directly by its full path; they
are equally available and none is the primary repo:
- <path>
- <path>
```

When no paths are linked, the block stays empty (unchanged behavior).

## Data Flow

Unchanged end to end: global `settings.linked_workspace_paths` →
`ParserProviderConfig` → `CodexParserProvider::linked_workspace_paths` →
filtered to existing dirs (`active_linked_workspace_paths`) → listed in the
prompt and used to pick the cwd anchor. The only behavioral change is the new
`-s read-only` arg and the reworded prompt block.

## Error Handling

- `-s read-only` cannot break output writing (Codex writes `-o` itself), so no
  new failure mode is introduced.
- Non-existent linked paths are still filtered out before they reach the
  prompt or cwd (existing `active_linked_workspace_paths`).
- If the model attempts a write against a repo under `read-only`, Codex denies
  the command; that is the intended guarantee, not an error to handle.

## Testing

Unit (`codex_provider.rs`):

- `builds_codex_exec_command_with_schema_and_output_file` updated to expect
  `-s read-only` in the exact args, in the specified position.

Unit (`prompt.rs`):

- `build_parse_prompt_with_context_lists_linked_workspaces` updated to assert
  the new read-only / absolute-root wording and that each path is still
  listed.
- Existing `build_parse_prompt_includes_parser_rules_and_raw_note` keeps the
  two retained inspection rules.

Manual live verification (the one assumption to prove — that Codex
`read-only` permits cross-tree reads on Windows, which lacks OS-level sandbox
enforcement):

- Link two real repos in settings.
- Write a note that references a fact only discoverable in the **second**
  repo.
- Parse the note and confirm the cleaned output reflects a fact read from the
  second repo (i.e., the agent inspected a non-cwd repo), and that neither
  repo was modified.
- If `read-only` blocks the cross-tree read, fall back to adding
  `-c sandbox_permissions=["disk-full-read-access"]` and re-verify.

## Implementation Order

1. Update the command builder to emit `-s read-only`; update its args test.
2. Refine the prompt linked-context block; update the prompt test.
3. Run the Rust test suite; then perform the manual two-repo live check.
