// Sample Jira data for running the frontend in a plain browser (no Tauri
// backend). Nothing here talks to Jira or to an agent.

type UnknownRecord = Record<string, unknown>;

const SITE_URL = "https://example.atlassian.net";

function sampleIssue(key: string, summary: string, status: string, statusCategory: string, issueType: string) {
  return {
    key,
    summary,
    status,
    statusCategory,
    issueType,
    priority: "Medium",
    assignee: null as string | null,
    updated: new Date().toISOString(),
    labels: [] as string[],
    url: `${SITE_URL}/browse/${key}`,
    description: "Sample ticket shown in the browser preview.\n\n- Jira is only reachable from the desktop app.",
    descriptionEditableLosslessly: true,
    reporter: "Preview User",
    parentKey: null as string | null,
    comments: [] as { id: string; author: string; created: string; body: string }[],
    transitions: [
      { id: "11", name: "Start work", toStatus: "In Progress" },
      { id: "21", name: "Done", toStatus: "Done" },
    ],
  };
}

let issues = [
  sampleIssue("DEMO-1", "Printer reports paper out after reboot", "To Do", "new", "Bug"),
  sampleIssue("DEMO-2", "Add meter export to the service menu", "In Progress", "indeterminate", "Story"),
];
let threads: { id: string; title: string; createdAt: string; updatedAt: string }[] = [];
let messages: UnknownRecord[] = [];
let sequence = 0;

function nextId(prefix: string): string {
  sequence += 1;
  return `${prefix}-${sequence}`;
}

function findIssue(key: unknown) {
  const issue = issues.find((item) => item.key === key);
  if (!issue) {
    throw new Error(`Jira returned 404 for issue/${String(key)}`);
  }
  return issue;
}

function chatMessage(threadId: unknown, role: string, content: string): UnknownRecord {
  return {
    id: nextId("message"),
    threadId,
    role,
    content,
    status: null,
    result: null,
    createdAt: new Date().toISOString(),
  };
}

export function jiraFallbackCommand(command: string, args: UnknownRecord = {}): unknown {
  switch (command) {
    case "get_jira_status":
      return {
        configured: true,
        missing: [],
        siteUrl: SITE_URL,
        projectKey: "DEMO",
        filters: [{ name: "Preview tickets", jql: "project = DEMO" }],
      };
    case "test_jira_connection":
      return { accountId: "preview", displayName: "Preview User" };
    case "list_jira_issues":
      return issues;
    case "get_jira_issue":
      return findIssue(args.key);
    case "list_jira_issue_types":
      return ["Task", "Bug", "Story"];
    case "create_jira_issue": {
      const input = (args.input ?? {}) as UnknownRecord;
      const created = sampleIssue(
        `DEMO-${issues.length + 1}`,
        String(input.summary ?? ""),
        "To Do",
        "new",
        String(input.issueType ?? "Task"),
      );
      created.description = String(input.description ?? "");
      issues = [created, ...issues];
      return created;
    }
    case "update_jira_issue": {
      const issue = findIssue(args.key);
      const input = (args.input ?? {}) as UnknownRecord;
      if (typeof input.summary === "string") {
        issue.summary = input.summary;
      }
      if (typeof input.description === "string") {
        issue.description = input.description;
      }
      return issue;
    }
    case "transition_jira_issue": {
      const issue = findIssue(args.key);
      const transition = issue.transitions.find((item) => item.id === args.transitionId);
      if (transition) {
        issue.status = transition.toStatus;
        issue.statusCategory = transition.toStatus === "Done" ? "done" : "indeterminate";
      }
      return issue;
    }
    case "add_jira_comment": {
      const issue = findIssue(args.key);
      issue.comments = [
        ...issue.comments,
        {
          id: nextId("comment"),
          author: "Preview User",
          created: new Date().toISOString(),
          body: String(args.text ?? ""),
        },
      ];
      return issue;
    }
    case "assign_jira_issue_to_me": {
      const issue = findIssue(args.key);
      issue.assignee = "Preview User";
      return issue;
    }
    case "list_jira_chat_threads":
      return threads;
    case "create_jira_chat_thread": {
      const now = new Date().toISOString();
      const thread = { id: nextId("thread"), title: "New chat", createdAt: now, updatedAt: now };
      threads = [thread, ...threads];
      return thread;
    }
    case "delete_jira_chat_thread":
      threads = threads.filter((thread) => thread.id !== args.threadId);
      messages = messages.filter((message) => message.threadId !== args.threadId);
      return undefined;
    case "list_jira_chat_messages":
      return messages.filter((message) => message.threadId === args.threadId);
    case "send_jira_chat_message":
      messages = [
        ...messages,
        chatMessage(args.threadId, "user", String(args.text ?? "")),
        chatMessage(args.threadId, "assistant", "The Jira agent only runs in the desktop app."),
      ];
      return messages.filter((message) => message.threadId === args.threadId);
    case "resolve_jira_chat_proposal":
      return messages;
    default:
      throw new Error(`Unsupported fallback command: ${command}`);
  }
}
