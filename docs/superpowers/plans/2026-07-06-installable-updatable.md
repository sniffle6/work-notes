# Installable & Updatable Work Notes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Work Notes as a Windows NSIS installer that auto-updates via Tauri's updater plugin, published from GitHub Actions to public GitHub Releases.

**Architecture:** A deep TypeScript update-decision core (`runUpdateCheck`) sits behind an injected `UpdaterPort` seam; a Tauri-backed adapter wires the real updater/dialog/process plugins, and a test fake exercises every branch. The Rust side registers the updater + process plugins, the tray gains a "Check for updates" trigger that emits an event to the main window, and `tauri-action` builds/signs/publishes releases with a `latest.json` the app reads from the `releases/latest/download` permalink.

**Tech Stack:** Tauri 2, Rust, SvelteKit 2 + Svelte 5, TypeScript, Vitest, `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`, `@tauri-apps/plugin-dialog`, GitHub Actions + `tauri-apps/tauri-action`, minisign signing.

## Global Constraints

- Tauri version: `2` (all plugins pinned to `2`).
- Platforms now: **Windows only**. Cross-platform (macOS/Linux) is deferred — do not add other matrix rows.
- Windows installer target: **NSIS** (`bundle.targets: ["nsis"]`).
- App version source of truth: `version` in `src-tauri/tauri.conf.json`; keep `src-tauri/Cargo.toml` and `package.json` in sync at the same value.
- Updater endpoint (verbatim): `https://github.com/sniffle6/work-notes/releases/latest/download/latest.json`
- `plugins.updater.pubkey` must be the **key content** (base64 string), never a file path.
- Windows updater install mode: `passive`.
- Tauri plugins are loaded via **dynamic import** in adapter/runtime code (matches existing `api.ts` convention `await import("@tauri-apps/plugin-dialog")`), so the pure core stays free of Tauri imports.
- Event naming convention: `work-notes:<name>` (matches existing `NOTE_CAPTURED_EVENT = "work-notes:note-captured"`).
- Runtime-guard convention: `isTauriRuntime()` = `typeof window !== "undefined" && "__TAURI_INTERNALS__" in window`.
- Update check runs in the **main window only** (both windows load `index.html`; guard on window label).
- TDD is required for the update-decision core (it is a state machine). Rust/config/CI/docs tasks are declarative and verified by compile/lint, not unit tests.
- Work happens on branch `feat/app-distribution-updater` (already created). Commit after every task.

---

### Task 1: Update-decision core (`runUpdateCheck`) — pure TS, TDD

**Files:**
- Create: `src/lib/updater.ts`
- Test: `src/lib/updater.test.ts`

**Interfaces:**
- Consumes: nothing (pure module, no Tauri imports).
- Produces:
  - `interface UpdateHandle { version: string; downloadAndInstall: () => Promise<void>; }`
  - `interface UpdaterPort { check: () => Promise<UpdateHandle | null>; confirm: (message: string) => Promise<boolean>; relaunch: () => Promise<void>; notify: (message: string) => void; }`
  - `type UpdateOutcome = { kind: "none" } | { kind: "up-to-date" } | { kind: "installing" } | { kind: "declined" } | { kind: "error"; error: unknown }`
  - `function runUpdateCheck(port: UpdaterPort, opts: { silent: boolean }): Promise<UpdateOutcome>`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/updater.test.ts`:

```ts
import { describe, expect, it, vi } from "vitest";
import { runUpdateCheck, type UpdateHandle, type UpdaterPort } from "./updater";

function createPort(overrides: Partial<UpdaterPort> = {}): UpdaterPort {
  return {
    check: vi.fn(async () => null),
    confirm: vi.fn(async () => false),
    relaunch: vi.fn(async () => {}),
    notify: vi.fn(),
    ...overrides,
  };
}

function createUpdate(): UpdateHandle {
  return { version: "1.2.3", downloadAndInstall: vi.fn(async () => {}) };
}

