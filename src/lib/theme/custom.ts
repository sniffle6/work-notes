import type { CustomTheme, GeneratedTheme, ThemeTokenValues } from "$lib/types";

import { getThemeById, themes } from "./themes";
import type { ThemeDefinition, ThemeToken } from "./tokens";

/** Id of a theme being previewed before the user keeps it. */
export const DRAFT_THEME_ID = "custom-draft";

const CUSTOM_ID_PREFIX = "custom-";
// Matches the limit the settings service enforces on the part after the prefix.
const MAX_SLUG_CHARS = 48;
// Room for a "-12" style suffix that keeps ids unique.
const MAX_BASE_SLUG_CHARS = MAX_SLUG_CHARS - 4;

const TOKEN_KEYS: Record<keyof ThemeTokenValues, ThemeToken> = {
  appBg: "app.bg",
  surface1: "surface.1",
  surface2: "surface.2",
  surfaceInput: "surface.input",
  borderDefault: "border.default",
  borderStrong: "border.strong",
  textPrimary: "text.primary",
  textMuted: "text.muted",
  accentPrimary: "accent.primary",
  accentHot: "accent.hot",
  statusSuccess: "status.success",
  statusWarning: "status.warning",
  statusError: "status.error",
};

function toDefinition(id: string, label: string, mode: ThemeDefinition["mode"], values: ThemeTokenValues): ThemeDefinition {
  const tokens = {} as Record<ThemeToken, string>;
  for (const [key, token] of Object.entries(TOKEN_KEYS) as [keyof ThemeTokenValues, ThemeToken][]) {
    tokens[token] = values[key];
  }
  // A saved theme is its own family: it has one mode and no sibling variant.
  return { id, label, family: id, mode, compact: true, tokens };
}

/** Built-in themes followed by the user's saved themes. */
export function allThemes(customThemes: readonly CustomTheme[] = []): ThemeDefinition[] {
  return [...themes, ...customThemes.map((theme) => toDefinition(theme.id, theme.label, theme.mode, theme.tokens))];
}

/** The theme with this id, or the default theme when nothing matches. */
export function resolveTheme(
  id: string | null | undefined,
  customThemes: readonly CustomTheme[] = [],
): ThemeDefinition {
  const custom = customThemes.find((theme) => theme.id === id);
  return custom ? toDefinition(custom.id, custom.label, custom.mode, custom.tokens) : getThemeById(id);
}

/** A generated theme as something the app can paint while the user decides. */
export function draftThemeDefinition(theme: GeneratedTheme): ThemeDefinition {
  return toDefinition(DRAFT_THEME_ID, theme.name, theme.mode, theme.tokens);
}

/** Builds a `custom-` id from a theme name that is not in `takenIds`. */
export function customThemeId(name: string, takenIds: readonly string[]): string {
  const slug =
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, MAX_BASE_SLUG_CHARS)
      .replace(/-+$/g, "") || "theme";

  const base = `${CUSTOM_ID_PREFIX}${slug}`;
  let id = base;
  for (let suffix = 2; takenIds.includes(id) || id === DRAFT_THEME_ID; suffix += 1) {
    id = `${base}-${suffix}`;
  }
  return id;
}

/** Turns a generated theme into a saved theme with an id that is free. */
export function toCustomTheme(theme: GeneratedTheme, takenIds: readonly string[]): CustomTheme {
  return {
    id: customThemeId(theme.name, takenIds),
    label: theme.name,
    mode: theme.mode,
    tokens: theme.tokens,
  };
}
