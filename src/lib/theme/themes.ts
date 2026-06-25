import type { ThemeDefinition } from "./tokens";

export { toCssVariables } from "./applyTheme";

export const darkCompactTheme: ThemeDefinition = {
  id: "dark-compact",
  label: "Dark Compact",
  family: "dark-compact",
  mode: "dark",
  compact: true,
  tokens: {
    "app.bg": "#11151c",
    "surface.1": "#1b212b",
    "surface.2": "#202833",
    "surface.input": "#111720",
    "border.default": "#334052",
    "border.strong": "#4a5a70",
    "text.primary": "#edf4fb",
    "text.muted": "#93a3b7",
    "accent.primary": "#2f6f7a",
    "accent.hot": "#6fc7bd",
    "status.success": "#4f9f6f",
    "status.warning": "#d69a2d",
    "status.error": "#d05252",
  },
};

export const memphisTheme: ThemeDefinition = {
  id: "memphis",
  label: "Memphis '86",
  family: "memphis",
  mode: "light",
  compact: true,
  tokens: {
    "app.bg": "#f6f0d8",
    "surface.1": "#fffdf7",
    "surface.2": "#fff0bf",
    "surface.input": "#fffefa",
    "border.default": "#121315",
    "border.strong": "#050506",
    "text.primary": "#17151f",
    "text.muted": "#62556d",
    "accent.primary": "#ec4899",
    "accent.hot": "#49c5e3",
    "status.success": "#65cfa8",
    "status.warning": "#f4c736",
    "status.error": "#e0523e",
  },
};

export const everforestDarkTheme: ThemeDefinition = {
  id: "everforest-dark",
  label: "Everforest Dark",
  family: "everforest",
  mode: "dark",
  compact: true,
  tokens: {
    "app.bg": "#232a2e",
    "surface.1": "#2d353b",
    "surface.2": "#343f44",
    "surface.input": "#272e33",
    "border.default": "#4f585e",
    "border.strong": "#7a8478",
    "text.primary": "#d3c6aa",
    "text.muted": "#859289",
    "accent.primary": "#a7c080",
    "accent.hot": "#83c092",
    "status.success": "#a7c080",
    "status.warning": "#dbbc7f",
    "status.error": "#e67e80",
  },
};

export const everforestLightTheme: ThemeDefinition = {
  id: "everforest-light",
  label: "Everforest Light",
  family: "everforest",
  mode: "light",
  compact: true,
  tokens: {
    "app.bg": "#f2efdf",
    "surface.1": "#fdf6e3",
    "surface.2": "#efebd4",
    "surface.input": "#fffbef",
    "border.default": "#ddd8be",
    "border.strong": "#a6b0a0",
    "text.primary": "#5c6a72",
    "text.muted": "#626e66",
    "accent.primary": "#5c6b00",
    "accent.hot": "#35a77c",
    "status.success": "#8da101",
    "status.warning": "#dfa000",
    "status.error": "#cf2f2b",
  },
};

export const themes = [
  darkCompactTheme,
  memphisTheme,
  everforestDarkTheme,
  everforestLightTheme,
] as const;

export function getThemeById(id: string | null | undefined): ThemeDefinition {
  return themes.find((theme) => theme.id === id || (id === "dark" && theme.id === "dark-compact")) ?? darkCompactTheme;
}
