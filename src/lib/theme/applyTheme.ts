import { tokenCssVariables, type ThemeDefinition } from "./tokens";

export type ThemeCssVariables = Record<`--color-${string}`, string>;

/**
 * What fills a pane that shows the app background. Unlike `--color-app-bg`
 * this may be a gradient, so it only works as a whole `background` value and
 * never inside `color-mix()`.
 */
export const APP_BG_PAINT_VARIABLE = "--color-app-bg-paint";

/** The app background as a CSS image or colour: the gradient when the theme has one, otherwise `app.bg`. */
export function appBackgroundCss(theme: ThemeDefinition): string {
  const gradient = theme.backgroundGradient;
  return gradient ? `linear-gradient(${gradient.angle}deg, ${gradient.stops.join(", ")})` : theme.tokens["app.bg"];
}

export function toCssVariables(theme: ThemeDefinition): ThemeCssVariables {
  const variables = Object.entries(theme.tokens).reduce<ThemeCssVariables>((variables, [token, value]) => {
    const cssVariable = tokenCssVariables[token as keyof typeof tokenCssVariables];
    variables[cssVariable] = value;
    return variables;
  }, {});

  // Each pane paints its own background. Pinning the gradient to the window
  // makes those separate paints line up as one continuous sweep.
  variables[APP_BG_PAINT_VARIABLE] = theme.backgroundGradient
    ? `${appBackgroundCss(theme)} fixed`
    : appBackgroundCss(theme);
  return variables;
}

export function applyTheme(theme: ThemeDefinition, target: HTMLElement = document.documentElement) {
  const variables = toCssVariables(theme);

  for (const [name, value] of Object.entries(variables)) {
    target.style.setProperty(name, value);
  }
}
