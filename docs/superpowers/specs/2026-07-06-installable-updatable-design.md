# Design: Making Work Notes Installable & Updatable

**Date:** 2026-07-06
**Status:** Approved (design), pending implementation plan
**Repo:** https://github.com/sniffle6/work-notes (public, default branch `master`)
**App:** Work Notes — Tauri 2 desktop app (SvelteKit frontend, Rust backend, SQLite via rusqlite, tray icon, global-shortcut quick capture, single-instance)

## Decisions (locked)

| Decision | Choice | Rationale |
|---|---|---|
| Audience | Just me now, public later | Build personal-scale on a public-ready foundation |
| Build/release | GitHub Actions CI (`tauri-action`) | Same pipeline now and when public; zero manual per-release steps |
| Hosting | GitHub Releases (public repo) | Public repo → clean `latest.json` permalink, no auth headers |
| Update UX | Prompt on startup + manual tray check | Respectful; no surprise installs |
| Platforms | Windows-only now | User is on Windows; cross-platform deferred to "public later" |

## Strategy

Build the personal-scale setup on the **public-ready foundation**, so "going public later" is *additive* (add code-signing certs + more OS targets to the matrix), never a rewrite. Everything below is the same pipeline a public release would use — just a Windows-only matrix with no signing cert yet.

### The one non-retrofittable constraint

Tauri's updater refuses any update whose minisign signature was not produced by the key whose **public half was baked into the installed binary**. Therefore the updater signing keypair must exist **before the first real install**. A build shipped without it can never auto-update; existing installs would require a manual reinstall to get onto the updatable track. Because distribution is currently just the developer, the practical impact is nil — but the key must be generated and its public half committed as part of this work, before the first CI release.

## Component 1 — Installer ("installable")

- Set Windows `bundle.targets` to `["nsis"]` (drop the redundant MSI). NSIS gives per-user install (no admin prompt) and clean **passive** auto-updates.
- Icons already present (`icons/`), identifier `com.aweber.worknotes` unchanged.
- Output artifact: `Work Notes_X.Y.Z_x64-setup.exe`.

## Component 2 — Auto-updater ("updatable")

### Rust
`src-tauri/Cargo.toml` — add to the existing `[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]` block:
```toml
tauri-plugin-updater = "2"
tauri-plugin-process = "2"   # powers relaunch() after install
```
`src-tauri/src/lib.rs` — register both plugins, guarded so mobile still compiles:
```rust
#[cfg(desktop)]
{
    builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
}
```
(Adapt to the existing builder chain; the existing desktop-only plugins — global-shortcut, single-instance — can move under the same guard for correctness, but that is optional and out of scope unless trivial.)

### Config — `src-tauri/tauri.conf.json`
```jsonc
"bundle": {
  "active": true,
  "targets": ["nsis"],
  "createUpdaterArtifacts": true,
  "icon": [ /* unchanged */ ]
},
"plugins": {
  "updater": {
    "endpoints": [
      "https://github.com/sniffle6/work-notes/releases/latest/download/latest.json"
    ],
    "pubkey": "<minisign public key — filled in after key generation>",
    "windows": { "installMode": "passive" }
  }
}
```

### Capabilities — `src-tauri/capabilities/default.json`
Add to `permissions`:
```json
"updater:default",
"process:allow-restart"
```

### Frontend
- npm deps: `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`.
- New module `src/lib/updater.ts`:
  - `runUpdateCheck({ silent }): Promise<void>`
    - `const update = await check();`
    - If `update` is null: when `silent` is false, show "You're up to date."; when silent, do nothing.
    - If an update exists: show dialog *"Work Notes vX.Y.Z is available — Install & restart? / Later"*. On confirm: `await update.downloadAndInstall(); await relaunch();`
    - Wrap in try/catch; on error, log and (if not silent) surface a non-blocking "Update check failed" message. A failed/absent network must never block app startup.
  - **Startup**: call `runUpdateCheck({ silent: true })` once on app mount (main window), non-blocking.
  - **Manual**: listen for a `check-for-updates` event (emitted by the tray) and call `runUpdateCheck({ silent: false })`.

### Tray — `src-tauri/src/windowing/tray.rs`
- Add a `CHECK_UPDATES_MENU_ID` item labeled **"Check for updates"** to the menu.
- In `on_menu_event`, when that id fires: get the main window and `emit("check-for-updates", ())` to it (show the main window first if hidden so the dialog has a parent).

