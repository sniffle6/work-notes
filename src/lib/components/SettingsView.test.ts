// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";

import type { AppSettings, CustomTheme } from "$lib/types";
import { selectLinkedWorkspaceDirectory } from "$lib/api";
import SettingsView from "./SettingsView.svelte";

vi.mock("$lib/api", () => ({
  selectLinkedWorkspaceDirectory: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(selectLinkedWorkspaceDirectory).mockReset();
});

afterEach(() => cleanup());

describe("SettingsView", () => {
  it("lists every theme with its mode and marks the one in use", () => {
    render(SettingsView, {
      props: { settings: { ...settings(), selectedTheme: "everforest-light" }, open: true },
    });

    const list = screen.getByRole("group", { name: "Theme" });
    expect(within(list).getAllByRole("button").map((button) => button.getAttribute("aria-label"))).toEqual([
      "Dark Compact",
      "Memphis '86",
      "Everforest Dark",
      "Everforest Light",
    ]);
    expect(screen.getByRole("button", { name: "Everforest Light" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Dark Compact" }).getAttribute("aria-pressed")).toBe("false");
    expect(within(screen.getByRole("button", { name: "Memphis '86" })).getByText("Light")).toBeTruthy();
  });

  it("applies a theme immediately when it is picked", async () => {
    const selectTheme = vi.fn();

    render(SettingsView, {
      props: { settings: settings(), open: true },
      events: { selectTheme },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Memphis '86" }));
    await fireEvent.click(screen.getByRole("button", { name: "Memphis '86" }));

    expect(selectTheme).toHaveBeenCalledTimes(1);
    expect(selectTheme.mock.calls[0][0].detail).toBe("memphis");
  });

  it("lists the user's own themes, picks them, and deletes them", async () => {
    const selectTheme = vi.fn();
    const deleteTheme = vi.fn();
    render(SettingsView, {
      props: { settings: { ...settings(), customThemes: [customTheme()] }, open: true },
      events: { selectTheme, deleteTheme },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Amber Terminal" }));
    await fireEvent.click(screen.getByRole("button", { name: "Delete Amber Terminal" }));

    expect(selectTheme.mock.calls[0][0].detail).toBe("custom-amber-terminal");
    expect(deleteTheme.mock.calls[0][0].detail).toBe("custom-amber-terminal");
    expect(screen.queryByRole("button", { name: "Delete Dark Compact" })).toBeNull();
  });

  it("sends a theme description to the theme agent from the button or Enter", async () => {
    const buildTheme = vi.fn();
    render(SettingsView, { props: { settings: settings(), open: true }, events: { buildTheme } });
    const box = screen.getByRole("textbox", { name: "Describe a theme" }) as HTMLTextAreaElement;
    const build = screen.getByRole("button", { name: "Build theme" }) as HTMLButtonElement;

    expect(build.disabled).toBe(true);
    await fireEvent.input(box, { target: { value: "  amber phosphor terminal  " } });
    await fireEvent.click(build);
    await fireEvent.input(box, { target: { value: "paper and blue ink" } });
    await fireEvent.keyDown(box, { key: "Enter", shiftKey: true });
    await fireEvent.keyDown(box, { key: "Enter" });

    expect(buildTheme.mock.calls.map((call) => call[0].detail)).toEqual([
      "amber phosphor terminal",
      "paper and blue ink",
    ]);
    expect(box.value).toBe("");
  });

  it("fills the box from a suggestion without sending it", async () => {
    const buildTheme = vi.fn();
    render(SettingsView, { props: { settings: settings(), open: true }, events: { buildTheme } });

    await fireEvent.click(screen.getByRole("button", { name: "Amber phosphor terminal" }));

    expect((screen.getByRole("textbox", { name: "Describe a theme" }) as HTMLTextAreaElement).value).toBe(
      "Amber phosphor terminal",
    );
    expect(buildTheme).not.toHaveBeenCalled();
  });

  it("shows progress and blocks a second request while the agent is building", async () => {
    const buildTheme = vi.fn();
    render(SettingsView, {
      props: { settings: settings(), open: true, themeBusy: true },
      events: { buildTheme },
    });
    const box = screen.getByRole("textbox", { name: "Describe a theme" });

    await fireEvent.input(box, { target: { value: "anything" } });
    await fireEvent.keyDown(box, { key: "Enter" });

    expect(screen.getByRole("status").textContent).toContain("Building theme");
    expect((screen.getByRole("button", { name: "Build theme" }) as HTMLButtonElement).disabled).toBe(true);
    expect(buildTheme).not.toHaveBeenCalled();
  });

  it("previews a built theme with its notes and warnings and lets it be kept or discarded", async () => {
    const keepTheme = vi.fn();
    const discardTheme = vi.fn();
    render(SettingsView, {
      props: {
        settings: settings(),
        open: true,
        themeDraft: {
          name: "Amber Terminal",
          mode: "dark",
          tokens: customTheme().tokens,
          notes: "Warm phosphor on brown.",
          warnings: ["textMuted on appBg has contrast 3.9; it needs at least 4.5"],
        },
      },
      events: { keepTheme, discardTheme },
    });
    const preview = screen.getByRole("group", { name: "Previewing Amber Terminal" });

    expect(within(preview).getByText("Warm phosphor on brown.")).toBeTruthy();
    expect(within(preview).getByText(/textMuted on appBg has contrast 3.9/)).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Describe a theme" }).getAttribute("placeholder")).toMatch(/change/i);
    await fireEvent.click(within(preview).getByRole("button", { name: "Keep theme" }));
    await fireEvent.click(within(preview).getByRole("button", { name: "Discard" }));

    expect(keepTheme).toHaveBeenCalledTimes(1);
    expect(discardTheme).toHaveBeenCalledTimes(1);
  });

  it("shows why the theme agent failed", () => {
    render(SettingsView, {
      props: { settings: settings(), open: true, themeError: "codex command was not found: codex.cmd" },
    });

    expect(screen.getByRole("alert").textContent).toContain("codex command was not found");
  });

  it("keeps the user's themes when settings are saved", async () => {
    const save = vi.fn();
    render(SettingsView, {
      props: { settings: { ...settings(), customThemes: [customTheme()] }, open: true },
      events: { save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(save.mock.calls[0][0].detail.customThemes).toEqual([customTheme()]);
  });

  it("shows a settings save error without closing the modal", () => {
    render(SettingsView, {
      props: {
        settings: settings(),
        open: true,
        error: "bad hotkey",
      },
    });

    expect(screen.getByText("bad hotkey")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save settings" })).toBeTruthy();
  });

  it("shows only real settings sections and controls", async () => {
    render(SettingsView, {
      props: { settings: settings(), open: true },
    });

    expect(screen.getByRole("button", { name: "Appearance" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Capture" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Parser" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "About" })).toBeTruthy();
    expect(screen.queryByText("Tags and people")).toBeNull();
    expect(screen.queryByText("Data and backup")).toBeNull();
    expect(screen.queryByText("Match system color scheme")).toBeNull();
    expect(screen.queryByText("Capture window position")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Capture" }));

    expect(screen.getByRole("heading", { name: "Capture" })).toBeTruthy();
    expect(screen.getByLabelText("Global hotkey")).toBeTruthy();
  });

  it("shows app version and update actions in the about section", async () => {
    render(SettingsView, {
      props: { settings: settings(), open: true, appVersion: "0.1.4" },
    });

    await fireEvent.click(screen.getByRole("button", { name: "About" }));

    expect(screen.getByRole("heading", { name: "About" })).toBeTruthy();
    expect(screen.getByText("0.1.4")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Check for updates" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "View release notes" }).getAttribute("href")).toBe(
      "https://github.com/sniffle6/work-notes/releases/latest",
    );
  });

  it("dispatches checkUpdates from the about section", async () => {
    const checkUpdates = vi.fn();

    render(SettingsView, {
      props: { settings: settings(), open: true, appVersion: "0.1.4" },
      events: { checkUpdates },
    });

    await fireEvent.click(screen.getByRole("button", { name: "About" }));
    await fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(checkUpdates).toHaveBeenCalledTimes(1);
  });

  it("saves linked repo and directory paths from the folder picker", async () => {
    const save = vi.fn();
    vi.mocked(selectLinkedWorkspaceDirectory).mockResolvedValue("D:\\work\\second");

    render(SettingsView, {
      props: {
        settings: {
          ...settings(),
          linkedWorkspacePaths: ["C:\\code\\first"],
        },
        open: true,
      },
      events: { save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Parser" }));
    await fireEvent.click(screen.getByRole("button", { name: "Add folder" }));
    await waitFor(() => expect(screen.getByText("D:\\work\\second")).toBeTruthy());
    await fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(save.mock.calls[0][0].detail.linkedWorkspacePaths).toEqual([
      "C:\\code\\first",
      "D:\\work\\second",
    ]);
  });

  it("saves Jira site, project, and filters and keeps the resolved cloud id", async () => {
    const save = vi.fn();

    render(SettingsView, {
      props: {
        settings: {
          ...settings(),
          jiraSiteUrl: "https://example.atlassian.net",
          jiraProjectKey: "AB",
          jiraCloudId: "cloud-1",
          jiraFilters: [{ name: "Mine", jql: "assignee = currentUser()" }],
        },
        open: true,
      },
      events: { save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Jira" }));
    expect((screen.getByLabelText("Jira site URL") as HTMLInputElement).value).toBe(
      "https://example.atlassian.net",
    );
    await fireEvent.input(screen.getByLabelText("Project key"), { target: { value: " cd " } });
    await fireEvent.click(screen.getByRole("button", { name: "Add filter" }));
    await fireEvent.input(screen.getByLabelText("Filter 2 name"), { target: { value: "Bugs" } });
    await fireEvent.input(screen.getByLabelText("Filter 2 JQL"), { target: { value: "type = Bug" } });
    await fireEvent.click(screen.getByRole("button", { name: "Remove filter Mine" }));
    await fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    const saved = save.mock.calls[0][0].detail as AppSettings;
    expect(saved.jiraSiteUrl).toBe("https://example.atlassian.net");
    expect(saved.jiraProjectKey).toBe("cd");
    expect(saved.jiraCloudId).toBe("cloud-1");
    expect(saved.jiraFilters).toEqual([{ name: "Bugs", jql: "type = Bug" }]);
  });

  it("keeps Jira settings when saving from another section", async () => {
    const save = vi.fn();

    render(SettingsView, {
      props: {
        settings: { ...settings(), jiraSiteUrl: "https://example.atlassian.net", jiraProjectKey: "AB" },
        open: true,
      },
      events: { save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    const saved = save.mock.calls[0][0].detail as AppSettings;
    expect(saved.jiraSiteUrl).toBe("https://example.atlassian.net");
    expect(saved.jiraProjectKey).toBe("AB");
    expect(saved.jiraFilters).toEqual([]);
  });

  it("removes linked repo and directory paths before saving", async () => {
    const save = vi.fn();

    render(SettingsView, {
      props: {
        settings: {
          ...settings(),
          linkedWorkspacePaths: ["C:\\code\\first", "D:\\work\\second"],
        },
        open: true,
      },
      events: { save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Parser" }));
    await fireEvent.click(screen.getByRole("button", { name: "Remove C:\\code\\first" }));
    await fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(save.mock.calls[0][0].detail.linkedWorkspacePaths).toEqual(["D:\\work\\second"]);
  });
});

function settings(): AppSettings {
  return {
    hotkey: "Ctrl+Shift+Space",
    parserTimeoutSeconds: 45,
    parserMaxRetries: 3,
    codexCommandPath: "codex.cmd",
    linkedWorkspacePaths: [],
    selectedTheme: "dark-compact",
    launchAtStartup: true,
    minimizeToTray: true,
  };
}

function customTheme(): CustomTheme {
  return {
    id: "custom-amber-terminal",
    label: "Amber Terminal",
    mode: "dark",
    tokens: {
      appBg: "#1c1410",
      surface1: "#241a14",
      surface2: "#2e2219",
      surfaceInput: "#17100c",
      borderDefault: "#4a382b",
      borderStrong: "#6b5240",
      textPrimary: "#f3e6d4",
      textMuted: "#b39b84",
      accentPrimary: "#a8591c",
      accentHot: "#ffb454",
      statusSuccess: "#8fbf6a",
      statusWarning: "#e6b450",
      statusError: "#e06c5a",
    },
  };
}
