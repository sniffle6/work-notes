import { describe, expect, it } from "vitest";

import { darkCompactTheme, toCssVariables } from "./themes";

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
});
