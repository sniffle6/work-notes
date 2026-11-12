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

The database owns notes, tags, action items, parse jobs, parse runs, settings, Jira agent conversations, note-to-ticket links, and the FTS5 index.

## Jira

The Jira view needs these before it can load tickets:

1. Set `JIRA_EMAIL` (or `BITBUCKET_EMAIL`) to the Atlassian account email.
2. Set `JIRA_API_TOKEN` to a Jira-scoped API token with `read:jira-work` and `write:jira-work`.
3. Restart Work Notes so it sees the variables.
4. Enter the Jira site URL and project key in Settings > Jira.

```powershell
[Environment]::SetEnvironmentVariable("JIRA_EMAIL", "you@example.com", "User")
[Environment]::SetEnvironmentVariable("JIRA_API_TOKEN", "<token>", "User")
```

The token is read from the environment only. See [jira.md](jira.md) for the gateway routing, the agent contract, and the approval rules.

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
- `everforest-dark` and `everforest-light`: the two modes of the Everforest family.
- `dusk`: dark theme with a background gradient. Use it to check how a pane looks over a gradient.

### Theme Agent

Settings > Appearance can build a theme from a description. The agent runs through local `codex exec` and must return JSON matching `schemas/theme-agent.schema.json`: a name, a one-line note, one `#rrggbb` colour per token, and `backgroundGradient`, which is `null` or an angle with two or three colour stops.

- The app normalizes the colours, decides dark or light from the background's luminance, and checks WCAG contrast: 4.5:1 for primary and muted text, 3:1 for the hot accent and status colours.
- A background gradient is held to the same minimums as `appBg`. The check samples the blend between stops, because the weakest point can fall between them.
- The gradient is exposed as `--color-app-bg-paint`, which holds the flat `appBg` colour for themes without one. Panes that show the app background use `background: var(--color-app-bg-paint, var(--color-app-bg))`. The gradient is pinned to the window, so each pane's own paint lines up as one sweep. Keep using `--color-app-bg` inside `color-mix()` and for small controls: a gradient is not a colour.
- Failed checks get one repair turn that starts from the first attempt. Problems that remain are returned as warnings and shown beside the preview.
- A built theme is only a draft. It is painted over the whole app while the settings dialog is open and is dropped when the dialog closes unless the user keeps it.
- Kept themes are stored in settings as `customThemes`. Their ids start with `custom-` so they cannot shadow a built-in theme, and their mode is always recomputed from the background.

To add another built-in theme:

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
- `parser/codex_provider.rs` is the only module that knows how to execute `codex`. The Jira agent runs its turns through `run_structured_exec` there.
- `jira/transport.rs` is the only module that performs Jira network I/O.
- `JiraAgentService` owns the agent loop and the rule that Jira changes wait for approval.
- `ParserResultApplier` owns parser-output trust rules.
- `windowing` owns tray, hotkey, and quick-capture positioning.

This keeps capture, parsing, persistence, and UI rendering separate enough to extend without scattering behavior across components.
