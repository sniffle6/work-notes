import type { BackgroundGradient, CustomTheme, GeneratedTheme, ThemeTokenValues } from "$lib/types";

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

const MIN_GRADIENT_STOPS = 2;
const MAX_GRADIENT_STOPS = 3;
const MAX_GRADIENT_ANGLE = 360;
const HEX_COLOUR = /^#[0-9a-f]{6}$/i;

/**
 * Reads a gradient the backend sent. Returns `undefined`, meaning a flat
 * background, unless it has an angle from 0 to 360 and two or three
 * `#rrggbb` stops. The stops end up in a style attribute, so nothing but a
 * hex colour is let through.
 */
export function parseBackgroundGradient(value: unknown): BackgroundGradient | undefined {
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const { angle, stops } = value as { angle?: unknown; stops?: unknown };
  if (typeof angle !== "number" || !Number.isFinite(angle) || angle < 0 || angle > MAX_GRADIENT_ANGLE) {
    return undefined;
  }
  if (!Array.isArray(stops) || stops.length < MIN_GRADIENT_STOPS || stops.length > MAX_GRADIENT_STOPS) {
    return undefined;
  }
  if (!stops.every((stop): stop is string => typeof stop === "string" && HEX_COLOUR.test(stop))) {
    return undefined;
  }
  return { angle, stops: [...stops] };
}

type ThemeColours = Pick<CustomTheme, "mode" | "tokens" | "backgroundGradient">;

function toDefinition(id: string, label: string, theme: ThemeColours): ThemeDefinition {
  const tokens = {} as Record<ThemeToken, string>;
  for (const [key, token] of Object.entries(TOKEN_KEYS) as [keyof ThemeTokenValues, ThemeToken][]) {
    tokens[token] = theme.tokens[key];
  }
  // A saved theme is its own family: it has one mode and no sibling variant.
  const definition: ThemeDefinition = { id, label, family: id, mode: theme.mode, compact: true, tokens };
  if (theme.backgroundGradient) {
    definition.backgroundGradient = theme.backgroundGradient;
  }
  return definition;
}

/** Built-in themes followed by the user's saved themes. */
export function allThemes(customThemes: readonly CustomTheme[] = []): ThemeDefinition[] {
  return [...themes, ...customThemes.map((theme) => toDefinition(theme.id, theme.label, theme))];
}

/** The theme with this id, or the default theme when nothing matches. */
export function resolveTheme(
  id: string | null | undefined,
  customThemes: readonly CustomTheme[] = [],
): ThemeDefinition {
  const custom = customThemes.find((theme) => theme.id === id);
  return custom ? toDefinition(custom.id, custom.label, custom) : getThemeById(id);
}

/** A generated theme as something the app can paint while the user decides. */
export function draftThemeDefinition(theme: GeneratedTheme): ThemeDefinition {
  return toDefinition(DRAFT_THEME_ID, theme.name, theme);
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
  const custom: CustomTheme = {
    id: customThemeId(theme.name, takenIds),
    label: theme.name,
    mode: theme.mode,
    tokens: theme.tokens,
  };
  if (theme.backgroundGradient) {
    custom.backgroundGradient = theme.backgroundGradient;
  }
  return custom;
}
