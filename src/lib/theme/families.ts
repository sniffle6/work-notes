import { getThemeById } from "./themes";
import type { ThemeDefinition, ThemeMode } from "./tokens";

export type ThemeFamily = {
  family: string;
  label: string;
  dark?: ThemeDefinition;
  light?: ThemeDefinition;
};

export function groupThemeFamilies(themes: readonly ThemeDefinition[]): ThemeFamily[] {
  const families: ThemeFamily[] = [];
  const byKey = new Map<string, ThemeFamily>();

  for (const theme of themes) {
    let family = byKey.get(theme.family);
    if (!family) {
      family = { family: theme.family, label: familyLabel(theme.label) };
      byKey.set(theme.family, family);
      families.push(family);
    }
    family[theme.mode] = theme;
  }

  for (const family of families) {
    const variantCount = (family.dark ? 1 : 0) + (family.light ? 1 : 0);
    if (variantCount === 1) {
      family.label = (family.dark ?? family.light)!.label;
    }
  }

  return families;
}

export function resolveVariant(family: ThemeFamily, mode: ThemeMode): ThemeDefinition {
  return (family[mode] ?? family.dark ?? family.light)!;
}

export function modeOf(themeId: string | null | undefined): ThemeMode {
  return getThemeById(themeId).mode;
}

export function familyOf(themeId: string | null | undefined): string {
  return getThemeById(themeId).family;
}

function familyLabel(variantLabel: string): string {
  return variantLabel.replace(/ (Dark|Light)$/u, "");
}
