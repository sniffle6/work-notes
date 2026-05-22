# Work Notes

Work Notes is a Windows-only local desktop app for fast coworker drive-by note capture. It is optimized for opening quickly, saving raw notes immediately, and letting background parsing generate a scannable title, clean up notes, add tags, summarize, and extract suggested action items later.

## Stack

- Tauri v2 for the Windows desktop shell, tray, hotkey, and native process work.
- Svelte, SvelteKit, TypeScript, and Vite for the UI.
- Rust for commands, SQLite persistence, parsing, and window ownership.
- SQLite with FTS5 for local note storage and search.
- Local `codex exec` for parsing through the user's Codex subscription.

The app does not use OpenAI API keys or an OpenAI API billing path. Parser work runs through the local Codex CLI and the user's Codex subscription.

## Development

```powershell
npm install
npm run tauri dev
npm test
npm run build
scripts\cargo-test.cmd
```

Use `scripts\cargo-test.cmd` for Rust tests on Windows. It loads the Visual Studio Developer Command Prompt environment before running Cargo.

For a debug package build from a normal PowerShell session:

```powershell
cmd.exe /c "call ""C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 >nul && npm run tauri build -- --debug"
```

More setup details are in [docs/development.md](docs/development.md).

## Capture Workflow

- The app runs with a main inbox window plus a hidden `quick-capture` window.
- The tray menu exposes `Open`, `Quick Note`, and `Quit`.
- The default global hotkey is `Ctrl+Shift+Space`.
- Quick capture is positioned bottom-right, focuses the note field, and saves with `Enter`.
- `Shift+Enter` inserts a newline.
- `Esc` closes the quick-capture window and preserves the draft while the process remains alive.

Raw note text is saved first and remains the source of truth. Parser output is stored as derived data.

## Parsing

Saving a note enqueues parse work. The background parser worker claims one queued job at a time, invokes local `codex exec`, validates JSON against `schemas/parse-note.schema.json`, records the parse run, and applies a title, Markdown cleaned text, summary, tags, and suggested action items as derived data.

Action items stay `suggested` until accepted, dismissed, or completed by the user. Failed or unsatisfactory parses can be retried, including with user feedback.

## Verification

Use [docs/testing.md](docs/testing.md) to choose verification for the change being made. Historical verification snapshots are kept in [docs/verification/](docs/verification/).
