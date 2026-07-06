/** A pending update the user can choose to install. */
export interface UpdateHandle {
  version: string;
  downloadAndInstall: () => Promise<void>;
  close: () => Promise<void>;
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
  let update: UpdateHandle | null = null;
  let confirmedInstall = false;
  try {
    update = await port.check();
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
      try {
        await update.close();
      } catch {
        // best-effort resource cleanup
      }
      return { kind: "declined" };
    }

    confirmedInstall = true;
    await update.downloadAndInstall();
    await port.relaunch();
    return { kind: "installing" };
  } catch (error) {
    if (update) {
      try {
        await update.close();
      } catch {
        // best-effort resource cleanup
      }
    }
    if (!opts.silent) {
      port.notify(
        confirmedInstall
          ? "Update failed to install. Please try again later."
          : "Update check failed. Please try again later.",
      );
    }
    return { kind: "error", error };
  }
}

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
      void import("@tauri-apps/plugin-dialog")
        .then(({ message: showMessage }) => showMessage(message, { title: "Work Notes" }))
        .catch(() => {});
    },
  };
}
