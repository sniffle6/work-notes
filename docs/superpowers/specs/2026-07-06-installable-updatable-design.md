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
`src-tauri/src/lib.rs` — add both plugins inline to the existing fluent `tauri::Builder::default()` chain, right next to the current desktop-only plugins (`global_shortcut`, `single_instance`), matching the codebase's established registration style:
```rust
.plugin(tauri_plugin_updater::Builder::new().build())
.plugin(tauri_plugin_process::init())
```
The project is desktop-only and already registers `global_shortcut`/`single_instance` inline without `#[cfg(desktop)]` guards. **Follow that existing pattern** — do not introduce a `let mut builder = …; #[cfg(desktop)] { builder = builder… }` refactor just to add these two. That would fight the current convention for no benefit and hurt locality.

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

npm deps: `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`.

New module `src/lib/updater.ts`. Designed as a **deep module tested through its interface**: the decision logic (a small state machine over check → prompt → install → relaunch, plus error/silent branches) is the thing worth testing, and it must be testable without Tauri IPC. So the module separates a pure-ish **decision core** from a thin **Tauri-backed adapter** at an injected seam.

**Seam — `UpdaterPort`** (the interface the core depends on; everything Tauri-specific hides behind it):
```ts
interface UpdaterPort {
  check: () => Promise<UpdateHandle | null>;   // UpdateHandle.downloadAndInstall()
  confirm: (message: string) => Promise<boolean>;
  relaunch: () => Promise<void>;
  notify: (message: string) => void;           // "up to date" / "check failed"
}
```

**Core — deep, returns a result instead of firing side effects into the void:**
```ts
type UpdateOutcome =
  | { kind: 'none' }            // no update, silent
  | { kind: 'up-to-date' }      // no update, user was told
  | { kind: 'installing' }      // user confirmed; install + relaunch invoked
  | { kind: 'declined' }        // update existed; user chose Later
  | { kind: 'error'; error: unknown };

async function runUpdateCheck(port: UpdaterPort, opts: { silent: boolean }): Promise<UpdateOutcome>;
```
Behaviour:
- `check()` returns null → `silent` ? `{none}` : `port.notify("You're up to date.")` + `{up-to-date}`.
- update exists → `port.confirm("Work Notes vX.Y.Z is available — Install & restart?")`; on true → `handle.downloadAndInstall()` then `port.relaunch()` → `{installing}`; on false → `{declined}`.
- anything throws → `port.notify(...)` only when `!silent`; return `{error}`. **A failed or offline check must never block startup** — the caller ignores the returned promise's timing.

**Adapter — `createTauriUpdaterPort(): UpdaterPort`** wires the real plugins (`plugin-updater` `check()`, `plugin-dialog` `ask()`, `plugin-process` `relaunch()`). This is the "large adapter, small implementation" that confines *all* Tauri coupling to one spot.

**Why the seam is real, not speculative:** two adapters exist — the Tauri-backed one for production and a fake `UpdaterPort` in tests. Per the deep-module rule, two adapters = a justified seam. The fake port lets the branch/state-machine logic be unit-tested by asserting the returned `UpdateOutcome` and which port methods were called — no jsdom/IPC mocking, and it satisfies the TDD-for-state-machines rule.

**Wiring (two callers, one seam):**
- **Startup:** on main-window mount, `void runUpdateCheck(createTauriUpdaterPort(), { silent: true })` — fire-and-forget, non-blocking.
- **Manual:** subscribe to the `check-for-updates` event (from the tray) and run `runUpdateCheck(createTauriUpdaterPort(), { silent: false })`.

### Tray — `src-tauri/src/windowing/tray.rs`
- Add a `CHECK_UPDATES_MENU_ID` item labeled **"Check for updates"** to the menu.
- In `on_menu_event`, when that id fires: get the main window and `emit("check-for-updates", ())` to it (show the main window first if hidden so the dialog has a parent).

**Seam placement (deliberate):** all update orchestration lives on the JS side; the tray is a *dumb trigger* that only fires a named event and knows nothing about updating. This keeps a single implementation of the update flow (invoked from both startup and the tray) rather than splitting logic across the Rust/JS IPC — better locality, and the prompt UX is JS-native. Running the orchestration in Rust (a `check_for_updates` command) was considered and rejected: it would smear the flow across the IPC seam (logic in Rust, prompt in JS) for no gain.

**Fragile seam to note:** `"check-for-updates"` is a stringly-typed contract shared across the Rust→JS boundary — a typo on either side breaks it silently with no compile error. Define the literal once per side (a Rust `const` and a TS constant) and cover this path in the manual verification, since it is inherently integration-only (not unit-testable).

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
| `src/lib/updater.ts` (new) | deep decision core (`runUpdateCheck` + `UpdaterPort` + `UpdateOutcome`) and the Tauri-backed adapter (`createTauriUpdaterPort`) |
| `src/lib/updater.test.ts` (new) | unit tests of the decision core via a fake `UpdaterPort` |
| main window startup wiring | call silent check on mount; subscribe to `check-for-updates` |
| `package.json` | + `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process` |
| `.github/workflows/release.yml` (new) | `tauri-action` CI on `v*` tag |
| `RELEASING.md` (new) | release runbook |

## Verification plan

**Unit (through the interface, no Tauri):** drive `runUpdateCheck` with a fake `UpdaterPort` and assert the returned `UpdateOutcome` plus which port methods were called, for each branch: no-update/silent, no-update/loud, update+confirm (installs + relaunches), update+decline, and check-throws (silent vs loud). This is the state-machine coverage the TDD rule requires and runs in the existing vitest setup.

**End-to-end:** an updater cannot be proven with a single build — it needs a "from" and a "to":
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
