import { describe, expect, it } from "vitest";

import {
  familyOf,
  groupThemeFamilies,
  modeOf,
  resolveVariant,
  type ThemeFamily,
} from "./families";
import {
  darkCompactTheme,
  everforestDarkTheme,
  everforestLightTheme,
  memphisTheme,
} from "./themes";

describe("groupThemeFamilies", () => {
  it("groups variants under one family and preserves order", () => {
    const families = groupThemeFamilies([
      darkCompactTheme,
      memphisTheme,
      everforestDarkTheme,
      everforestLightTheme,
    ]);

    expect(families.map((family) => family.family)).toEqual([
      "dark-compact",
      "memphis",
      "everforest",
    ]);

    const everforest = families.find((family) => family.family === "everforest") as ThemeFamily;
    expect(everforest.label).toBe("Everforest");
    expect(everforest.dark).toBe(everforestDarkTheme);
    expect(everforest.light).toBe(everforestLightTheme);
  });

  it("keeps the full variant label for single-variant families", () => {
    const families = groupThemeFamilies([darkCompactTheme, memphisTheme]);
    expect(families.find((family) => family.family === "dark-compact")?.label).toBe("Dark Compact");
    expect(families.find((family) => family.family === "memphis")?.label).toBe("Memphis '86");
  });
});

describe("resolveVariant", () => {
  const everforest = groupThemeFamilies([everforestDarkTheme, everforestLightTheme])[0];
  const darkOnly = groupThemeFamilies([darkCompactTheme])[0];

  it("returns the requested mode when present", () => {
    expect(resolveVariant(everforest, "light")).toBe(everforestLightTheme);
    expect(resolveVariant(everforest, "dark")).toBe(everforestDarkTheme);
  });

  it("falls back to the only available variant when the mode is missing", () => {
    expect(resolveVariant(darkOnly, "light")).toBe(darkCompactTheme);
  });
});

describe("modeOf / familyOf", () => {
  it("reads metadata for known ids", () => {
    expect(modeOf("everforest-light")).toBe("light");
    expect(familyOf("everforest-light")).toBe("everforest");
    expect(modeOf("memphis")).toBe("light");
    expect(familyOf("dark-compact")).toBe("dark-compact");
  });
});
