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
