// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { getNavSummary, listFollowups, listInbox, listSuggestedActions } from "./api";
import { buildNavSummary } from "./nav-summary";
import { createInboxFilters } from "./stores/filters";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  vi.clearAllMocks();
});

describe("getNavSummary", () => {
  it("reads the native summary and sorts tag names for display", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    vi.mocked(invoke).mockResolvedValue({
      inbox: 4,
      needsReview: 3,
      followups: 2,
      parseFailed: 1,
      parseQueue: 5,
      tags: ["zeta", "Apple", "maya"],
    });

    const summary = await getNavSummary();

    expect(invoke).toHaveBeenCalledWith("get_nav_summary", undefined);
    expect(summary).toEqual({
      inbox: 4,
      needsReview: 3,
      followups: 2,
      parseFailed: 1,
      parseQueue: 5,
      tags: ["Apple", "maya", "zeta"],
    });
  });

  it("counts the browser preview data the same way the preview lists show it", async () => {
    const [notes, actions, followups] = await Promise.all([
      listInbox(createInboxFilters({ limit: 1000 })),
      listSuggestedActions(1000),
      listFollowups(1000),
    ]);

    expect(await getNavSummary()).toEqual(buildNavSummary(notes, actions, followups));
  });
});
