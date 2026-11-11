// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";

import type { JiraIssueDetail as JiraIssue } from "$lib/types";
import JiraIssueDetail from "./JiraIssueDetail.svelte";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function issue(overrides: Partial<JiraIssue> = {}): JiraIssue {
  return {
    key: "AB-7",
    summary: "Printer jams on reboot",
    status: "To Do",
    statusCategory: "new",
    issueType: "Bug",
    priority: "High",
    assignee: null,
    updated: "2026-11-01T10:00:00.000Z",
    labels: ["printer"],
    url: "https://example.atlassian.net/browse/AB-7",
    description: "## Context\n\nJams after reboot.",
    descriptionEditableLosslessly: true,
    reporter: "Rina Q",
    parentKey: null,
    subtasks: [],
    linkedIssues: [],
    attachments: [],
    comments: [{ id: "c1", author: "Maya Lopez", created: "2026-11-02T09:00:00.000Z", body: "Reproduced on bench 1." }],
    transitions: [
      { id: "11", name: "Start work", toStatus: "In Progress" },
      { id: "21", name: "Dev Complete", toStatus: "Dev Complete" },
    ],
    ...overrides,
  };
}

const ATTACHMENTS: JiraIssue["attachments"] = [
  {
    id: "10001",
    filename: "bench.log",
    mimeType: "text/plain",
    size: 2048,
    author: "Maya Lopez",
    created: "2026-11-02T09:00:00.000Z",
    hasThumbnail: false,
  },
  { id: "10002", filename: "photo.png", mimeType: null, size: 0, author: null, created: null, hasThumbnail: true },
];
const BENCH_LOG_PATH = "C:\\Downloads\\Work Notes\\AB-7\\bench.log";

