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
  return {
    version: "1.2.3",
    downloadAndInstall: vi.fn(async () => {}),
    close: vi.fn(async () => {}),
  };
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

  it("closes the update when the user declines", async () => {
    const update = createUpdate();
    const port = createPort({ check: vi.fn(async () => update), confirm: vi.fn(async () => false) });
    const outcome = await runUpdateCheck(port, { silent: false });
    expect(update.close).toHaveBeenCalledOnce();
    expect(outcome).toEqual({ kind: "declined" });
  });

  it("closes the update when install throws after confirm", async () => {
    const update = createUpdate();
    update.downloadAndInstall = vi.fn(async () => {
      throw new Error("install failed");
    });
    const port = createPort({ check: vi.fn(async () => update), confirm: vi.fn(async () => true) });
    const outcome = await runUpdateCheck(port, { silent: true });
    expect(update.close).toHaveBeenCalledOnce();
    expect(outcome.kind).toBe("error");
  });

  it("reports an install-failure message (not a check-failure message) when install throws after confirm and not silent", async () => {
    const update = createUpdate();
    update.downloadAndInstall = vi.fn(async () => {
      throw new Error("install failed");
    });
    const port = createPort({ check: vi.fn(async () => update), confirm: vi.fn(async () => true) });
    await runUpdateCheck(port, { silent: false });
    expect(port.notify).toHaveBeenCalledWith("Update failed to install. Please try again later.");
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
