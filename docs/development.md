# Development Guide

## Prerequisites

- Windows.
- Node.js and npm.
- Rust stable.
- Visual Studio 2022 Build Tools or Visual Studio 2022 with the MSVC toolchain.
- Codex CLI installed and logged in if real parsing should run.

No OpenAI API key is required for this app. Parsing is designed around the local Codex CLI and the user's Codex subscription.

## Common Commands

```powershell
npm install
npm run tauri dev
npm test
npm run build
scripts\cargo-test.cmd
```

`scripts\cargo-test.cmd` should be used instead of plain `cargo test` from a normal PowerShell session. It calls Visual Studio's `VsDevCmd.bat` first so native Windows linker paths are available.

If `npm run tauri dev` is launched from a normal PowerShell session and MSVC linker paths are missing, start it through the Visual Studio developer environment:

```powershell
cmd.exe /c "call ""C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 >nul && npm run tauri dev"
```

Do not open `src-tauri\target\debug\work-notes.exe` directly as the normal development launch path. A dev-mode Tauri binary expects the Vite dev server at `localhost:1420`; without that server it can show a `localhost refused to connect` page. Use `npm run tauri dev` for development, or build/install a package when testing the packaged app.

For a debug installer build:

```powershell
cmd.exe /c "call ""C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 >nul && npm run tauri build -- --debug"
```

## Local Data

SQLite is opened from:

```text
directories::ProjectDirs::from("com", "aweber", "Work Notes").data_dir()\work-notes.sqlite3
```

On this Windows setup that resolves under:

```text
%APPDATA%\aweber\Work Notes\data\work-notes.sqlite3
```

The database owns notes, tags, action items, parse jobs, parse runs, settings, and the FTS5 index.

## Codex Parser

The parser provider invokes:

```powershell
codex exec `
  --ephemeral `
  --skip-git-repo-check `
  --model gpt-5.6-luna `
  --output-schema schemas/parse-note.schema.json `
  -o <temporary parse-result.json> `
  -
```

The app pins `gpt-5.6-luna`, writes the parser prompt and raw note to stdin, and validates parser output against `schemas/parse-note.schema.json` before applying it. Current parser output includes a short inbox title, Markdown `cleanedText`, a summary, tags, and suggested action items.

Settings include:

- `codexCommandPath`: defaults to `codex.cmd` on Windows.
- `parserTimeoutSeconds`: defaults to `90`.
- `parserMaxRetries`: defaults to `3`.
- `linkedWorkspacePaths`: optional local repo or directory paths that the parser agent may inspect when a note needs repo context.

The background worker reads persisted parser settings before processing jobs, so command path, timeout, retry count, and linked workspace changes affect later jobs without an app restart.

## Theme Template

Themes are developer-defined. Components should consume semantic CSS variables only and should not hardcode palette colors.

Registered themes currently include:

- `dark-compact`: default dense utility theme.
- `memphis`: alternate compact theme exposed in settings as `Memphis '86`.

To add another theme:

1. Add a `ThemeDefinition` in `src/lib/theme/themes.ts`.
2. Populate every token from `src/lib/theme/tokens.ts`.
3. Add it to the exported `themes` array so the settings UI can list it.
4. Keep component CSS pointed at variables such as `--color-surface-1`, `--color-text-primary`, and `--color-accent-primary`.

Required tokens:

```text
app.bg
surface.1
surface.2
surface.input
border.default
border.strong
text.primary
text.muted
accent.primary
accent.hot
status.success
status.warning
status.error
```

Default theme: Dark Compact (`dark-compact`).

## Architecture Notes

- Svelte components call narrow Tauri commands.
- Rust services own workflow behavior.
- Repositories own SQLite reads and writes.
- `ParseQueue` owns parser job lifecycle.
- `CodexParserProvider` is the only module that knows how to execute `codex`.
- `ParserResultApplier` owns parser-output trust rules.
- `windowing` owns tray, hotkey, and quick-capture positioning.

This keeps capture, parsing, persistence, and UI rendering separate enough to extend without scattering behavior across components.