describe("JiraIssueDetail", () => {
  describe("attachments", () => {
    it("shows no attachments section for a ticket without any", () => {
      render(JiraIssueDetail, { props: { issue: issue() } });

      expect(screen.queryByRole("region", { name: "Attachments" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Download all" })).toBeNull();
    });

    it("lists each attachment with its size and author", () => {
      render(JiraIssueDetail, { props: { issue: issue({ attachments: ATTACHMENTS }) } });

      expect(screen.getByRole("heading", { name: "Attachments (2)" })).toBeTruthy();
      expect(screen.getByText("bench.log")).toBeTruthy();
      expect(screen.getByText(/2\.0 KB · Maya Lopez/)).toBeTruthy();
      expect(screen.getByText("photo.png")).toBeTruthy();
    });

    it("asks for one attachment or all of them", async () => {
      const downloadAttachments = vi.fn();
      render(JiraIssueDetail, {
        props: { issue: issue({ attachments: ATTACHMENTS }) },
        events: { downloadAttachments },
      });

      await fireEvent.click(screen.getByRole("button", { name: "Download photo.png" }));
      await fireEvent.click(screen.getByRole("button", { name: "Download all" }));

      expect(downloadAttachments.mock.calls.map((call) => call[0].detail)).toEqual(["10002", null]);
    });

    it("offers to show a downloaded file instead of downloading it again", async () => {
      const revealAttachment = vi.fn();
      render(JiraIssueDetail, {
        props: {
          issue: issue({ attachments: ATTACHMENTS }),
          attachmentDownload: {
            folder: "C:\\Downloads\\Work Notes\\AB-7",
            files: [{ id: "10001", filename: "bench.log", path: BENCH_LOG_PATH }],
          },
        },
        events: { revealAttachment },
      });

      expect(screen.queryByRole("button", { name: "Download bench.log" })).toBeNull();
      expect(screen.getByRole("button", { name: "Download photo.png" })).toBeTruthy();

      await fireEvent.click(screen.getByRole("button", { name: "Show bench.log in folder" }));
      await fireEvent.click(screen.getByRole("button", { name: "Show folder" }));

      expect(revealAttachment.mock.calls.map((call) => call[0].detail)).toEqual([BENCH_LOG_PATH, BENCH_LOG_PATH]);
    });

    it("shows a loaded thumbnail, and clicking it asks to open the attachment in the app", async () => {
      const downloadAttachments = vi.fn();
      const openAttachment = vi.fn();
      render(JiraIssueDetail, {
        props: {
          issue: issue({ attachments: ATTACHMENTS }),
          attachmentThumbnails: { "10002": "blob:thumbnail-10002" },
        },
        events: { downloadAttachments, openAttachment },
      });

      const preview = screen.getByRole("button", { name: "Open photo.png" });
      expect(preview.querySelector("img")?.getAttribute("src")).toBe("blob:thumbnail-10002");
      expect(screen.queryByRole("button", { name: "Open bench.log" })).toBeNull();
      expect(screen.queryByRole("dialog")).toBeNull();

      await fireEvent.click(preview);

      expect(openAttachment.mock.calls.map((call) => call[0].detail)).toEqual(["10002"]);
      expect(downloadAttachments).not.toHaveBeenCalled();
    });

    it("offers to play a video that has no thumbnail", async () => {
      const openAttachment = vi.fn();
      const video = { ...ATTACHMENTS[0], id: "10003", filename: "repro.mp4", mimeType: "video/mp4" };
      render(JiraIssueDetail, {
        props: { issue: issue({ attachments: [...ATTACHMENTS, video] }) },
        events: { openAttachment },
      });

      expect(screen.queryByRole("button", { name: "Open bench.log" })).toBeNull();
      await fireEvent.click(screen.getByRole("button", { name: "Open repro.mp4" }));

      expect(openAttachment.mock.calls.map((call) => call[0].detail)).toEqual(["10003"]);
    });

    it("plays a video in the viewer, without the image zoom controls", () => {
      render(JiraIssueDetail, {
        props: {
          issue: issue({ attachments: ATTACHMENTS }),
          attachmentViewer: {
            attachmentId: "10003",
            filename: "repro.mp4",
            url: "blob:video-10003",
            kind: "video",
            path: "C:\\Downloads\\Work Notes\\AB-7\\repro.mp4",
          },
        },
      });
      const dialog = screen.getByRole("dialog", { name: "repro.mp4" });

      const player = dialog.querySelector("video");
      expect(player?.getAttribute("src")).toBe("blob:video-10003");
      expect(player?.controls).toBe(true);
      expect(within(dialog).queryByRole("img")).toBeNull();
      expect(within(dialog).queryByRole("button", { name: "Zoom in" })).toBeNull();
      expect(within(dialog).getByRole("button", { name: "Show in folder" })).toBeTruthy();
    });

    it("disables the thumbnail being opened", () => {
      render(JiraIssueDetail, {
        props: {
          issue: issue({ attachments: ATTACHMENTS }),
          attachmentThumbnails: { "10002": "blob:thumbnail-10002" },
          openingAttachmentId: "10002",
        },
      });

      expect((screen.getByRole("button", { name: "Open photo.png" }) as HTMLButtonElement).disabled).toBe(true);
    });

    describe("viewer", () => {
      const photoPath = "C:\\Downloads\\Work Notes\\AB-7\\photo.png";

      function renderViewer(events: Record<string, (event: CustomEvent) => void>) {
        render(JiraIssueDetail, {
          props: {
            issue: issue({ attachments: ATTACHMENTS }),
            attachmentViewer: {
              attachmentId: "10002",
              filename: "photo.png",
              url: "blob:image-10002",
              kind: "image",
              path: photoPath,
            },
          },
          events,
        });
        return screen.getByRole("dialog", { name: "photo.png" });
      }

      it("shows the full image in the app", () => {
        const dialog = renderViewer({});

        expect(within(dialog).getByRole("img", { name: "photo.png" }).getAttribute("src")).toBe("blob:image-10002");
      });

      function zoomText(dialog: HTMLElement): string {
        return within(dialog).getByRole("button", { name: "Fit to window" }).textContent?.trim() ?? "";
      }

      it("starts fitted to the window and zooms with the buttons", async () => {
        const dialog = renderViewer({});
        expect(zoomText(dialog)).toBe("Fit");

        await fireEvent.click(within(dialog).getByRole("button", { name: "Zoom in" }));
        expect(zoomText(dialog)).toBe("125%");

        await fireEvent.click(within(dialog).getByRole("button", { name: "Zoom out" }));
        await fireEvent.click(within(dialog).getByRole("button", { name: "Zoom out" }));
        expect(zoomText(dialog)).toBe("80%");

        await fireEvent.click(within(dialog).getByRole("button", { name: "Fit to window" }));
        expect(zoomText(dialog)).toBe("Fit");
      });

      it("sizes the image from its real width once zoomed", async () => {
        const dialog = renderViewer({});
        const image = within(dialog).getByRole("img", { name: "photo.png" }) as HTMLImageElement;
        Object.defineProperty(image, "naturalWidth", { value: 400 });
        await fireEvent.load(image);
        expect(image.style.width).toBe("");

        await fireEvent.dblClick(image);
        expect(zoomText(dialog)).toBe("100%");
        expect(image.style.width).toBe("400px");

        await fireEvent.click(within(dialog).getByRole("button", { name: "Zoom in" }));
        expect(image.style.width).toBe("500px");

        await fireEvent.dblClick(image);
        expect(zoomText(dialog)).toBe("Fit");
        expect(image.style.width).toBe("");
      });

      it("zooms with the keyboard and with Ctrl and the mouse wheel", async () => {
        const dialog = renderViewer({});
        const image = within(dialog).getByRole("img", { name: "photo.png" });

        await fireEvent.keyDown(window, { key: "+" });
        expect(zoomText(dialog)).toBe("125%");
        await fireEvent.keyDown(window, { key: "-" });
        expect(zoomText(dialog)).toBe("100%");
        await fireEvent.keyDown(window, { key: "0" });
        expect(zoomText(dialog)).toBe("Fit");

        await fireEvent.wheel(image, { ctrlKey: true, deltaY: -100 });
        expect(zoomText(dialog)).toBe("125%");
        await fireEvent.wheel(image, { ctrlKey: true, deltaY: 100 });
        expect(zoomText(dialog)).toBe("100%");
        // Without Ctrl the wheel scrolls the zoomed image instead.
        await fireEvent.wheel(image, { deltaY: 100 });
        expect(zoomText(dialog)).toBe("100%");
      });

      it("keeps the point under the cursor in place when zooming with the wheel", async () => {
        const dialog = renderViewer({});
        const image = within(dialog).getByRole("img", { name: "photo.png" }) as HTMLImageElement;
        const stage = image.parentElement as HTMLElement;
        Object.defineProperty(image, "naturalWidth", { value: 400 });
        // A 2:1 image whose top-left corner stays at (100, 50) in the window.
        image.getBoundingClientRect = () => {
          const width = Number.parseFloat(image.style.width) || 400;
          return new DOMRect(100, 50, width, width / 2);
        };
        await fireEvent.load(image);
        await fireEvent.dblClick(image);
        expect(image.style.width).toBe("400px");

        // The cursor is over the image's centre, which moves by (50, 25) at 125%.
        await fireEvent.wheel(image, { ctrlKey: true, deltaY: -100, clientX: 300, clientY: 150 });
        await vi.waitFor(() => expect(stage.scrollLeft).toBe(50));
        expect(image.style.width).toBe("500px");
        expect(stage.scrollTop).toBe(25);
      });

      it("pans by dragging, and the drag does not close the viewer", async () => {
        const closeAttachmentViewer = vi.fn();
        const dialog = renderViewer({ closeAttachmentViewer });
        const image = within(dialog).getByRole("img", { name: "photo.png" });
        const stage = image.parentElement as HTMLElement;
        stage.scrollLeft = 100;
        stage.scrollTop = 50;

        await fireEvent.mouseDown(image, { button: 0, clientX: 200, clientY: 200 });
        await fireEvent.mouseMove(window, { clientX: 170, clientY: 180 });
        expect(stage.scrollLeft).toBe(130);
        expect(stage.scrollTop).toBe(70);

        await fireEvent.mouseUp(window, { clientX: 170, clientY: 180 });
        await fireEvent.click(stage);
        expect(closeAttachmentViewer).not.toHaveBeenCalled();

        // Moving the mouse after the button is released no longer pans.
        await fireEvent.mouseMove(window, { clientX: 0, clientY: 0 });
        expect(stage.scrollLeft).toBe(130);

        await fireEvent.mouseDown(stage, { button: 0, clientX: 10, clientY: 10 });
        await fireEvent.mouseUp(window, { clientX: 10, clientY: 10 });
        await fireEvent.click(stage);
        expect(closeAttachmentViewer).toHaveBeenCalledTimes(1);
      });

      it("goes to the folder the image was downloaded to", async () => {
        const revealAttachment = vi.fn();
        const dialog = renderViewer({ revealAttachment });

        await fireEvent.click(within(dialog).getByRole("button", { name: "Show in folder" }));

        expect(revealAttachment.mock.calls.map((call) => call[0].detail)).toEqual([photoPath]);
      });

      it("closes from the close button, the Escape key, and a click outside the image", async () => {
        const closeAttachmentViewer = vi.fn();
        const dialog = renderViewer({ closeAttachmentViewer });

        await fireEvent.click(within(dialog).getByRole("button", { name: "Close" }));
        await fireEvent.keyDown(window, { key: "Escape" });
        await fireEvent.click(dialog);
        await fireEvent.click(within(dialog).getByRole("img", { name: "photo.png" }));

        expect(closeAttachmentViewer).toHaveBeenCalledTimes(3);
      });
    });

    it("blocks further downloads while one is running", () => {
      render(JiraIssueDetail, {
        props: { issue: issue({ attachments: ATTACHMENTS }), downloadingAttachments: true },
      });

      expect((screen.getByRole("button", { name: "Downloading" }) as HTMLButtonElement).disabled).toBe(true);
      expect((screen.getByRole("button", { name: "Download photo.png" }) as HTMLButtonElement).disabled).toBe(true);
    });

    it("copies the prompt it is given for the open ticket", async () => {
      const writeText = vi.fn(async () => undefined);
      vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
      const preparePrompt = vi.fn(async () => "prompt with attachment paths");
      const open = issue({ attachments: ATTACHMENTS });
      render(JiraIssueDetail, { props: { issue: open, preparePrompt } });

      await fireEvent.click(screen.getByRole("button", { name: "Copy agent prompt" }));

      await vi.waitFor(() => expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy());
      expect(preparePrompt).toHaveBeenCalledWith(open);
      expect(writeText).toHaveBeenCalledWith("prompt with attachment paths");
    });
  });

  it("prompts for a selection when no ticket is open", () => {
    render(JiraIssueDetail, { props: { issue: null } });

    expect(screen.getByText("Select a ticket")).toBeTruthy();
  });

  it("shows the ticket with a link to Jira, its description, and comments", () => {
    render(JiraIssueDetail, { props: { issue: issue() } });

    const link = screen.getByRole("link", { name: "AB-7" });
    expect(link.getAttribute("href")).toBe("https://example.atlassian.net/browse/AB-7");
    expect(screen.getByRole("heading", { name: "Printer jams on reboot" })).toBeTruthy();
    expect(screen.getByText("To Do")).toBeTruthy();
    expect(screen.getByText("Unassigned")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Context" })).toBeTruthy();
    expect(screen.getByText("Reproduced on bench 1.")).toBeTruthy();
    expect(screen.getByText("Maya Lopez")).toBeTruthy();
  });

  it("offers each available transition and dispatches its id", async () => {
    const transition = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { transition } });

    await fireEvent.click(screen.getByRole("button", { name: "Move to Dev Complete" }));

    expect(screen.getByRole("button", { name: "Move to In Progress" })).toBeTruthy();
    expect(transition.mock.calls[0][0].detail).toBe("21");
  });

  it("copies a prompt for the open ticket and confirms it", async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
    render(JiraIssueDetail, { props: { issue: issue() } });

    await fireEvent.click(screen.getByRole("button", { name: "Copy agent prompt" }));

    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy());
    const prompt = (writeText.mock.calls[0] as unknown as [string])[0];
    expect(prompt).toContain("Work on Jira ticket AB-7: Printer jams on reboot");
    expect(prompt).toContain("Jams after reboot.");
    expect(prompt).toContain("Reproduced on bench 1.");
  });

  it("says so when the prompt cannot be copied", async () => {
    vi.stubGlobal("navigator", {
      ...navigator,
      clipboard: {
        writeText: vi.fn(async () => {
          throw new Error("denied");
        }),
      },
    });
    render(JiraIssueDetail, { props: { issue: issue() } });

    await fireEvent.click(screen.getByRole("button", { name: "Copy agent prompt" }));

    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Copy failed" })).toBeTruthy());
  });

  it("disables ticket changes while a change is being saved", () => {
    render(JiraIssueDetail, { props: { issue: issue(), saving: true } });

    expect((screen.getByRole("button", { name: "Move to In Progress" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Assign to me" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Edit" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("dispatches assign to me", async () => {
    const assignToMe = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { assignToMe } });

    await fireEvent.click(screen.getByRole("button", { name: "Assign to me" }));

    expect(assignToMe).toHaveBeenCalledTimes(1);
  });

  it("sends a trimmed comment and clears the box once it is saved", async () => {
    const comment = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { comment } });
    const box = screen.getByRole("textbox", { name: "Comment" }) as HTMLTextAreaElement;
    const add = screen.getByRole("button", { name: "Add comment" }) as HTMLButtonElement;

    expect(add.disabled).toBe(true);
    await fireEvent.input(box, { target: { value: "  Verified on bench  " } });
    await fireEvent.click(add);

    const detail = comment.mock.calls[0][0].detail;
    expect(detail.text).toBe("Verified on bench");
    expect(box.value).toBe("  Verified on bench  ");
    detail.done();
    await vi.waitFor(() => expect(box.value).toBe(""));
  });

  it("saves only the fields that changed", async () => {
    const saveEdit = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { saveEdit } });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const save = screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    await fireEvent.input(screen.getByRole("textbox", { name: "Summary" }), {
      target: { value: "Printer jams after reboot" },
    });
    await fireEvent.click(save);

    const detail = saveEdit.mock.calls[0][0].detail;
    expect(detail.edit).toEqual({ summary: "Printer jams after reboot" });
    detail.done();
    await vi.waitFor(() => expect(screen.queryByRole("textbox", { name: "Summary" })).toBeNull());
  });

  it("requires confirmation before a description edit flattens Jira formatting", async () => {
    const saveEdit = vi.fn();
    render(JiraIssueDetail, {
      props: { issue: issue({ descriptionEditableLosslessly: false }) },
      events: { saveEdit },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    expect(screen.queryByRole("checkbox")).toBeNull();
    await fireEvent.input(screen.getByRole("textbox", { name: "Description" }), {
      target: { value: "Rewritten body" },
    });
    const save = screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    await fireEvent.click(screen.getByRole("checkbox", { name: /Replace the formatted description/ }));
    expect(save.disabled).toBe(false);
    await fireEvent.click(save);

    expect(saveEdit.mock.calls[0][0].detail.edit).toEqual({ description: "Rewritten body" });
  });

  it("cancels an edit without dispatching", async () => {
    const saveEdit = vi.fn();
    render(JiraIssueDetail, { props: { issue: issue() }, events: { saveEdit } });

    await fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await fireEvent.input(screen.getByRole("textbox", { name: "Summary" }), { target: { value: "Changed" } });
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(saveEdit).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Printer jams on reboot" })).toBeTruthy();
  });
});
