export type ParseStatus = "queued" | "parsing" | "parsed" | "failed";

export type ReviewStatus = "none" | "needs_review" | "reviewed";

export type TagKind = "person" | "project" | "topic" | "urgency" | "category" | "custom";

export type TagSource = "ai" | "user";

export type ActionStatus = "suggested" | "accepted" | "dismissed" | "done";

export type ActionItemStatus = ActionStatus;

export type Tag = {
  id: string;
  name: string;
  kind: TagKind;
  source?: TagSource;
  confidence?: number | null;
  createdAt?: string;
};

export type ActionItem = {
  id: string;
  noteId: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  status: ActionStatus;
  source: "parser" | "user" | string;
  confidence?: number | null;
  noteTitle?: string;
};

export type NoteListItem = {
  id: string;
  title: string;
  rawText: string;
  cleanedText?: string | null;
  summary?: string | null;
  captureSource: string;
  createdAt: string;
  updatedAt: string;
  parseStatus: ParseStatus;
  reviewStatus: ReviewStatus;
  isArchived?: boolean;
  tags: Tag[];
  actionItemCount: number;
  suggestedActionItemCount: number;
};

export type NoteDetail = NoteListItem & {
  actionItems: ActionItem[];
  parseError?: string | null;
};

export type InboxFilters = {
  search: string;
  tagIds: string[];
  parseStatuses: ParseStatus[];
  reviewStatuses: ReviewStatus[];
  includeArchived?: boolean;
};

export type AppSettings = {
  hotkey: string;
  parserTimeoutSeconds: number;
  parserMaxRetries?: number;
  codexCommandPath: string;
  selectedTheme: string;
  launchAtStartup?: boolean;
  minimizeToTray?: boolean;
};
