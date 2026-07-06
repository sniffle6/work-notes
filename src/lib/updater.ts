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
