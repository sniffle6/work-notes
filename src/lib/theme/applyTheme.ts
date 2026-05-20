import { tokenCssVariables, type ThemeDefinition } from "./tokens";

export type ThemeCssVariables = Record<`--color-${string}`, string>;

export function toCssVariables(theme: ThemeDefinition): ThemeCssVariables {
  return Object.entries(theme.tokens).reduce<ThemeCssVariables>((variables, [token, value]) => {
    const cssVariable = tokenCssVariables[token as keyof typeof tokenCssVariables];
    variables[cssVariable] = value;
    return variables;
  }, {});
}

export function applyTheme(theme: ThemeDefinition, target: HTMLElement = document.documentElement) {
  const variables = toCssVariables(theme);

  for (const [name, value] of Object.entries(variables)) {
    target.style.setProperty(name, value);
  }
}
