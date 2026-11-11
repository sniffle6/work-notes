import { get, writable } from "svelte/store";

import { generateTheme } from "$lib/api";
import type { GeneratedTheme } from "$lib/types";

export type ThemeBuilderApi = {
  generateTheme: typeof generateTheme;
};

/**
 * Workflow store for building a theme with the theme agent. It holds one
 * draft; each new request refines that draft until it is discarded. Keeping
 * a draft is the settings store's job.
 */
export function createThemeBuilderStore(api: ThemeBuilderApi = { generateTheme }) {
  const draft = writable<GeneratedTheme | null>(null);
  const busy = writable(false);
  const error = writable<string | null>(null);

  // Bumped on discard so an answer to an abandoned request is not shown.
  let generation = 0;

  /** Asks the agent for a theme, refining the current draft when there is one. */
  async function build(request: string): Promise<void> {
    const text = request.trim();
    if (!text || get(busy)) {
      return;
    }

    const startedIn = generation;
    busy.set(true);
    error.set(null);

    try {
      const theme = await api.generateTheme(text, get(draft));
      if (startedIn === generation) {
        draft.set(theme);
      }
    } catch (unknownError) {
      if (startedIn === generation) {
        error.set(errorMessage(unknownError));
      }
    } finally {
      busy.set(false);
    }
  }

  function discard(): void {
    generation += 1;
    draft.set(null);
    error.set(null);
  }

  return { draft, busy, error, build, discard };
}

export type ThemeBuilderStore = ReturnType<typeof createThemeBuilderStore>;

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") {
    return error.message;
  }
  return "The theme agent could not build a theme.";
}
