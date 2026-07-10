import { describe, expect, it } from "vitest";

import { getSettings, listFollowups, listSuggestedActions } from "./api";

describe("browser fallback settings", () => {
  it("matches native defaults for startup and tray settings", async () => {
    const settings = await getSettings();

    expect(settings.launchAtStartup).toBe(false);
    expect(settings.minimizeToTray).toBe(true);
  });
});

describe("browser fallback calendar data", () => {
  it("seeds dated suggested, open, and completed tasks", async () => {
    const [suggested, followups] = await Promise.all([
      listSuggestedActions(),
      listFollowups(),
    ]);
    const open = followups.filter((item) => item.status === "accepted");
    const done = followups.filter((item) => item.status === "done");

    expect(suggested).toHaveLength(2);
    expect(open).toHaveLength(3);
    expect(done).toHaveLength(3);
    expect([...suggested, ...open].every((item) => item.dueDate)).toBe(true);
    expect(done.every((item) => item.completedAt)).toBe(true);
    expect([...suggested, ...followups].every((item) => {
      const capturedAt = new Date(item.createdAt).getTime();
      const ageInDays = (Date.now() - capturedAt) / 86_400_000;
      return ageInDays >= 0 && ageInDays < 14;
    })).toBe(true);
  });
});
