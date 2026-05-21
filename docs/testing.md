# Testing And Verification

Use this file to choose verification for changes in this repo. Prefer targeted checks while developing, then run the broader set before handing off meaningful behavior changes.

## Common Commands

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
```

Use `scripts\cargo-test.cmd` for Rust tests from normal PowerShell. It loads the Visual Studio developer environment before running `cargo test` inside `src-tauri`.

For native app development:

```powershell
npm run tauri dev
```

For a debug package build:

```powershell
cmd.exe /c "call ""C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 >nul && npm run tauri build -- --debug"
```

## Frontend Checks

Run these after Svelte, TypeScript, store, API wrapper, or theme changes:

```powershell
npm test
npm run check
npm run build
```

Important frontend tests:

- `src/lib/stores/inbox.test.ts`: filters and workflow store behavior with injected fake APIs.
- `src/lib/components/QuickCapturePanel.test.ts`: quick-capture keyboard behavior.
- `src/lib/components/NoteDetail.test.ts`: reparse feedback and delete events.
- `src/lib/theme/theme.test.ts`: theme token to CSS variable mapping.
- `src/lib/scaffold.test.ts`: scaffold sanity check.

Testing conventions:

- Prefer fake/injected APIs for store tests.
- Use Testing Library Svelte for component behavior.
- Keep Tauri command invocation centralized in `src/lib/api.ts`.
- Preserve the browser fallback path so `npm run build` and browser smoke checks can run without native Tauri.

## Rust Checks

Run this after Rust command, service, repository, parser, database, or windowing changes:

```powershell
scripts\cargo-test.cmd
```

Rust tests are inline in `src-tauri/src/**`. Current coverage includes:

- database migration shape
- repository note/tag/action/parse-job behavior
- FTS search over raw and cleaned text
- parser result deserialization
- parser schema validation
- Codex command builder behavior
- prompt text behavior
- parser result application trust rules
- capture service behavior
- parse queue success, failure, retry, and parse-run recording
- command DTO serialization
- settings validation
- hotkey validation
- quick-capture positioning helpers

## Parser Checks

For parser-specific work, run targeted Rust tests first, then the full Rust command:

```powershell
scripts\cargo-test.cmd parser
scripts\cargo-test.cmd parse_queue
scripts\cargo-test.cmd
```

Do not require live `codex exec` for unit tests. Provider tests should exercise command construction, schema validation, process handling abstractions, and typed errors without spending real parser work unless a test is explicitly marked for manual integration.

## Native Windows Manual Checks

Run manual checks when touching `src-tauri/src/windowing/`, Tauri command registration, settings that affect hotkeys/tray behavior, or quick-capture UI integration.

Start the app:

```powershell
npm run tauri dev
```

Check:

- Tray icon appears in the Windows notification area.
- Tray menu `Open` shows the main window.
- Tray menu `Quick Note` opens quick capture.
- Default global hotkey `Ctrl+Shift+Space` opens quick capture from another focused app.
- Quick capture appears bottom-right on the active monitor.
- Quick-capture textarea is focused when shown.
- `Enter` saves and closes.
- `Shift+Enter` inserts a newline.
- `Esc` closes and preserves the draft while the process remains alive.
- Saved note appears in the inbox.
- Parser failure is visible and raw note text remains intact.
- Real parsing works when the local Codex CLI is installed and logged in.

## Final Handoff Checks

For meaningful feature or bugfix work, run:

```powershell
npm test
npm run check
npm run build
scripts\cargo-test.cmd
git status --short --branch
```

For docs-only work, at minimum run:

```powershell
git diff --check
git status --short --branch
```

