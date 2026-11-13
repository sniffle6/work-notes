# Agent Guide

This repo is a Windows-only local desktop app named Work Notes. It is built with Tauri v2, Svelte/SvelteKit, TypeScript, Rust, SQLite, and local `codex exec` parsing.

## Start Here

- Run `git status --short --branch` before editing. Do not revert or overwrite changes you did not make.
- Read `README.md` for the product overview and `docs/development.md` for setup details.
- Use `docs/architecture.md` for the current module map.
- Use `docs/parser-contract.md` before touching parsing, retries, schema validation, or Codex CLI execution.
- Use `docs/jira.md` before touching Jira access, the Jira agent tool contract, or proposal approval.
- Use `docs/testing.md` to choose the right verification commands.
- Before pushing a release tag, triggering a GitHub release build, or publishing a draft GitHub release, use `.codex/skills/work-notes-release-build` and add release notes. Use the skill's dry run mode when testing the release flow. Do not publish a Work Notes release with a placeholder or generic release body.
- Treat `docs/superpowers/plans/2026-05-20-work-notes-agent-team-implementation.md` as historical implementation context, not current task status.

## Product Invariants

- Raw note text is the source of truth and must save immediately.
- Parser output is derived data only: cleaned text, summary, tags, suggested actions, and parse history.
- Parser failure must not damage or replace raw note text.
- Suggested action items stay `suggested` until the user accepts, dismisses, or completes them.
- Dropping a follow-up stops tracking it without recording it as done. Never mark work done to clear it from a list.
- A theme the theme agent builds is a preview until the user keeps it. Do not save or select one on the agent's say-so.
- The app uses local `codex exec` through the user's Codex subscription. Do not add OpenAI API keys or OpenAI API billing paths.
- Quick capture is keyboard-first: `Enter` saves, `Shift+Enter` inserts a newline, and `Esc` closes while preserving the draft.
- Default UI style is Dark Compact. Components should use semantic theme CSS variables, not raw palette colors.
- The UI is quiet and text-first: regular-weight type, hairline rules instead of boxes, and colour only for state and the next action. Use the shared tokens declared in `src/routes/+page.svelte` (`--font-ui`, `--font-display`, `--font-mono`, `--weight-*`, `--rule`, `--radius-*`) instead of hard-coded weights, font stacks, or uppercase letter-spaced labels.
- A toolbar or header stays on one line. Drop labels to icons with `aria-label` and `title` before letting controls wrap.
- The Jira agent never changes Jira on its own. Every change is a stored proposal that runs only after the user approves it. The agent may change the app's own Jira view (ticket list, open ticket, new-ticket draft) without asking. A draft becomes a ticket only when the user creates it.
- The Jira API token comes from the environment. Do not store it in SQLite or include it in an agent prompt.

## Boundaries

- Frontend workflow sequencing belongs in `src/lib/stores/inbox.ts`, and in `src/lib/stores/jira.ts` for the Jira view.
- Tauri command calls belong in `src/lib/api.ts`.
- Svelte components should stay thin and emit UI events rather than owning persistence or parser behavior.
- Tauri command DTOs and command validation belong in `src-tauri/src/commands.rs`, and in `src-tauri/src/commands/jira.rs` for Jira.
- Workflow behavior belongs in `src-tauri/src/services/`.
- SQLite reads and writes belong in `src-tauri/src/repositories/`.
- Parser prompt, schema validation, Codex process execution, and result application belong in `src-tauri/src/parser/`.
- Jira HTTP access, document conversion, and the agent tool contract belong in `src-tauri/src/jira/`. Network I/O stays in `jira/transport.rs`.
- The theme agent's prompt, output checks, and contrast rules belong in `src-tauri/src/theme.rs`. Turning saved themes into theme definitions belongs in `src/lib/theme/custom.ts`.
- Tray, hotkey, main window, and quick-capture positioning belong in `src-tauri/src/windowing/`.

## Commands

```powershell
npm install
npm run tauri dev
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```

Use `scripts\cargo-test.cmd` for Rust tests from normal PowerShell. It loads the Visual Studio developer environment before running Cargo.

For a debug package build:

```powershell
cmd.exe /c "call ""C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 >nul && npm run tauri build -- --debug"
```

## Avoid

- Do not scan or edit generated/heavy folders unless the task explicitly needs them: `node_modules`, `build`, `.svelte-kit`, `src-tauri/target`.
- Do not put SQLite details in Svelte components.
- Do not run `codex` directly from frontend code.
- Do not add background parser concurrency without revisiting the queue and SQLite locking assumptions.
- Do not silently accept parser-inferred owners, dates, or obligations.
- Do not give the Jira agent a way to write to Jira that skips proposal approval.

