// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import QuickCapturePanel from "./QuickCapturePanel.svelte";

afterEach(() => cleanup());

describe("QuickCapturePanel", () => {
  it("dispatches input, save, and close from compact keyboard flows", async () => {
    const input = vi.fn();
    const save = vi.fn();
    const close = vi.fn();

    render(QuickCapturePanel, {
      props: { value: "initial note" },
      events: { input, save, close },
    });

    const textarea = screen.getByLabelText("Note text") as HTMLTextAreaElement;
    await fireEvent.input(textarea, { target: { value: "follow up Friday" } });
    await fireEvent.keyDown(textarea, { key: "Enter" });
    await fireEvent.keyDown(textarea, { key: "Escape" });

    expect(input).toHaveBeenCalledTimes(1);
    expect(input.mock.calls[0][0].detail).toBe("follow up Friday");
    expect(save).toHaveBeenCalledTimes(1);
    expect(save.mock.calls[0][0].detail).toBe("follow up Friday");
    expect(close).toHaveBeenCalledTimes(1);
  });

  it("does not dispatch save for blank notes", async () => {
    const save = vi.fn();

    render(QuickCapturePanel, {
      props: { value: "   " },
      events: { save },
    });

    const textarea = screen.getByLabelText("Note text") as HTMLTextAreaElement;
    await fireEvent.keyDown(textarea, { key: "Enter" });

    expect(save).not.toHaveBeenCalled();
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("lets shift enter keep native textarea newline behavior", () => {
    const save = vi.fn();

    render(QuickCapturePanel, {
      props: { value: "line one" },
      events: { save },
    });

    const textarea = screen.getByLabelText("Note text");
    const event = new KeyboardEvent("keydown", {
      key: "Enter",
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    });

    textarea.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(false);
    expect(save).not.toHaveBeenCalled();
  });
});
