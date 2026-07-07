# Agent Guide

This repo is a Windows-only local desktop app named Work Notes. It is built with Tauri v2, Svelte/SvelteKit, TypeScript, Rust, SQLite, and local `codex exec` parsing.

## Start Here

- Run `git status --short --branch` before editing. Do not revert or overwrite changes you did not make.
- Read `README.md` for the product overview and `docs/development.md` for setup details.
- Use `docs/architecture.md` for the current module map.
- Use `docs/parser-contract.md` before touching parsing, retries, schema validation, or Codex CLI execution.
- Use `docs/testing.md` to choose the right verification commands.
- Before pushing a release tag, triggering a GitHub release build, or publishing a draft GitHub release, use `.codex/skills/work-notes-release-build` and add release notes. Use the skill's dry run mode when testing the release flow. Do not publish a Work Notes release with a placeholder or generic release body.
- Treat `docs/superpowers/plans/2026-05-20-work-notes-agent-team-implementation.md` as historical implementation context, not current task status.

## Product Invariants

- Raw note text is the source of truth and must save immediately.
- Parser output is derived data only: cleaned text, summary, tags, suggested actions, and parse history.
- Parser failure must not damage or replace raw note text.
- Suggested action items stay `suggested` until the user accepts, dismisses, or completes them.
- The app uses local `codex exec` through the user's Codex subscription. Do not add OpenAI API keys or OpenAI API billing paths.
- Quick capture is keyboard-first: `Enter` saves, `Shift+Enter` inserts a newline, and `Esc` closes while preserving the draft.
- Default UI style is Dark Compact. Components should use semantic theme CSS variables, not raw palette colors.

## Boundaries

- Frontend workflow sequencing belongs in `src/lib/stores/inbox.ts`.
- Tauri command calls belong in `src/lib/api.ts`.
- Svelte components should stay thin and emit UI events rather than owning persistence or parser behavior.
- Tauri command DTOs and command validation belong in `src-tauri/src/commands.rs`.
- Workflow behavior belongs in `src-tauri/src/services/`.
- SQLite reads and writes belong in `src-tauri/src/repositories/`.
- Parser prompt, schema validation, Codex process execution, and result application belong in `src-tauri/src/parser/`.
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

