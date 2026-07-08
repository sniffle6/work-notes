import { describe, expect, it } from "vitest";

import { getSettings } from "./api";

describe("browser fallback settings", () => {
  it("matches native defaults for startup and tray settings", async () => {
    const settings = await getSettings();

    expect(settings.launchAtStartup).toBe(false);
    expect(settings.minimizeToTray).toBe(true);
  });
});
