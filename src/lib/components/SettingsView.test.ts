// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";

import type { AppSettings } from "$lib/types";
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
    expect(screen.queryByText("Tags and people")).toBeNull();
    expect(screen.queryByText("Data and backup")).toBeNull();
    expect(screen.queryByText("Match system color scheme")).toBeNull();
    expect(screen.queryByText("Capture window position")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "Capture" }));

    expect(screen.getByRole("heading", { name: "Capture" })).toBeTruthy();
    expect(screen.getByLabelText("Global hotkey")).toBeTruthy();
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
