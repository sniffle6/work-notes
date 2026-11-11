import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

import type { GeneratedTheme } from "$lib/types";
import { createThemeBuilderStore, type ThemeBuilderApi } from "./theme-builder";

function theme(overrides: Partial<GeneratedTheme> = {}): GeneratedTheme {
  return {
    name: "Amber Terminal",
    mode: "dark",
    tokens: {
      appBg: "#1c1410",
      surface1: "#241a14",
      surface2: "#2e2219",
      surfaceInput: "#17100c",
      borderDefault: "#4a382b",
      borderStrong: "#6b5240",
      textPrimary: "#f3e6d4",
      textMuted: "#b39b84",
      accentPrimary: "#a8591c",
      accentHot: "#ffb454",
      statusSuccess: "#8fbf6a",
      statusWarning: "#e6b450",
      statusError: "#e06c5a",
    },
    notes: "Warm phosphor.",
    warnings: [],
    ...overrides,
  };
}

function fakeApi(overrides: Partial<ThemeBuilderApi> = {}): ThemeBuilderApi {
  return { generateTheme: vi.fn(async () => theme()), ...overrides };
}

describe("createThemeBuilderStore", () => {
  it("builds a first draft from a description", async () => {
    const api = fakeApi();
    const store = createThemeBuilderStore(api);

    await store.build("  amber terminal  ");

    expect(api.generateTheme).toHaveBeenCalledWith("amber terminal", null);
    expect(get(store.draft)).toEqual(theme());
    expect(get(store.busy)).toBe(false);
    expect(get(store.error)).toBeNull();
  });

  it("refines the current draft on the next request", async () => {
    const warmer = theme({ name: "Warmer Terminal" });
    const api = fakeApi({
      generateTheme: vi.fn().mockResolvedValueOnce(theme()).mockResolvedValueOnce(warmer),
    });
    const store = createThemeBuilderStore(api);

    await store.build("amber terminal");
    await store.build("warmer accent");

    expect(api.generateTheme).toHaveBeenLastCalledWith("warmer accent", theme());
    expect(get(store.draft)).toEqual(warmer);
  });

  it("keeps the last draft and reports the reason when a request fails", async () => {
    const api = fakeApi({
      generateTheme: vi
        .fn()
        .mockResolvedValueOnce(theme())
        .mockRejectedValueOnce(new Error("codex command was not found")),
    });
    const store = createThemeBuilderStore(api);

    await store.build("amber terminal");
    await store.build("warmer accent");

    expect(get(store.draft)).toEqual(theme());
    expect(get(store.error)).toBe("codex command was not found");
    expect(get(store.busy)).toBe(false);
  });

  it("ignores blank requests and a second request while one is running", async () => {
    let finish!: (value: GeneratedTheme) => void;
    const api = fakeApi({
      generateTheme: vi.fn(() => new Promise<GeneratedTheme>((resolve) => (finish = resolve))),
    });
    const store = createThemeBuilderStore(api);

    await store.build("   ");
    const first = store.build("amber terminal");
    await store.build("something else");

    expect(api.generateTheme).toHaveBeenCalledTimes(1);
    expect(get(store.busy)).toBe(true);
    finish(theme());
    await first;
    expect(get(store.busy)).toBe(false);
  });

  it("discards the draft and any error", async () => {
    const store = createThemeBuilderStore(fakeApi());
    await store.build("amber terminal");

    store.discard();

    expect(get(store.draft)).toBeNull();
    expect(get(store.error)).toBeNull();
  });

  it("drops a draft that arrives after it was discarded", async () => {
    let finish!: (value: GeneratedTheme) => void;
    const store = createThemeBuilderStore(
      fakeApi({ generateTheme: vi.fn(() => new Promise<GeneratedTheme>((resolve) => (finish = resolve))) }),
    );

    const pending = store.build("amber terminal");
    store.discard();
    finish(theme());
    await pending;

    expect(get(store.draft)).toBeNull();
    expect(get(store.busy)).toBe(false);
  });
});
