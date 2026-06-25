# Appearance Theme Variants Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Appearance theme selection apply and persist instantly on click, add an Everforest dark/light pair, redesign cards as mini app mockups, and add a Dark/Light toggle that locks to the variant a theme provides.

**Architecture:** Keep the flat `themes` list; add `family`/`mode` metadata to each `ThemeDefinition` and derive theme families in the UI. Persistence is unchanged — `AppSettings.selectedTheme` stays a single theme-id string saved through the existing `save_settings` command. A new store method `setTheme` updates the settings store optimistically (instant re-theme via the existing `+page` derived `currentTheme`) then persists, reverting on error. The Appearance UI dispatches a `selectTheme` event that `+page` routes to `setTheme`.

**Tech Stack:** Svelte 5 (runes), TypeScript, Vitest, Testing Library Svelte, `@lucide/svelte` icons.

## Global Constraints

- Frontend only. No Rust/Tauri/backend changes. Persistence reuses the existing `save_settings` command and the `selectedTheme: string` field on `AppSettings`.
- TDD: write the failing test first, watch it fail, implement minimally, watch it pass.
- Components consume semantic theme CSS variables only; no palette hex values hardcoded in component CSS. Card preview colors are injected as inline `--preview-*` custom properties (existing `themePreviewStyle` pattern).
- Theme ids are stable strings: `dark-compact`, `memphis`, `everforest-dark`, `everforest-light`.
- Everforest contrast level is **medium** only.
- Follow the existing lucide import style: `import Name from "@lucide/svelte/icons/<icon>";`.
- **Repo rule:** Do not commit during execution unless the user explicitly asks. This plan intentionally omits commit steps; each task ends at a green test run.

---

## File Map

- Modify `src/lib/theme/tokens.ts`: add `ThemeMode` and `family`/`mode` fields to `ThemeDefinition`.
- Modify `src/lib/theme/themes.ts`: tag existing themes with `family`/`mode`; add Everforest dark/light themes; export them in `themes`.
- Modify `src/lib/theme/theme.test.ts`: assert metadata and Everforest presence/tokens/resolution.
- Create `src/lib/theme/families.ts`: pure family grouping/resolution helpers.
- Create `src/lib/theme/families.test.ts`: helper unit tests.
- Modify `src/lib/stores/inbox.ts`: add `setTheme(themeId)` with optimistic apply + persist + revert; expose it.
- Modify `src/lib/stores/inbox.test.ts`: cover optimistic apply, persist-from-last-saved, revert-on-error.
- Modify `src/lib/components/SettingsView.svelte`: derive families, render Dark/Light toggle with lock, render mini-mockup family cards, dispatch `selectTheme`.
- Modify `src/lib/components/SettingsView.test.ts`: replace the old save-based theme test with click-applies and toggle-lock tests.
- Modify `src/routes/+page.svelte`: handle `selectTheme` by calling `workNotes.setTheme`.

---

### Task 1: Theme model metadata

**Files:**
- Modify: `src/lib/theme/tokens.ts`
- Modify: `src/lib/theme/themes.ts`
- Modify: `src/lib/theme/theme.test.ts`

**Interfaces:**
- Produces: `ThemeMode = "dark" | "light"`; `ThemeDefinition` gains `family: string` and `mode: ThemeMode`. Existing exports `darkCompactTheme` (family `dark-compact`, mode `dark`), `memphisTheme` (family `memphis`, mode `light`) keep their ids and tokens.

- [ ] **Step 1: Write the failing test**

In `src/lib/theme/theme.test.ts`, update the import line and append a new describe block:

```ts
import { darkCompactTheme, getThemeById, memphisTheme, toCssVariables } from "./themes";
```

```ts
describe("theme metadata", () => {
  it("tags existing themes with family and mode", () => {
    expect(darkCompactTheme.family).toBe("dark-compact");
    expect(darkCompactTheme.mode).toBe("dark");
    expect(memphisTheme.family).toBe("memphis");
    expect(memphisTheme.mode).toBe("light");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- theme`
