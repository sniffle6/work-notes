import type { ThemeDefinition } from "./tokens";

export { toCssVariables } from "./applyTheme";

export const darkCompactTheme: ThemeDefinition = {
  id: "dark-compact",
  label: "Dark Compact",
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

export const themes = [darkCompactTheme, memphisTheme] as const;

export function getThemeById(id: string | null | undefined): ThemeDefinition {
  return themes.find((theme) => theme.id === id || (id === "dark" && theme.id === "dark-compact")) ?? darkCompactTheme;
}
