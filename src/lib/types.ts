export type ParseStatus = "queued" | "parsing" | "parsed" | "failed";

export type ReviewStatus = "none" | "needs_review" | "reviewed";

export type TagKind = "person" | "project" | "topic" | "urgency" | "category" | "custom";

export type TagSource = "ai" | "user";

export type ActionStatus = "suggested" | "accepted" | "dismissed" | "done";

export type ActionItemStatus = ActionStatus;

export type CardNote = {
  id: string;
  noteId: string;
  text: string;
  createdAt: string;
};

export type FollowupState = "open" | "waiting" | "blocked";

export type FollowupDisplayState = FollowupState | "done";

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
  followupState?: FollowupState | null;
  followupLane?: string | null;
  completedAt?: string | null;
};

export type ActionReviewItem = {
  id: string;
  noteId: string;
  noteTitle: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  confidence?: number | null;
  createdAt: string;
};

export type FollowupItem = {
  id: string;
  noteId: string;
  noteTitle: string;
  text: string;
  owner?: string | null;
  dueDate?: string | null;
  status: Extract<ActionStatus, "accepted" | "done">;
  source: "parser" | "user" | string;
  confidence?: number | null;
  followupState?: FollowupState | null;
  followupLane?: string | null;
  tags: Tag[];
  createdAt: string;
  completedAt?: string | null;
};

export type FollowupLane = {
  name: string;
  activeCount: number;
  followups: FollowupItem[];
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
  completedAt?: string | null;
  completionNote?: string | null;
  tags: Tag[];
  actionItemCount: number;
  suggestedActionItemCount: number;
};

export type NoteDetail = NoteListItem & {
  actionItems: ActionItem[];
  cardNotes?: CardNote[];
  parseError?: string | null;
  cleanedEdited?: boolean;
  /** Keys of Jira tickets created from this note. */
  jiraIssueKeys?: string[];
};

export type InboxFilters = {
  search: string;
  tagIds: string[];
  parseStatuses: ParseStatus[];
  reviewStatuses: ReviewStatus[];
  includeArchived?: boolean;
  includeCompleted?: boolean;
  limit?: number;
};

/** One colour per theme token, each as `#rrggbb`. */
export type ThemeTokenValues = {
  appBg: string;
  surface1: string;
  surface2: string;
  surfaceInput: string;
  borderDefault: string;
  borderStrong: string;
  textPrimary: string;
  textMuted: string;
  accentPrimary: string;
  accentHot: string;
  statusSuccess: string;
  statusWarning: string;
  statusError: string;
};

/** A theme the user built and kept. Its id starts with `custom-`. */
export type CustomTheme = {
  id: string;
  label: string;
  mode: "dark" | "light";
  tokens: ThemeTokenValues;
};

/** A theme the theme agent produced, not yet kept. */
export type GeneratedTheme = {
  name: string;
  mode: "dark" | "light";
  tokens: ThemeTokenValues;
  /** The agent's one-line account of the look. */
  notes: string;
  /** Readability problems the agent could not fix. */
  warnings: string[];
};

export type AppSettings = {
  hotkey: string;
  parserTimeoutSeconds: number;
  parserMaxRetries?: number;
  codexCommandPath: string;
  linkedWorkspacePaths?: string[];
  selectedTheme: string;
  launchAtStartup?: boolean;
  minimizeToTray?: boolean;
  jiraSiteUrl?: string;
  jiraProjectKey?: string;
  /** Resolved by the backend from the site URL; passed through unchanged. */
  jiraCloudId?: string;
  /** Custom ticket filters. Empty means the built-in filters are used. */
  jiraFilters?: JiraFilter[];
  /** Themes built with the theme agent. `selectedTheme` may name one. */
  customThemes?: CustomTheme[];
};

export type JiraFilter = {
  name: string;
  jql: string;
};

export type JiraStatus = {
  configured: boolean;
  /** Human-readable setup steps still missing when not configured. */
  missing: string[];
  siteUrl: string;
  projectKey: string;
  filters: JiraFilter[];
};

export type JiraAccount = {
  accountId: string;
  displayName: string;
};

/** A change the Jira agent makes to the Jira view. It never changes Jira. */
export type JiraViewAction =
  | { kind: "showIssues"; title: string; jql: string }
  | { kind: "openIssue"; issueKey: string };

export type JiraStatusCategory = "new" | "indeterminate" | "done" | "unknown";

export type JiraIssueSummary = {
  key: string;
  summary: string;
  status: string;
  statusCategory: JiraStatusCategory;
  issueType: string;
  priority: string | null;
  assignee: string | null;
  updated: string | null;
  labels: string[];
  url: string;
};

export type JiraComment = {
  id: string;
  author: string;
  created: string;
  body: string;
};

export type JiraTransition = {
  id: string;
  name: string;
  toStatus: string;
};

/** The short form of an issue named by another issue. */
export type JiraRelatedIssue = {
  key: string;
  summary: string;
  status: string;
};

export type JiraLinkedIssue = JiraRelatedIssue & {
  /** How the open issue relates to this one, such as "blocks". */
  relationship: string;
};

/** A file attached to a ticket. Its content is downloaded on request. */
export type JiraAttachment = {
  id: string;
  filename: string;
  mimeType: string | null;
  /** Size in bytes; 0 when Jira does not report one. */
  size: number;
  author: string | null;
  created: string | null;
  /** True when Jira offers a small preview image for this attachment. */
  hasThumbnail: boolean;
};

/** An attachment saved to disk. */
export type JiraAttachmentFile = {
  id: string;
  /** The local file name, which can differ from the name in Jira. */
  filename: string;
  path: string;
};

/** An image attachment open in the in-app viewer. */
export type JiraAttachmentView = {
  attachmentId: string;
  filename: string;
  /** Object URL of the full image. */
  url: string;
  /** Where the file is saved on disk. */
  path: string;
};

export type JiraAttachmentDownload = {
  /** The ticket's folder under Downloads\Work Notes. */
  folder: string;
  files: JiraAttachmentFile[];
};

export type JiraIssueDetail = JiraIssueSummary & {
  description: string;
  /** False when saving a plain-text edit would flatten Jira formatting. */
  descriptionEditableLosslessly: boolean;
  reporter: string | null;
  parentKey: string | null;
  subtasks: JiraRelatedIssue[];
  linkedIssues: JiraLinkedIssue[];
  attachments: JiraAttachment[];
  comments: JiraComment[];
  transitions: JiraTransition[];
};

export type JiraIssueDraft = {
  issueType: string;
  summary: string;
  description: string;
  /** Note the ticket is created from, so the key can be linked back. */
  noteId?: string | null;
};

export type JiraIssueEdit = {
  summary?: string;
  description?: string;
};

export type JiraChatThread = {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
};

export type JiraChatRole = "user" | "assistant" | "tool" | "proposal";

export type JiraChatStatus = "pending" | "approved" | "rejected" | "failed" | "ok" | "error";

export type JiraChatMessage = {
  id: string;
  threadId: string;
  role: JiraChatRole;
  content: string;
  status: JiraChatStatus | null;
  result: string | null;
  createdAt: string;
};