Expected: FAIL — `family`/`mode` are `undefined` (or a type error during transform).

- [ ] **Step 3: Add the fields to the type**

In `src/lib/theme/tokens.ts`, add the mode type and extend `ThemeDefinition`:

```ts
export type ThemeMode = "dark" | "light";

export type ThemeDefinition = {
  id: string;
  label: string;
  family: string;
  mode: ThemeMode;
  compact: boolean;
  tokens: Record<ThemeToken, string>;
};
```

- [ ] **Step 4: Tag the existing themes**

In `src/lib/theme/themes.ts`, add `family`/`mode` to both existing themes (insert after the `label` line of each):

```ts
export const darkCompactTheme: ThemeDefinition = {
  id: "dark-compact",
  label: "Dark Compact",
  family: "dark-compact",
  mode: "dark",
  compact: true,
  tokens: {
    // unchanged
  },
};
```

```ts
export const memphisTheme: ThemeDefinition = {
  id: "memphis",
  label: "Memphis '86",
  family: "memphis",
  mode: "light",
  compact: true,
  tokens: {
    // unchanged
  },
};
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `npm test -- theme`
Expected: PASS (the pre-existing `toCssVariables` tests stay green; the new metadata test passes).

---

### Task 2: Everforest themes

**Files:**
- Modify: `src/lib/theme/themes.ts`
- Modify: `src/lib/theme/theme.test.ts`

**Interfaces:**
- Produces: `everforestDarkTheme` (id `everforest-dark`, family `everforest`, mode `dark`) and `everforestLightTheme` (id `everforest-light`, family `everforest`, mode `light`); both appended to `themes`. `getThemeById("everforest-dark"|"everforest-light")` resolves them.

- [ ] **Step 1: Write the failing test**

In `src/lib/theme/theme.test.ts`, extend the import and add a describe block:

```ts
import {
  darkCompactTheme,
  everforestDarkTheme,
  everforestLightTheme,
  getThemeById,
  memphisTheme,
  themes,
  toCssVariables,
} from "./themes";
```

```ts
describe("everforest themes", () => {
  it("registers dark and light everforest variants", () => {
    expect(themes).toContain(everforestDarkTheme);
    expect(themes).toContain(everforestLightTheme);
    expect(everforestDarkTheme.family).toBe("everforest");
    expect(everforestDarkTheme.mode).toBe("dark");
    expect(everforestLightTheme.mode).toBe("light");
  });

  it("defines all thirteen tokens for each everforest variant", () => {
    const tokenKeys = Object.keys(darkCompactTheme.tokens);
    expect(Object.keys(everforestDarkTheme.tokens).sort()).toEqual(tokenKeys.sort());
    expect(Object.keys(everforestLightTheme.tokens).sort()).toEqual(tokenKeys.sort());
  });

  it("resolves everforest ids and falls back for unknown ids", () => {
    expect(getThemeById("everforest-dark")).toBe(everforestDarkTheme);
    expect(getThemeById("everforest-light")).toBe(everforestLightTheme);
    expect(getThemeById("does-not-exist")).toBe(darkCompactTheme);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- theme`
Expected: FAIL — `everforestDarkTheme`/`everforestLightTheme` are not exported.

- [ ] **Step 3: Add the Everforest themes**

In `src/lib/theme/themes.ts`, add both themes after `memphisTheme` and before the `themes` array:

```ts
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
    "text.muted": "#939f91",
    "accent.primary": "#8da101",
    "accent.hot": "#35a77c",
    "status.success": "#8da101",
    "status.warning": "#dfa000",
    "status.error": "#f85552",
  },
};
```

- [ ] **Step 4: Add them to the `themes` array**

In `src/lib/theme/themes.ts`, replace the `themes` export:

```ts
export const themes = [
  darkCompactTheme,
  memphisTheme,
  everforestDarkTheme,
  everforestLightTheme,
] as const;
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `npm test -- theme`
Expected: PASS.

---

### Task 3: Pure family helpers

**Files:**
- Create: `src/lib/theme/families.ts`
- Create: `src/lib/theme/families.test.ts`

**Interfaces:**
- Produces:
  - `type ThemeFamily = { family: string; label: string; dark?: ThemeDefinition; light?: ThemeDefinition }`
  - `groupThemeFamilies(themes: readonly ThemeDefinition[]): ThemeFamily[]`
  - `resolveVariant(family: ThemeFamily, mode: ThemeMode): ThemeDefinition`
  - `modeOf(themeId: string | null | undefined): ThemeMode`
  - `familyOf(themeId: string | null | undefined): string`

- [ ] **Step 1: Write the failing test**

Create `src/lib/theme/families.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import {
  familyOf,
  groupThemeFamilies,
  modeOf,
  resolveVariant,
  type ThemeFamily,
} from "./families";
import {
  darkCompactTheme,
  everforestDarkTheme,
  everforestLightTheme,
  memphisTheme,
} from "./themes";

describe("groupThemeFamilies", () => {
  it("groups variants under one family and preserves order", () => {
    const families = groupThemeFamilies([
      darkCompactTheme,
      memphisTheme,
      everforestDarkTheme,
      everforestLightTheme,
    ]);

    expect(families.map((family) => family.family)).toEqual([
      "dark-compact",
      "memphis",
      "everforest",
    ]);

    const everforest = families.find((family) => family.family === "everforest") as ThemeFamily;
    expect(everforest.label).toBe("Everforest");
    expect(everforest.dark).toBe(everforestDarkTheme);
    expect(everforest.light).toBe(everforestLightTheme);
  });

  it("keeps the full variant label for single-variant families", () => {
    const families = groupThemeFamilies([darkCompactTheme, memphisTheme]);
    expect(families.find((family) => family.family === "dark-compact")?.label).toBe("Dark Compact");
    expect(families.find((family) => family.family === "memphis")?.label).toBe("Memphis '86");
  });
});

describe("resolveVariant", () => {
  const everforest = groupThemeFamilies([everforestDarkTheme, everforestLightTheme])[0];
  const darkOnly = groupThemeFamilies([darkCompactTheme])[0];

  it("returns the requested mode when present", () => {
    expect(resolveVariant(everforest, "light")).toBe(everforestLightTheme);
    expect(resolveVariant(everforest, "dark")).toBe(everforestDarkTheme);
  });

  it("falls back to the only available variant when the mode is missing", () => {
    expect(resolveVariant(darkOnly, "light")).toBe(darkCompactTheme);
  });
});

describe("modeOf / familyOf", () => {
  it("reads metadata for known ids", () => {
    expect(modeOf("everforest-light")).toBe("light");
    expect(familyOf("everforest-light")).toBe("everforest");
    expect(modeOf("memphis")).toBe("light");
    expect(familyOf("dark-compact")).toBe("dark-compact");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- families`
Expected: FAIL — `./families` module does not exist.

- [ ] **Step 3: Create the helpers**

Create `src/lib/theme/families.ts`:

```ts
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
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npm test -- families`
Expected: PASS.

---

### Task 4: Store `setTheme`

**Files:**
- Modify: `src/lib/stores/inbox.ts`
- Modify: `src/lib/stores/inbox.test.ts`

**Interfaces:**
- Consumes: existing `settings` writable, injected `api.saveSettings`, `error` writable, `errorMessage` helper, `get` from `svelte/store`.
- Produces: `setTheme(themeId: string): Promise<void>` on the store return object.

- [ ] **Step 1: Write the failing test**

In `src/lib/stores/inbox.test.ts`, add three tests inside the existing top-level `describe("createWorkNotesStore", ...)` block (near the other settings tests around line 390):

```ts
  it("applies a theme optimistically before persistence resolves", async () => {
    let resolveSave: (value: AppSettings) => void = () => {};
    const api = testApi({
      getSettings: vi.fn().mockResolvedValue(settings()),
      saveSettings: vi.fn(
        () => new Promise<AppSettings>((resolve) => (resolveSave = resolve)),
      ),
    });
    const store = createWorkNotesStore(api);
    await store.loadSettings();

    const pending = store.setTheme("everforest-dark");
    expect(get(store.settings)?.selectedTheme).toBe("everforest-dark");

    resolveSave({ ...settings(), selectedTheme: "everforest-dark" });
    await pending;
    expect(get(store.settings)?.selectedTheme).toBe("everforest-dark");
  });

  it("persists the last saved settings merged with the new theme", async () => {
    const api = testApi({
      getSettings: vi.fn().mockResolvedValue({ ...settings(), codexCommandPath: "codex.cmd" }),
      saveSettings: vi.fn((next: AppSettings) => Promise.resolve(next)),
    });
    const store = createWorkNotesStore(api);
    await store.loadSettings();

    await store.setTheme("everforest-light");

    expect(api.saveSettings).toHaveBeenCalledWith({
      ...settings(),
      codexCommandPath: "codex.cmd",
      selectedTheme: "everforest-light",
    });
  });

  it("reverts the theme and reports an error when saving fails", async () => {
    const api = testApi({
      getSettings: vi.fn().mockResolvedValue(settings()),
      saveSettings: vi.fn().mockRejectedValue(new Error("disk full")),
    });
    const store = createWorkNotesStore(api);
    await store.loadSettings();

    await store.setTheme("everforest-dark");

    expect(get(store.settings)?.selectedTheme).toBe("dark-compact");
    expect(get(store.error)).toBe("disk full");
  });
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- inbox`
Expected: FAIL — `store.setTheme` is not a function.

- [ ] **Step 3: Add the `setTheme` method**

In `src/lib/stores/inbox.ts`, add this function next to `persistSettings` (after it, before the store return):

```ts
  async function setTheme(themeId: string): Promise<void> {
    const previous = get(settings);
    if (!previous || previous.selectedTheme === themeId) {
      return;
    }

    const next = { ...previous, selectedTheme: themeId };
    settings.set(next);
    error.set(null);

    try {
      settings.set(await api.saveSettings(next));
    } catch (unknownError) {
      settings.set(previous);
      error.set(errorMessage(unknownError, "Could not change theme."));
    }
  }
```

- [ ] **Step 4: Expose `setTheme` from the store**

In `src/lib/stores/inbox.ts`, add `setTheme` to the returned object (next to `loadSettings`/`persistSettings`):

```ts
    setTheme,
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `npm test -- inbox`
Expected: PASS.

---

### Task 5: Appearance UI redesign

**Files:**
- Modify: `src/lib/components/SettingsView.svelte`
- Modify: `src/lib/components/SettingsView.test.ts`

**Interfaces:**
- Consumes: `themes`, `groupThemeFamilies`, `resolveVariant`, `modeOf`, `familyOf`, `ThemeFamily`.
- Produces: a new component event `selectTheme: string` (the resolved theme id), dispatched on card click and on enabled Dark/Light button click. The existing `save` event is unchanged.

- [ ] **Step 1: Write the failing tests**

In `src/lib/components/SettingsView.test.ts`, replace the existing `it("saves the Memphis theme from the appearance picker", ...)` test (lines 21-34) with these three tests:

```ts
  it("applies a theme immediately on card click", async () => {
    const selectTheme = vi.fn();

    render(SettingsView, {
      props: { settings: settings(), open: true },
      events: { selectTheme },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Memphis '86" }));

    expect(selectTheme).toHaveBeenCalledTimes(1);
    expect(selectTheme.mock.calls[0][0].detail).toBe("memphis");
  });

  it("toggles between dark and light variants of the selected theme", async () => {
    const selectTheme = vi.fn();

    render(SettingsView, {
      props: { settings: { ...settings(), selectedTheme: "everforest-dark" }, open: true },
      events: { selectTheme },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Light" }));

    expect(selectTheme.mock.calls[0][0].detail).toBe("everforest-light");
  });

  it("locks the variant toggle to what a single-variant theme provides", () => {
    render(SettingsView, {
      props: { settings: { ...settings(), selectedTheme: "dark-compact" }, open: true },
    });

    expect((screen.getByRole("button", { name: "Light" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Dark" }) as HTMLButtonElement).disabled).toBe(false);
  });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npm test -- SettingsView`
Expected: FAIL — no `selectTheme` event / no Dark/Light buttons.

- [ ] **Step 3: Update the component script**

In `src/lib/components/SettingsView.svelte`, add imports near the other lucide/theme imports:

```ts
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import { groupThemeFamilies, resolveVariant, modeOf, familyOf, type ThemeFamily } from "$lib/theme/families";
```

Add `selectTheme` to the dispatcher type (extend the existing `createEventDispatcher` call):

```ts
  const dispatch = createEventDispatcher<{
    save: AppSettings;
    close: void;
    selectTheme: string;
  }>();
```

Add derived family state and click handlers (place after `themePreviewStyle`, inside `<script>`):

```ts
  const families = groupThemeFamilies(themes);
  let selectedMode = $derived(modeOf(selectedTheme));
  let selectedFamily = $derived(familyOf(selectedTheme));
  let selectedFamilyObject = $derived(families.find((family) => family.family === selectedFamily));

  function applyTheme(themeId: string): void {
    if (themeId === selectedTheme) {
      return;
    }
    selectedTheme = themeId;
    dispatch("selectTheme", themeId);
  }

  function selectFamily(family: ThemeFamily): void {
    applyTheme(resolveVariant(family, selectedMode).id);
  }

  function applyVariant(mode: "dark" | "light"): void {
    const target = selectedFamilyObject?.[mode];
    if (target) {
      applyTheme(target.id);
    }
  }
```

- [ ] **Step 4: Replace the appearance markup**

In `src/lib/components/SettingsView.svelte`, replace the whole `<div class="theme-grid" ...>...</div>` block (the `{#each themes as theme}` grid, currently lines 183-210) with a Dark/Light toggle plus a family-card grid:

```svelte
              <div class="variant-toggle" role="group" aria-label="Theme mode">
                <button
                  class:active={selectedMode === "dark"}
                  type="button"
                  aria-pressed={selectedMode === "dark"}
                  disabled={!selectedFamilyObject?.dark}
                  onclick={() => applyVariant("dark")}
                >
                  <Moon size={14} strokeWidth={2.2} aria-hidden="true" />
                  <span>Dark</span>
                </button>
                <button
                  class:active={selectedMode === "light"}
                  type="button"
                  aria-pressed={selectedMode === "light"}
                  disabled={!selectedFamilyObject?.light}
                  onclick={() => applyVariant("light")}
                >
                  <Sun size={14} strokeWidth={2.2} aria-hidden="true" />
                  <span>Light</span>
                </button>
              </div>

              <div class="theme-grid" aria-label="Theme">
                {#each families as family}
                  {@const previewVariant = resolveVariant(family, selectedMode)}
                  <button
                    class:active={family.family === selectedFamily}
                    class="theme-card"
                    type="button"
                    aria-label={family.label}
                    aria-pressed={family.family === selectedFamily}
                    onclick={() => selectFamily(family)}
                  >
                    <span class="theme-preview" style={themePreviewStyle(previewVariant)}>
                      <span class="preview-sidebar">
                        <span class="preview-nav"></span>
                        <span class="preview-nav"></span>
                        <span class="preview-nav"></span>
                      </span>
                      <span class="preview-content">
                        <span class="preview-note">
                          <span class="preview-line strong"></span>
                          <span class="preview-line"></span>
                          <span class="preview-line short"></span>
                          <span class="preview-button">Accent</span>
                        </span>
                      </span>
                    </span>
                    <span class="theme-meta">
                      <strong>{family.label}</strong>
                      {#if family.family === selectedFamily}
                        <span class="selected-badge"><Check size={13} strokeWidth={2.5} />In use</span>
                      {/if}
                    </span>
                  </button>
                {/each}
              </div>
```

- [ ] **Step 5: Add toggle and preview-note styles**

In `src/lib/components/SettingsView.svelte`, add the following to the `<style>` block (after the existing `.theme-grid` rule):

```css
  .variant-toggle {
    display: inline-flex;
    gap: 4px;
    margin-bottom: 12px;
    padding: 3px;
    border: 1px solid var(--color-border-default);
    border-radius: 8px;
    background: var(--color-surface-1);
  }

  .variant-toggle button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-muted);
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }

  .variant-toggle button.active {
    background: var(--color-surface-2);
    color: var(--color-text-primary);
  }

  .variant-toggle button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
```

Then add note-card preview styles after the existing `.preview-content` rule:

```css
  .preview-nav {
    height: 8px;
    margin: 8px 8px 0;
    border-radius: 999px;
    background: var(--preview-muted);
  }

  .preview-note {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--preview-border);
    border-radius: 6px;
    background: var(--preview-surface-2);
  }

  .preview-button {
    align-self: flex-start;
    margin-top: 2px;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--preview-accent);
    color: var(--preview-bg);
    font-size: 9px;
    font-weight: 800;
  }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `npm test -- SettingsView`
Expected: PASS (the three new theme tests plus the unchanged section/workspace tests).

---

### Task 6: Wire `selectTheme` in the page

**Files:**
- Modify: `src/routes/+page.svelte`

**Interfaces:**
- Consumes: `workNotes.setTheme` (Task 4) and the `SettingsView` `selectTheme` event (Task 5).

- [ ] **Step 1: Add the event handler**

In `src/routes/+page.svelte`, add a handler next to `saveSettings` (after it, around line 204):

```ts
  function selectTheme(event: CustomEvent<string>) {
    void workNotes.setTheme(event.detail);
  }
```

- [ ] **Step 2: Bind the handler on the SettingsView**

In `src/routes/+page.svelte`, add the `on:selectTheme` binding to the main-window `<SettingsView>` (next to `on:save`, around line 418):

```svelte
      on:save={(event) => void saveSettings(event)}
      on:selectTheme={selectTheme}
      on:close={() => (settingsOpen = false)}
```

- [ ] **Step 3: Type-check and run the full frontend suite**

Run: `npm run check`
Expected: no new type errors.

Run: `npm test`
Expected: PASS — all suites (theme, families, inbox, SettingsView, and the rest) green.

---

### Task 7: Verify

**Files:** none (verification only).

- [ ] **Step 1: Run the full frontend suite**

Run: `npm test`
Expected: all test files pass.

- [ ] **Step 2: Manual verification (dev app)**

Run: `npm run tauri dev` (use the VsDevCmd wrapper if the linker path is missing — see `docs/development.md`).

Confirm:
- Open Settings -> Appearance. Click Everforest: the whole app re-themes immediately, no Save press.
- Toggle Light/Dark on Everforest: app and the Everforest card switch variants.
- Select Dark Compact: the Light button is disabled. Select Memphis '86: the Dark button is disabled.
- Close and reopen the app: the last-clicked theme is still applied.
- Edit the Capture hotkey field but do not Save, then click a theme: the hotkey edit is not committed (reopen Settings and confirm the old hotkey).
```
