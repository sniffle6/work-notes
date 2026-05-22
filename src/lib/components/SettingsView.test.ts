// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import type { AppSettings } from "$lib/types";
import SettingsView from "./SettingsView.svelte";

afterEach(() => cleanup());

describe("SettingsView", () => {
  it("saves the Memphis theme from the appearance picker", async () => {
    const save = vi.fn();

    render(SettingsView, {
      props: { settings: settings(), open: true },
      events: { save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Memphis '86" }));
    await fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(save).toHaveBeenCalledTimes(1);
    expect(save.mock.calls[0][0].detail.selectedTheme).toBe("memphis");
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
});

function settings(): AppSettings {
  return {
    hotkey: "Ctrl+Shift+Space",
    parserTimeoutSeconds: 45,
    parserMaxRetries: 3,
    codexCommandPath: "codex.cmd",
    selectedTheme: "dark-compact",
    launchAtStartup: true,
    minimizeToTray: true,
  };
}
