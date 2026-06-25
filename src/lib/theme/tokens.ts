export type ThemeToken =
  | "app.bg"
  | "surface.1"
  | "surface.2"
  | "surface.input"
  | "border.default"
  | "border.strong"
  | "text.primary"
  | "text.muted"
  | "accent.primary"
  | "accent.hot"
  | "status.success"
  | "status.warning"
  | "status.error";

export type ThemeMode = "dark" | "light";

export type ThemeDefinition = {
  id: string;
  label: string;
  family: string;
  mode: ThemeMode;
  compact: boolean;
  tokens: Record<ThemeToken, string>;
};

export const tokenCssVariables: Record<ThemeToken, `--color-${string}`> = {
  "app.bg": "--color-app-bg",
  "surface.1": "--color-surface-1",
  "surface.2": "--color-surface-2",
  "surface.input": "--color-surface-input",
  "border.default": "--color-border-default",
  "border.strong": "--color-border-strong",
  "text.primary": "--color-text-primary",
  "text.muted": "--color-text-muted",
  "accent.primary": "--color-accent-primary",
  "accent.hot": "--color-accent-hot",
  "status.success": "--color-status-success",
  "status.warning": "--color-status-warning",
  "status.error": "--color-status-error",
};
