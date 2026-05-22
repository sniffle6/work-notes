# Work Notes v1 Verification - 2026-05-20

This is a historical verification snapshot from the original v1 handoff. Use `docs/testing.md` for current verification guidance and rerun the listed commands before treating results as current.

## Automated Checks

```powershell
npm test
```

Result: passed. Vitest reported 4 test files and 12 tests passing.

```powershell
npm run check
```

Result: passed. `svelte-check` reported 0 errors and 0 warnings.

```powershell
npm run build
```

Result: passed. Vite and SvelteKit production build completed and wrote `build`.

```powershell
scripts\cargo-test.cmd
```

Result: passed. Rust reported 24 tests passing.

```powershell
cmd.exe /c "call ""C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat"" -arch=x64 -host_arch=x64 >nul && npm run tauri build -- --debug"
```

Result: passed. Tauri built:

- `src-tauri\target\debug\work-notes.exe`
- `src-tauri\target\debug\bundle\msi\Work Notes_0.1.0_x64_en-US.msi`
- `src-tauri\target\debug\bundle\nsis\Work Notes_0.1.0_x64-setup.exe`

## Browser Smoke Check

Target: `http://127.0.0.1:1420/`

Verified through Playwright CLI:

- Page title became `Work Notes` after hydration.
- Inbox, note detail, review queue, settings, and Dark Compact theme rendered.
- Inbox search/status/tag filters were visible.
- Quick-capture textarea was focused.
- Enter saved a fallback note.
- The saved note appeared at the top of the inbox with `queued` parse status.
- Quick capture closed after save.

This smoke check uses the frontend fallback API, not the native Tauri command bridge.

## Native Windows Checks

Build-level native checks completed:

- Main and `quick-capture` windows are configured in `tauri.conf.json`.
- `quick-capture` is hidden by default, undecorated, skipped from the taskbar, and sized `560x260`.
- Rust windowing positions quick capture bottom-right on the target monitor.
- Tray menu registers `Open`, `Quick Note`, and `Quit`.
- Default global hotkey is `Ctrl+Shift+Space`.
- `hide_quick_capture` command is registered for `Esc` and post-save close behavior.
- Background parse worker starts during Tauri setup.
- Parser worker reads saved Codex command path, timeout, and retry settings before jobs.
- Hotkey settings are validated before persistence and shortcut replacement unregisters older app shortcuts.
- Note capture creates the raw note and queued parse job in one SQLite transaction.
- Active parse jobs are reused so repeated retry actions do not create duplicate queued/parsing work.
- Parse runs retain the provider raw response separately from normalized parsed JSON.

Manual interactive checks still need to be performed on the actual desktop session:

- Tray icon appears in the Windows notification area.
- Global hotkey opens quick capture from another focused app.
- Quick capture appears bottom-right on the active monitor.
- Native quick-capture `Enter`, `Shift+Enter`, and `Esc` behavior matches the spec.
- Main inbox opens from the tray menu.
- Real `codex exec` parsing succeeds with the user's logged-in Codex subscription.
- Parser failure is visible and leaves raw note text intact.

## Review Findings Addressed

- Added a running background parse worker.
- Applied successful parser output in a single SQLite transaction.
- Replaced unreviewed parser-suggested actions on retry to avoid duplicates.
- Kept accepted/dismissed/done actions intact during parser retries.
- Added saved parser settings into startup and worker runtime behavior.
- Drained Codex stderr while the process runs to avoid pipe deadlocks.
- Added a native hide command for quick capture.
- Added frontend focus handling for quick-capture show events.
- Rendered a dedicated quick-capture-only surface in the `quick-capture` Tauri window.
- Added visible inbox filters and backend tag data in inbox rows.
- Re-query inbox data when filters change so SQLite/FTS remains the filtering source of truth.
- Aligned Dark Compact theme tokens with the approved palette.
- Added parser health, launch-at-startup, and minimize-to-tray settings controls.
- Guarded startup against invalid persisted hotkeys by falling back to the default shortcut.

## Residual Risks

- The native tray/hotkey flow has build coverage and pure Rust positioning tests, but still needs a live Windows desktop pass.
- The parser worker is intentionally one job at a time for v1.
