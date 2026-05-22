import { describe, expect, it } from "vitest";

import { darkCompactTheme, getThemeById, memphisTheme, toCssVariables } from "./themes";

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
