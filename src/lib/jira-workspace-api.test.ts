// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listJiraChatMessages, listJiraChatThreads, listJiraPendingProposals, loadJiraWriting, saveJiraWriting, resolveJiraChatProposal } from "./api";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  localStorage.clear();
  vi.clearAllMocks();
});

describe("Jira workspace API", () => {
  it("offers a sample comparison and resolves it only inside the browser preview", async () => {
    const threads = await listJiraChatThreads();
    const example = threads.find((thread) => thread.title === "Proposal review example")!;
    expect(example).toBeTruthy();
    const pending = await listJiraChatMessages(example.id);
    const proposal = pending.find((message) => message.role === "proposal")!;
    expect(proposal.status).toBe("pending");
    expect(await listJiraPendingProposals()).toContainEqual(proposal);
    const resolved = await resolveJiraChatProposal(proposal.id, true);
    expect(resolved.find((message) => message.id === proposal.id)?.result).toContain("Preview only");
    expect(await listJiraPendingProposals()).not.toContainEqual(proposal);
  });
  it("passes saved comparisons through the native API without exposing payloads", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    const review = { title: "Update AB-7", capturedAt: "2026-11-16T12:00:00Z", warning: null,
      fields: [{ label: "Description", before: "Old text", after: "" }] };
    vi.mocked(invoke).mockResolvedValue([{ id: "p1", threadId: "t1", role: "proposal", status: "pending",
      content: "Update AB-7", result: null, review, payload: "private", createdAt: "today" }]);
    const [message] = await listJiraChatMessages("t1");
    expect(message.review).toEqual(review);
    expect(message).not.toHaveProperty("payload");
    const [queued] = await listJiraPendingProposals();
    expect(invoke).toHaveBeenLastCalledWith("list_jira_pending_proposals", undefined);
    expect(queued.review).toEqual(review);
    expect(queued).not.toHaveProperty("payload");
  });

  it("keeps browser preview writing across loads and isolates projects", async () => {
    const writing = { draft: { issueType: "Bug", summary: "Keep", description: "Draft" }, tickets: {} };
    await saveJiraWriting("preview/AB", writing);
    expect(await loadJiraWriting("preview/AB")).toEqual(writing);
    expect(await loadJiraWriting("preview/OTHER")).toEqual({ draft: null, tickets: {} });
  });
});
