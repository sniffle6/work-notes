import type { JiraViewAction } from "./types";

export const NOTE_CAPTURED_EVENT = "work-notes:note-captured";
export const CHECK_FOR_UPDATES_EVENT = "work-notes:check-for-updates";

export type NoteCapturedPayload = {
  noteId: string;
};

/** Emitted by the backend each time a Jira agent run stores new messages. */
export const JIRA_CHAT_UPDATED_EVENT = "jira-chat:updated";

export type JiraChatUpdatedPayload = {
  threadId: string;
};

/** Emitted by the backend when the Jira agent changes what the Jira view shows. */
export const JIRA_VIEW_ACTION_EVENT = "jira-view:action";

export type JiraViewActionPayload = {
  threadId: string;
  action: JiraViewAction;
};