describe("runUpdateCheck", () => {
  it("returns 'none' and stays quiet when no update and silent", async () => {
    const port = createPort();
    const outcome = await runUpdateCheck(port, { silent: true });
    expect(outcome).toEqual({ kind: "none" });
    expect(port.notify).not.toHaveBeenCalled();
  });

  it("notifies and returns 'up-to-date' when no update and not silent", async () => {
    const port = createPort();
    const outcome = await runUpdateCheck(port, { silent: false });
    expect(outcome).toEqual({ kind: "up-to-date" });
    expect(port.notify).toHaveBeenCalledWith("You're up to date.");
  });

  it("installs and relaunches when an update exists and the user confirms", async () => {
    const update = createUpdate();
    const port = createPort({ check: vi.fn(async () => update), confirm: vi.fn(async () => true) });
    const outcome = await runUpdateCheck(port, { silent: true });
    expect(update.downloadAndInstall).toHaveBeenCalledOnce();
    expect(port.relaunch).toHaveBeenCalledOnce();
    expect(outcome).toEqual({ kind: "installing" });
  });

  it("does nothing when an update exists but the user declines", async () => {
    const update = createUpdate();
    const port = createPort({ check: vi.fn(async () => update), confirm: vi.fn(async () => false) });
    const outcome = await runUpdateCheck(port, { silent: false });
    expect(update.downloadAndInstall).not.toHaveBeenCalled();
    expect(port.relaunch).not.toHaveBeenCalled();
    expect(outcome).toEqual({ kind: "declined" });
  });

  it("passes the version into the confirm prompt", async () => {
    const update = createUpdate();
    const confirm = vi.fn(async () => false);
    const port = createPort({ check: vi.fn(async () => update), confirm });
    await runUpdateCheck(port, { silent: true });
    expect(confirm).toHaveBeenCalledWith("Work Notes 1.2.3 is available — Install & restart?");
  });

  it("swallows errors silently when silent", async () => {
    const port = createPort({ check: vi.fn(async () => { throw new Error("network"); }) });
    const outcome = await runUpdateCheck(port, { silent: true });
    expect(outcome.kind).toBe("error");
    expect(port.notify).not.toHaveBeenCalled();
  });

  it("reports errors via notify when not silent", async () => {
    const port = createPort({ check: vi.fn(async () => { throw new Error("network"); }) });
    const outcome = await runUpdateCheck(port, { silent: false });
    expect(outcome.kind).toBe("error");
    expect(port.notify).toHaveBeenCalledWith("Update check failed. Please try again later.");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- src/lib/updater.test.ts`
Expected: FAIL — cannot resolve `./updater` / `runUpdateCheck is not a function`.

- [ ] **Step 3: Write the minimal implementation**

Create `src/lib/updater.ts`:

```ts
/** A pending update the user can choose to install. */
export interface UpdateHandle {
  version: string;
  downloadAndInstall: () => Promise<void>;
}

/**
 * The seam the update flow depends on. The production adapter wires these to the
 * Tauri updater/dialog/process plugins; tests pass a fake. Keeping the core
 * dependent only on this interface is what makes the state machine testable.
 */
export interface UpdaterPort {
  check: () => Promise<UpdateHandle | null>;
  confirm: (message: string) => Promise<boolean>;
  relaunch: () => Promise<void>;
  notify: (message: string) => void;
}

export type UpdateOutcome =
  | { kind: "none" }
  | { kind: "up-to-date" }
  | { kind: "installing" }
  | { kind: "declined" }
  | { kind: "error"; error: unknown };

/**
 * Check for an update; if one exists and the user confirms, install it and
 * relaunch. Never throws — failures are reported via the port (unless silent)
 * and returned as { kind: "error" }, so callers can safely fire-and-forget.
 */
export async function runUpdateCheck(
  port: UpdaterPort,
  opts: { silent: boolean },
): Promise<UpdateOutcome> {
  try {
    const update = await port.check();
    if (!update) {
      if (opts.silent) {
        return { kind: "none" };
      }
      port.notify("You're up to date.");
      return { kind: "up-to-date" };
    }

    const confirmed = await port.confirm(
      `Work Notes ${update.version} is available — Install & restart?`,
    );
    if (!confirmed) {
      return { kind: "declined" };
    }

    await update.downloadAndInstall();
    await port.relaunch();
    return { kind: "installing" };
  } catch (error) {
    if (!opts.silent) {
      port.notify("Update check failed. Please try again later.");
    }
    return { kind: "error", error };
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/lib/updater.test.ts`
Expected: PASS — 7 passing.

- [ ] **Step 5: Commit**

```bash
git add src/lib/updater.ts src/lib/updater.test.ts
git commit -m "feat: update-decision core with injected UpdaterPort seam"
```

---

### Task 2: Tauri-backed adapter + updater/process npm deps

**Files:**
- Modify: `src/lib/updater.ts` (append the adapter)
- Modify: `package.json` (new deps, written by `npm install`)

**Interfaces:**
- Consumes: `UpdaterPort` (Task 1).
- Produces: `function createTauriUpdaterPort(): UpdaterPort`

- [ ] **Step 1: Install the runtime plugins**

Run:
```bash
npm install @tauri-apps/plugin-updater@^2 @tauri-apps/plugin-process@^2
```
Expected: `package.json` `dependencies` now include both; `package-lock.json` updated. (`@tauri-apps/plugin-dialog` is already a dependency.)

- [ ] **Step 2: Append the adapter to `src/lib/updater.ts`**

Add at the end of the file:

```ts
/**
 * Production UpdaterPort backed by the Tauri plugins. Plugins are imported
 * dynamically (matching the app's api.ts convention) so the pure core and its
 * tests never load Tauri modules. Only call this inside a Tauri window.
 */
export function createTauriUpdaterPort(): UpdaterPort {
  return {
    check: async () => {
      const { check } = await import("@tauri-apps/plugin-updater");
      return check();
    },
    confirm: async (message: string) => {
      const { ask } = await import("@tauri-apps/plugin-dialog");
      return ask(message, {
        title: "Work Notes",
        kind: "info",
        okLabel: "Install & restart",
        cancelLabel: "Later",
      });
    },
    relaunch: async () => {
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    },
    notify: (message: string) => {
      void import("@tauri-apps/plugin-dialog").then(({ message: showMessage }) =>
        showMessage(message, { title: "Work Notes" }),
      );
    },
  };
}
```

- [ ] **Step 3: Verify types and existing tests**

Run: `npm run check`
Expected: PASS (0 errors). The Tauri `Update` object structurally satisfies `UpdateHandle`.

Run: `npm test -- src/lib/updater.test.ts`
Expected: PASS — still 7 passing (adapter is not exercised by unit tests; it is integration-verified later).

- [ ] **Step 4: Commit**

```bash
git add package.json package-lock.json src/lib/updater.ts
git commit -m "feat: Tauri-backed UpdaterPort adapter + updater/process deps"
```

---

### Task 3: Signing keypair + GitHub secrets  ⚠️ DEVELOPER-PERFORMED — DO NOT DELEGATE TO A SUBAGENT

This task involves choosing/storing a password, backing up a private key, and setting repository secrets. A subagent must not do it. If executing this plan with subagents, **pause here** and have the developer complete these steps, then resume at Task 4 with the public key in hand.

**Files:** none committed (the private key must never be committed).

**Interfaces:**
- Produces (out of band): the minisign **public key** string used in Task 4's `tauri.conf.json`, and the secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` used in Task 7's CI.

- [ ] **Step 1: Generate the keypair**

Run:
```bash
npm run tauri signer generate -- -w "$HOME/.tauri/work-notes.key"
```
Choose a strong password when prompted. This writes `~/.tauri/work-notes.key` (private) and `~/.tauri/work-notes.key.pub` (public), and prints the public key.

- [ ] **Step 2: Back up the private key + password**

Store the contents of `~/.tauri/work-notes.key` and the password in a password manager. **Losing them means no existing install can auto-update** until manually reinstalled with a new key.

- [ ] **Step 3: Capture the public key for Task 4**

Run:
```bash
cat "$HOME/.tauri/work-notes.key.pub"
```
Copy the full string — it goes into `plugins.updater.pubkey` in Task 4.

- [ ] **Step 4: Set the CI secrets**

Run (replace the password value):
```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < "$HOME/.tauri/work-notes.key"
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body "YOUR_KEY_PASSWORD"
```

- [ ] **Step 5: Verify secrets exist**

Run: `gh secret list`
Expected: both `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` listed.

---

### Task 4: Rust plugins, capabilities, and updater/bundle config

**Files:**
- Modify: `src-tauri/Cargo.toml` (desktop-target dependencies)
- Modify: `src-tauri/src/lib.rs` (plugin registration)
- Modify: `src-tauri/capabilities/default.json` (permissions)
- Modify: `src-tauri/tauri.conf.json` (bundle + updater config)

**Interfaces:**
- Consumes: the public key string from Task 3.
- Produces: a compiling Rust binary with the updater/process plugins registered and updater config embedded (verified by `cargo build`, which parses `tauri.conf.json` at compile time via `generate_context!`).

- [ ] **Step 1: Add the Rust plugin dependencies**

In `src-tauri/Cargo.toml`, add to the existing `[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]` block (which already holds `tauri-plugin-global-shortcut` and `tauri-plugin-single-instance`):

```toml
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

- [ ] **Step 2: Register the plugins in `src-tauri/src/lib.rs`**

Add these two lines to the existing fluent `tauri::Builder::default()` chain, immediately after the `.plugin(tauri_plugin_opener::init())` line (inline, matching how `global_shortcut`/`single_instance` are already registered — do **not** introduce a `let mut builder` refactor):

```rust
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
```

- [ ] **Step 3: Add the capability permissions**

In `src-tauri/capabilities/default.json`, add these four entries to the `permissions` array:

```json
    "updater:default",
    "process:allow-restart",
    "dialog:allow-ask",
    "dialog:allow-message"
```

The full `permissions` array becomes:

```json
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "dialog:allow-ask",
    "dialog:allow-message",
    "global-shortcut:allow-is-registered",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "opener:default",
    "updater:default",
    "process:allow-restart"
  ]
```

- [ ] **Step 4: Configure bundle + updater in `src-tauri/tauri.conf.json`**

Replace the `"bundle"` object and add a `"plugins"` object. Paste the public key from Task 3 into `pubkey`. The `"bundle"` block becomes:

```json
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "createUpdaterArtifacts": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  },
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/sniffle6/work-notes/releases/latest/download/latest.json"
      ],
      "pubkey": "PASTE_PUBLIC_KEY_FROM_TASK_3",
      "windows": {
        "installMode": "passive"
      }
    }
  }
```

(Keep the existing top-level `"app"` object as-is; `"plugins"` is a new sibling of `"bundle"`.)

- [ ] **Step 5: Verify it compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: builds successfully. This compiles the Rust and validates the strongly-typed `bundle` block (`targets`, `createUpdaterArtifacts`) via `generate_context!`. The `plugins.updater` block is validated by the updater plugin at runtime (confirmed end-to-end in Task 8), so double-check by eye that `pubkey` is the key **content** and `endpoints` matches the constant in Global Constraints.

Run: `npm run check`
Expected: PASS (no TS impact, but confirms nothing else broke).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/capabilities/default.json src-tauri/tauri.conf.json
git commit -m "feat: register updater/process plugins and updater config (NSIS, passive)"
```

---

### Task 5: Startup silent update check (main window only)

**Files:**
- Modify: `src/lib/events.ts` (event name constant)
- Modify: `src/routes/+page.svelte` (wire the startup check into `onMount`)

**Interfaces:**
- Consumes: `runUpdateCheck`, `createTauriUpdaterPort` (Tasks 1–2).
- Produces: `export const CHECK_FOR_UPDATES_EVENT = "work-notes:check-for-updates"` (consumed by Task 6's tray listener).

- [ ] **Step 1: Add the event constant**

In `src/lib/events.ts`, add below the existing `NOTE_CAPTURED_EVENT` line:

```ts
export const CHECK_FOR_UPDATES_EVENT = "work-notes:check-for-updates";
```

- [ ] **Step 2: Import the updater in `+page.svelte`**

In the `<script>` block of `src/routes/+page.svelte`, add these imports alongside the existing `$lib` imports:

```ts
  import { runUpdateCheck, createTauriUpdaterPort } from "$lib/updater";
  import { NOTE_CAPTURED_EVENT, CHECK_FOR_UPDATES_EVENT, type NoteCapturedPayload } from "$lib/events";
```

(Replace the existing `import { NOTE_CAPTURED_EVENT, type NoteCapturedPayload } from "$lib/events";` line with the combined import above.)

- [ ] **Step 3: Fire the silent check on startup**

In `onMount`, inside the existing `if (currentWindowLabel !== "quick-capture") { ... }` block (the one that also registers the `NOTE_CAPTURED_EVENT` listener), add as the first statement in that block:

```ts
      void runUpdateCheck(createTauriUpdaterPort(), { silent: true });
```

This runs only in the main window (the guard already excludes `quick-capture`), only under the Tauri runtime (the whole `onMount` body past the early `isTauriRuntime()` return is Tauri-only), and is fire-and-forget (the core never throws).

- [ ] **Step 4: Verify types**

Run: `npm run check`
Expected: PASS (0 errors).

- [ ] **Step 5: Commit**

```bash
git add src/lib/events.ts src/routes/+page.svelte
git commit -m "feat: silent update check on main-window startup"
```

---

### Task 6: Tray "Check for updates" item + event listener

**Files:**
- Modify: `src-tauri/src/windowing/tray.rs` (menu item + emit)
- Modify: `src/routes/+page.svelte` (listen for the event)

**Interfaces:**
- Consumes: `CHECK_FOR_UPDATES_EVENT` (Task 5), `runUpdateCheck`, `createTauriUpdaterPort` (Tasks 1–2).
- Produces: a tray menu item that emits `"work-notes:check-for-updates"` to the main window.

- [ ] **Step 1: Rewrite `initialize_tray_menu` in `src-tauri/src/windowing/tray.rs`**

Replace the entire file with:

```rust
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use super::{
    quick_capture::show_quick_capture_window, show_main_window, MAIN_WINDOW_LABEL,
    QUICK_CAPTURE_WINDOW_LABEL,
};

const OPEN_MAIN_MENU_ID: &str = MAIN_WINDOW_LABEL;
const SHOW_QUICK_CAPTURE_MENU_ID: &str = QUICK_CAPTURE_WINDOW_LABEL;
const CHECK_UPDATES_MENU_ID: &str = "check-for-updates";
const QUIT_MENU_ID: &str = "quit";
const TRAY_ID: &str = "work-notes-tray";
const CHECK_FOR_UPDATES_EVENT: &str = "work-notes:check-for-updates";

pub fn initialize_tray_menu(app: &AppHandle) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, OPEN_MAIN_MENU_ID, "Open", true, None::<&str>)?;
    let show_quick_capture = MenuItem::with_id(
        app,
        SHOW_QUICK_CAPTURE_MENU_ID,
        "Quick Note",
        true,
        None::<&str>,
    )?;
    let check_updates = MenuItem::with_id(
        app,
        CHECK_UPDATES_MENU_ID,
        "Check for updates",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_main, &show_quick_capture, &check_updates, &quit])?;

    let open_main_id = open_main.id().clone();
    let show_quick_capture_id = show_quick_capture.id().clone();
    let check_updates_id = check_updates.id().clone();
    let quit_id = quit.id().clone();

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Work Notes")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            if event.id() == &open_main_id {
                let _ = show_main_window(app);
            } else if event.id() == &show_quick_capture_id {
                let _ = show_quick_capture_window(app);
            } else if event.id() == &check_updates_id {
                let _ = show_main_window(app);
                if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = window.emit(CHECK_FOR_UPDATES_EVENT, ());
                }
            } else if event.id() == &quit_id {
                app.exit(0);
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}
```

- [ ] **Step 2: Listen for the event in `+page.svelte`**

In `onMount`, inside the same `if (currentWindowLabel !== "quick-capture") { ... }` block, after the silent check added in Task 5, add:

```ts
      void listen(CHECK_FOR_UPDATES_EVENT, () => {
        void runUpdateCheck(createTauriUpdaterPort(), { silent: false });
      }).then(registerUnlisten);
```

(`listen` and `registerUnlisten` already exist in this scope.)

- [ ] **Step 3: Verify it compiles and types check**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: builds successfully.

Run: `npm run check`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/windowing/tray.rs src/routes/+page.svelte
git commit -m "feat: tray 'Check for updates' triggers a visible update check"
```

---

### Task 7: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: the CI secrets from Task 3; the updater/bundle config from Task 4.
- Produces: on a `v*` tag push, a **draft** GitHub Release containing the NSIS `-setup.exe`, its `.sig`, and `latest.json`.

- [ ] **Step 1: Create the workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - "v*"
  workflow_dispatch:

jobs:
  release:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: windows-latest
            args: ""
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: npm

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: "./src-tauri -> target"

      - name: Install frontend dependencies
        run: npm ci

      - name: Build and release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Work Notes ${{ github.ref_name }}"
          releaseBody: "See the assets to download and install this version."
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```

Notes for the implementer:
- `tauri-action` auto-generates and uploads `latest.json` because `createUpdaterArtifacts` is `true` and the signing env vars are present.
- The release is a **draft**; the `releases/latest/download/latest.json` permalink only resolves after it is published and marked "Latest".

- [ ] **Step 2: Validate the YAML**

Run: `python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('yaml ok')"`
Expected: `yaml ok` (confirms the workflow parses). If `python`/PyYAML is unavailable, visually confirm the indentation instead — GitHub also validates the workflow on push.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: build, sign, and draft-release via tauri-action on v* tags"
```

---

### Task 8: Release runbook + end-to-end verification

**Files:**
- Create: `RELEASING.md`

**Interfaces:**
- Consumes: everything above (Tasks 3, 4, 7 especially).
- Produces: a documented release process and the two-release verification of auto-update.

- [ ] **Step 1: Create `RELEASING.md`**

Create `RELEASING.md`:

```markdown
# Releasing Work Notes

Prerequisites (one-time): the updater signing keypair exists, its public key is
in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`), and the CI secrets
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are set.

## Cut a release

1. Bump the version to the same value in all three files:
   - `src-tauri/tauri.conf.json` (`version`) — source of truth
   - `src-tauri/Cargo.toml` (`package.version`)
   - `package.json` (`version`)
2. Commit: `git commit -am "chore: release vX.Y.Z"`
3. Tag and push:
   ```
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
4. GitHub Actions builds, signs, and creates a **draft** release with the NSIS
   installer, its `.sig`, and `latest.json`.
5. Open the draft release on GitHub, confirm the three assets are attached, and
   click **Publish**. Publishing makes it the "Latest" release, which activates
   the updater permalink for existing installs.

## Notes

- Installed apps check for updates on launch and via the tray "Check for
  updates" item. They read
  `https://github.com/sniffle6/work-notes/releases/latest/download/latest.json`.
- The installer is currently unsigned (no code-signing cert), so first-time
  installs show a one-time Windows SmartScreen warning. This is expected until
  a certificate is added.
```

- [ ] **Step 2: Confirm versions are in sync**

Run:
```bash
grep -m1 '"version"' src-tauri/tauri.conf.json
grep -m1 '^version' src-tauri/Cargo.toml
grep -m1 '"version"' package.json
```
Expected: all three show the same version (`0.1.0` unless already bumped).

- [ ] **Step 3: Commit**

```bash
git add RELEASING.md
git commit -m "docs: release runbook"
```

- [ ] **Step 4: End-to-end auto-update verification (manual, requires Tasks 3, 4, 7 complete)**

This proves the updater works and cannot be shortcut with a single build:

1. Ensure version is `0.1.0`. Tag `v0.1.0`, push, let CI draft the release, then **Publish** it.
2. Download the published `Work Notes_0.1.0_x64-setup.exe`, install it, and launch it.
3. Bump the version to `0.1.1` in the three files (Step 1 above). Tag `v0.1.1`, push, let CI draft, then **Publish**.
4. Launch the installed `0.1.0`. Confirm it: detects `0.1.1`, shows the "Install & restart?" prompt, installs (passive UI), and relaunches into `0.1.1`.
5. On the now-`0.1.1` install, click the tray **Check for updates** → confirm the "You're up to date." message.
6. Disconnect the network and launch the app → confirm it starts normally with no error dialog (silent check swallows the failure).

Record the result. If step 4 fails signature verification, the `pubkey` in config does not match the CI signing key — regenerate/realign (Task 3) and re-release.

---

## Notes on execution order

- Tasks 1, 2, 5, 6 are frontend/TS and fully automatable.
- **Task 3 is developer-performed** and must be done before Task 4 (needs the public key) and before Task 7's first real run (needs the secrets).
- Task 4 depends on Task 3. Task 6 depends on Task 5. Task 8's Step 4 depends on Tasks 3, 4, 7.
