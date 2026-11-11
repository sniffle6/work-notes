import { describe, expect, it } from "vitest";

import type { CustomTheme, GeneratedTheme, ThemeTokenValues } from "$lib/types";
import { allThemes, customThemeId, draftThemeDefinition, DRAFT_THEME_ID, resolveTheme, toCustomTheme } from "./custom";
import { darkCompactTheme, everforestLightTheme, themes } from "./themes";

function tokens(overrides: Partial<ThemeTokenValues> = {}): ThemeTokenValues {
  return {
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
    ...overrides,
  };
}

function custom(overrides: Partial<CustomTheme> = {}): CustomTheme {
  return { id: "custom-amber-terminal", label: "Amber Terminal", mode: "dark", tokens: tokens(), ...overrides };
}

function generated(overrides: Partial<GeneratedTheme> = {}): GeneratedTheme {
  return { name: "Amber Terminal", mode: "dark", tokens: tokens(), notes: "Warm phosphor.", warnings: [], ...overrides };
}

describe("customThemeId", () => {
  it("builds a custom- id from the name and avoids ids already taken", () => {
    expect(customThemeId("Amber Terminal", [])).toBe("custom-amber-terminal");
    expect(customThemeId("  Café  NOIR!! ", [])).toBe("custom-caf-noir");
    expect(customThemeId("Amber Terminal", ["custom-amber-terminal"])).toBe("custom-amber-terminal-2");
    expect(customThemeId("Amber Terminal", ["custom-amber-terminal", "custom-amber-terminal-2"])).toBe(
      "custom-amber-terminal-3",
    );
    expect(customThemeId("!!!", [])).toBe("custom-theme");
  });

  it("keeps the id within the length the backend accepts", () => {
    const id = customThemeId("x".repeat(80), []);

    expect(id.length).toBeLessThanOrEqual("custom-".length + 48);
  });
});

describe("toCustomTheme", () => {
  it("turns a generated theme into a saved theme with a fresh id", () => {
    expect(toCustomTheme(generated(), ["custom-amber-terminal"])).toEqual(
      custom({ id: "custom-amber-terminal-2" }),
    );
  });
});

describe("theme definitions", () => {
  it("maps a saved theme's colours onto the app's theme tokens", () => {
    const [definition] = allThemes([custom()]).slice(themes.length);

    expect(definition).toMatchObject({
      id: "custom-amber-terminal",
      label: "Amber Terminal",
      family: "custom-amber-terminal",
      mode: "dark",
    });
    expect(definition.tokens).toEqual({
      "app.bg": "#1c1410",
      "surface.1": "#241a14",
      "surface.2": "#2e2219",
      "surface.input": "#17100c",
      "border.default": "#4a382b",
      "border.strong": "#6b5240",
      "text.primary": "#f3e6d4",
      "text.muted": "#b39b84",
      "accent.primary": "#a8591c",
      "accent.hot": "#ffb454",
      "status.success": "#8fbf6a",
      "status.warning": "#e6b450",
      "status.error": "#e06c5a",
    });
  });

  it("lists built-in themes before saved ones", () => {
    expect(allThemes([custom()]).map((theme) => theme.id)).toEqual([
      ...themes.map((theme) => theme.id),
      "custom-amber-terminal",
    ]);
  });

  it("previews a draft under a reserved id", () => {
    const draft = draftThemeDefinition(generated({ mode: "light" }));

    expect(draft.id).toBe(DRAFT_THEME_ID);
    expect(draft.label).toBe("Amber Terminal");
    expect(draft.mode).toBe("light");
    expect(draft.tokens["accent.hot"]).toBe("#ffb454");
  });
});

describe("resolveTheme", () => {
  it("finds built-in and saved themes and falls back to the default", () => {
    expect(resolveTheme("everforest-light", [custom()])).toBe(everforestLightTheme);
    expect(resolveTheme("custom-amber-terminal", [custom()]).label).toBe("Amber Terminal");
    expect(resolveTheme("custom-deleted", [custom()])).toBe(darkCompactTheme);
    expect(resolveTheme(undefined, [])).toBe(darkCompactTheme);
  });
});