### Update UX summary
- **On launch:** silent check; prompt only if an update is found; install on user confirm; relaunch.
- **Manual:** tray "Check for updates" always reports a result, including up-to-date.
- Passive Windows install mode: small progress UI, no user interaction during install.

## Component 3 — Signing keys & secrets

One-time, local:
```bash
npm run tauri signer generate -- -w "$HOME/.tauri/work-notes.key"
```
- Public key → `plugins.updater.pubkey` in `tauri.conf.json` (committed; baked into every build).
- Private key contents → GitHub repo secret `TAURI_SIGNING_PRIVATE_KEY`.
- Chosen password → GitHub repo secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- **Back up the private key** in a password manager. Loss = no install can auto-update until manually reinstalled with a new key.

## Component 4 — Release pipeline (GitHub Actions)

New `.github/workflows/release.yml`, triggered on pushing a `v*` tag (plus `workflow_dispatch` for manual runs):
- Runner: `windows-latest` (matrix structured so `macos-latest` / `ubuntu-latest` rows can be added later).
- Steps: checkout → setup Node → setup Rust (stable) → `npm ci` → `tauri-apps/tauri-action`.
- `tauri-action` inputs: `tagName`, `releaseName`, `releaseDraft: true`, `includeUpdaterJson: true`.
- Env: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `GITHUB_TOKEN`.
- Result: a **draft** GitHub Release containing the NSIS installer, its `.sig`, and `latest.json`.
- Developer reviews the draft and clicks **Publish** → it becomes "Latest" → the updater permalink (`releases/latest/download/latest.json`) resolves and clients can update.

Rationale for draft-then-publish: the `releases/latest/download/...` permalink only resolves to a published, non-prerelease "Latest" release. Draft gives a review gate before anything goes live; publishing is the single deliberate go-live action.

## Component 5 — Versioning & release process

- Single source of truth for app version: `version` in `tauri.conf.json`. Keep `src-tauri/Cargo.toml` and `package.json` in sync.
- The updater compares `latest.json`'s `version` against the running app's version. Tag `vX.Y.Z`; `tauri-action` strips the leading `v`.
- Release runbook (new `RELEASING.md`): bump version in the three files → commit → `git tag vX.Y.Z` → push tag → CI drafts the release → review → Publish.

## Files touched

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | + `tauri-plugin-updater`, `tauri-plugin-process` (desktop target) |
| `src-tauri/src/lib.rs` | register the two plugins under `#[cfg(desktop)]` |
| `src-tauri/tauri.conf.json` | updater config, `createUpdaterArtifacts: true`, `targets: ["nsis"]` |
| `src-tauri/capabilities/default.json` | + `updater:default`, `process:allow-restart` |
| `src-tauri/src/windowing/tray.rs` | + "Check for updates" item that emits `check-for-updates` |
| `src/lib/updater.ts` (new) | check / prompt / install / relaunch flow |
| main window startup wiring | call silent check on mount; subscribe to `check-for-updates` |
| `package.json` | + `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process` |
| `.github/workflows/release.yml` (new) | `tauri-action` CI on `v*` tag |
| `RELEASING.md` (new) | release runbook |

## Verification plan

An updater cannot be proven with a single build — it needs a "from" and a "to":
1. Generate keys, wire everything, set version `0.1.0`.
2. Tag `v0.1.0` → CI drafts → publish → install the resulting `-setup.exe` locally.
3. Bump to `0.1.1`, tag `v0.1.1` → CI drafts → publish.
4. Launch the installed `0.1.0`: confirm it detects `0.1.1`, prompts, downloads, verifies the signature, installs (passive), and relaunches into `0.1.1`.
5. Tray "Check for updates" on the latest version: confirm the "You're up to date" path.
6. Startup with no network: confirm the app launches normally and the failed check is silent.

## Deferred to "going public" (NOT built now)

None of these change the architecture above:
- **Code-signing certificate** to remove the Windows SmartScreen "unknown publisher" warning (OV cert, or Azure Trusted Signing). Until then, users other than the developer see a one-time warning.
- **Cross-platform matrix rows**: macOS `.app` + notarization, Linux AppImage.
- Optional switch from draft to auto-publish once the process is trusted.

## Known trade-offs / risks

- **Unsigned installer:** anyone other than the developer gets a one-time SmartScreen warning. Irrelevant for personal use; resolved at "public" via a cert.
- **Private key is a real secret:** losing it breaks auto-update for all existing installs. Must be backed up.
- **Updater needs two releases to validate** — cannot be fully verified from a single build.
