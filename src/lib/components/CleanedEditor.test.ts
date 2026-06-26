// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import CleanedEditor from "./CleanedEditor.svelte";

afterEach(() => cleanup());

describe("CleanedEditor", () => {
  it("emits save with the edited title, summary, and body", async () => {
    const save = vi.fn();
    render(CleanedEditor, {
      props: { title: "Old title", summary: "Old summary", cleanedText: "Old body" },
      events: { save },
    });

    await fireEvent.input(screen.getByLabelText("Edit title"), { target: { value: "New title" } });
    await fireEvent.input(screen.getByLabelText("Edit summary"), { target: { value: "New summary" } });
    await fireEvent.input(screen.getByLabelText("Edit cleaned note"), { target: { value: "New body" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(save).toHaveBeenCalledTimes(1);
    expect(save.mock.calls[0][0].detail).toEqual({
      title: "New title",
      summary: "New summary",
      cleanedText: "New body",
    });
  });

  it("emits cancel without saving", async () => {
    const cancel = vi.fn();
    const save = vi.fn();
    render(CleanedEditor, {
      props: { title: "T", summary: "S", cleanedText: "B" },
      events: { cancel, save },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(cancel).toHaveBeenCalledTimes(1);
    expect(save).not.toHaveBeenCalled();
  });
});
