import { describe, expect, it } from "vitest";

import {
  darkCompactTheme,
  duskTheme,
  everforestDarkTheme,
  everforestLightTheme,
  getThemeById,
  memphisTheme,
  themes,
  toCssVariables,
} from "./themes";

describe("toCssVariables", () => {
  it("maps the dark compact theme tokens to semantic CSS variables", () => {
    expect(toCssVariables(darkCompactTheme)).toMatchObject({
      "--color-app-bg": "#11151c",
      "--color-surface-1": "#1b212b",
      "--color-surface-2": "#202833",
      "--color-surface-input": "#111720",
      "--color-text-primary": "#edf4fb",
      "--color-accent-primary": "#2f6f7a",
    });
  });

  it("maps the Memphis theme and can resolve it by id", () => {
    expect(getThemeById("memphis")).toBe(memphisTheme);
    expect(toCssVariables(memphisTheme)).toMatchObject({
      "--color-app-bg": "#f6f0d8",
      "--color-surface-1": "#fffdf7",
      "--color-surface-2": "#fff0bf",
      "--color-surface-input": "#fffefa",
      "--color-border-default": "#121315",
      "--color-accent-primary": "#ec4899",
      "--color-accent-hot": "#49c5e3",
    });
  });
});

describe("app background paint", () => {
  it("paints the flat background colour when a theme has no gradient", () => {
    expect(toCssVariables(darkCompactTheme)["--color-app-bg-paint"]).toBe("#11151c");
  });

  it("paints a gradient pinned to the window so every pane shows one continuous sweep", () => {
    const variables = toCssVariables({
      ...darkCompactTheme,
      backgroundGradient: { angle: 135, stops: ["#11151c", "#1d2a3a", "#2a1d3a"] },
    });

    expect(variables["--color-app-bg-paint"]).toBe("linear-gradient(135deg, #11151c, #1d2a3a, #2a1d3a) fixed");
    expect(variables["--color-app-bg"]).toBe("#11151c");
  });
});

describe("dusk theme", () => {
  it("is a built-in dark theme that paints a three-stop gradient", () => {
    expect(themes).toContain(duskTheme);
    expect(getThemeById("dusk")).toBe(duskTheme);
    expect(duskTheme.mode).toBe("dark");
    expect(Object.keys(duskTheme.tokens).sort()).toEqual(Object.keys(darkCompactTheme.tokens).sort());
    expect(toCssVariables(duskTheme)["--color-app-bg-paint"]).toBe(
      "linear-gradient(155deg, #141a33, #241a36, #3a1f2a) fixed",
    );
  });

  it("takes its flat background colour from the middle of the gradient", () => {
    expect(duskTheme.tokens["app.bg"]).toBe(duskTheme.backgroundGradient?.stops[1]);
  });
});

describe("theme metadata", () => {
  it("tags existing themes with family and mode", () => {
    expect(darkCompactTheme.family).toBe("dark-compact");
    expect(darkCompactTheme.mode).toBe("dark");
    expect(memphisTheme.family).toBe("memphis");
    expect(memphisTheme.mode).toBe("light");
  });
});

describe("everforest themes", () => {
  it("registers dark and light everforest variants", () => {
    expect(themes).toContain(everforestDarkTheme);
    expect(themes).toContain(everforestLightTheme);
    expect(everforestDarkTheme.family).toBe("everforest");
    expect(everforestDarkTheme.mode).toBe("dark");
    expect(everforestLightTheme.mode).toBe("light");
  });

  it("defines all thirteen tokens for each everforest variant", () => {
    const tokenKeys = Object.keys(darkCompactTheme.tokens);
    expect(Object.keys(everforestDarkTheme.tokens).sort()).toEqual(tokenKeys.sort());
    expect(Object.keys(everforestLightTheme.tokens).sort()).toEqual(tokenKeys.sort());
  });

  it("resolves everforest ids and falls back for unknown ids", () => {
    expect(getThemeById("everforest-dark")).toBe(everforestDarkTheme);
    expect(getThemeById("everforest-light")).toBe(everforestLightTheme);
    expect(getThemeById("does-not-exist")).toBe(darkCompactTheme);
  });
});
