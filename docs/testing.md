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
- `src/lib/components/InboxList.test.ts`: Notes/Actions mode behavior and suggested-action count labels.
- `src/lib/components/QuickCapturePanel.test.ts`: quick-capture keyboard behavior.
- `src/lib/components/NoteDetail.test.ts`: reparse feedback and delete events.
- `src/lib/components/SettingsView.test.ts`: theme selection and settings save behavior.
- `src/lib/markdown.test.ts`: Markdown rendering, pipe tables, preserved line breaks, and HTML escaping.
- `src/lib/stores/jira.test.ts`: Jira store sequencing with an injected fake API, including proposal approval and stale-response handling.
- `src/lib/components/JiraView.test.ts`: Jira screen driven through the real store and a fake API.
- `src/lib/components/JiraIssueDetail.test.ts`: transitions, attachments, comments, edits, and the flatten-formatting confirmation.
- `src/lib/components/JiraChat.test.ts`: message entry keys, lookups, and Approve/Reject on proposals.
- `src/lib/theme/theme.test.ts`: theme token to CSS variable mapping, including the app background paint.
- `src/lib/theme/custom.test.ts`: saved and draft themes as theme definitions, and reading a background gradient.
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
- parser result application trust rules, including title updates
- capture service behavior
- parse queue success, failure, retry, cancelling, and parse-run recording
- command DTO serialization
- settings validation
- hotkey validation
- quick-capture positioning helpers
- Jira issue key validation, ADF conversion, transition matching, and HTTP error mapping
- Jira client requests against a fake transport
- Jira attachment downloads into a temporary folder: file naming, skipping saved files, and failed downloads
- Jira agent prompt, turn parsing, tool-call validation, and the approval-gated agent loop with fake agent and Jira
- Jira chat and note-link repositories
- structured Codex exec against a stand-in command script

## Jira Checks

For Jira work, run targeted Rust tests first, then the full Rust command:

```powershell
scripts\cargo-test.cmd jira
scripts\cargo-test.cmd
```

Unit tests must not call Jira or `codex exec`. Use the fake transport, fake agent runner, and fake Jira in the test modules.

Ignored tests check real services and are run by hand. Set the Jira environment variables and `WORK_NOTES_JIRA_SITE_URL` first:

```powershell
scripts\cargo-test.cmd live_jira -- --ignored --nocapture
scripts\cargo-test.cmd live_agent -- --ignored --nocapture
scripts\cargo-test.cmd live_theme -- --ignored --nocapture
```

`live_jira` only reads from Jira. `live_agent` runs real Codex conversations with read-only lookups and never approves a proposal; its draft test, `live_agent_drafts`, needs no Jira setup. `live_theme` asks real Codex for two themes, one flat and one with a background gradient, and needs no Jira setup.

Manual checks in the running app after Jira changes:

- With setup missing, the Jira view lists the missing steps and loads nothing.
- Filters load tickets; selecting a ticket shows description, comments, and transitions.
- A transition, a comment, an edit, and Assign to me each update the open ticket and its list row.
- An image attachment shows a thumbnail; clicking it opens the full image in the app, and the viewer's folder button opens File Explorer on the file.
- A ticket with attachments lists them. Download and Download all save files to `Downloads\Work Notes\<KEY>`, and Show in folder opens File Explorer on the file.
- Copy agent prompt on a ticket with attachments lists each attachment by local path.
- Create ticket from a note prefills the form, opens the agent, and the agent rewrites the form into a ticket. Nothing is in Jira yet.
- Asking the agent to change the draft updates the form, and an edit typed into the form is kept by the next rewrite of another field.
- Create ticket on the draft shows the new key on the note afterwards.
- An agent request to change a ticket appears as a proposal and changes nothing until approved.
- Rejecting a proposal changes nothing in Jira.

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

