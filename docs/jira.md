# Jira Integration

The Jira view lists tickets, edits them, and hosts a chat agent that works Jira for the user. Read this before touching Jira access, the agent tool contract, or proposal approval.

## Setup

Jira access needs a site URL in settings and credentials in the environment.

| Item | Where | Notes |
| --- | --- | --- |
| Site URL | Settings > Jira | The `https://<team>.atlassian.net` address. Required. |
| Project key | Settings > Jira | New tickets are created here. It scopes the built-in filters. |
| Account email | `JIRA_EMAIL`, else `BITBUCKET_EMAIL` | Atlassian account email. |
| API token | `JIRA_API_TOKEN` | Jira-scoped token with `read:jira-work` and `write:jira-work`. |
| Cloud ID | `JIRA_CLOUD_ID` (optional) | Normally resolved from the site URL and saved to settings. |

Create the token at <https://id.atlassian.com/manage-profile/security/api-tokens>. The app reads environment variables at startup, so restart Work Notes after changing them.

The API token is never written to SQLite, never sent to the chat agent, and only ever sent to `api.atlassian.com`.

## Ownership

- `src-tauri/src/jira/transport.rs`: HTTP, gateway URL, error mapping, cloud ID lookup. The only module that performs Jira network I/O.
- `src-tauri/src/jira/client.rs`: `JiraOps` trait and `JiraClient`, the Jira operations the app uses.
- `src-tauri/src/jira/adf.rs`: Atlassian Document Format to and from Markdown-flavored text.
- `src-tauri/src/jira/transitions.rs`: matching a status name to a workflow transition.
- `src-tauri/src/jira/agent.rs`: agent prompt, tool call types, and turn parsing.
- `src-tauri/src/jira/chat.rs`: conversation types.
- `src-tauri/src/services/jira.rs`: setup status, filters, and building a client from settings and environment.
- `src-tauri/src/services/jira_agent.rs`: the agent loop and proposal approval.
- `src-tauri/src/services/jira_attachments.rs`: saving a ticket's attachments to disk, including local file naming.
- `src-tauri/src/repositories/jira.rs`: chat threads, chat messages, and note-to-ticket links in SQLite.
- `src-tauri/src/commands/jira.rs`: Tauri commands and DTOs.
- `schemas/jira-agent-turn.schema.json`: output schema for one agent turn.
- `src/lib/stores/jira.ts`: frontend workflow store for the Jira view.
- `src/lib/jira-list.ts`: ticket list search, status grouping, and priority emphasis.
- `src/lib/jira-prompt.ts`: the prompt that "Copy agent prompt" puts on the clipboard.
- `src/lib/agent-prompt.ts`: the opening instructions shared with the note prompt in `src/lib/note-prompt.ts`.

## Jira Access

Requests go to `https://api.atlassian.com/ex/jira/<cloudId>/rest/api/3/` with HTTP basic auth. Scoped API tokens only work through this gateway; the site-direct REST URL answers 404 for them.

The cloud ID comes from `JIRA_CLOUD_ID`, else the saved setting, else an unauthenticated lookup of `<site>/_edge/tenant_info`. A looked-up ID is saved. Changing the site URL clears the saved ID.

Searches use `POST search/jql` with page tokens and return at most 200 tickets per filter.

## Ticket List And Agent Prompt

The list groups tickets by status in workflow order: to do, in progress, done. The search box narrows the loaded tickets only; it does not query Jira.

The ticket list and the agent panel each collapse. `[` toggles the list and `]` toggles the agent when no text field has focus. The choices are kept in browser storage.

"Copy agent prompt" copies a self-contained prompt for the open ticket: key, summary, facts, description, subtasks, linked issues, attachments, and comments. It downloads the ticket's attachments first and lists each one by local path. If the download fails, the error is shown and the prompt names the attachments as not downloaded. Subtasks and linked issues are listed by key, status, and summary; their own descriptions are not fetched. It is meant for a coding agent in another tool, which needs no Jira access to start. The prompt tells that agent to restate the ticket, question the user on anything unclear, and implement only once both agree on the work. It is separate from the Jira chat agent and changes nothing in Jira.

## Attachments

The ticket detail lists a ticket's attachments. The user downloads one or all of them; no full file is downloaded when a ticket opens. The Jira chat agent has no attachment tool.

- When a ticket opens, the app loads Jira's thumbnail (`attachment/thumbnail/<id>`) for each attachment Jira can preview, one request each, once per app run. Jira decides which files have one; in practice these are images. A thumbnail that fails to load is left out without an error. Thumbnails are held in memory only, limited to 2 MB each, and must be PNG, JPEG, GIF, or WebP.
- Clicking a thumbnail downloads the full file if it is not saved yet and opens it in a viewer over the ticket pane. The viewer's folder button shows the file in its folder; Esc, the close button, or a click beside the image closes it. Opening another ticket also closes it.
- An image opens fitted to the viewer. The header buttons, `+` and `-`, or Ctrl with the mouse wheel zoom between 10% and 800% of its real size. Zooming with the wheel keeps the point under the cursor in place. `0` or the zoom level button fits it again. A double-click switches between fitted and real size. A zoomed image pans by dragging and scrolls with the wheel.
- A video attachment without a thumbnail gets a play button. A video opens in the same viewer with the webview's player controls and starts playing; it has no zoom.
- The viewer shows PNG, JPEG, GIF, and WebP images up to 30 MB, and MP4, QuickTime, and WebM videos up to 200 MB. The file's own leading bytes decide which it is, not its name. Anything else is reported as an error and stays downloaded, to be opened from its folder.
- A video is passed to the viewer whole, in memory. Whether it plays depends on the codecs WebView2 supports; a QuickTime file plays only when its video is H.264.
- Files are saved to `Downloads\Work Notes\<KEY>\`. A downloaded file offers "Show in folder".
- The backend reads the attachment list from Jira again for each download. The frontend sends only the ticket key and attachment ids, never a file name or path.
- A file name from Jira is reduced to one safe Windows path component, so it cannot leave the ticket folder. A second attachment with the same name, ignoring case, gets its Jira id added: `screen (10002).png`.
- A file already present at the size Jira reports is kept and not downloaded again.
- Content is written to `<name>.part` and renamed when complete. A failed download removes the partial file and stops; files saved before it stay.
- Content comes from `attachment/content/<id>`, which redirects to Atlassian's media host. The HTTP client drops the credentials when the host changes, so the token still only reaches `api.atlassian.com`.
- A download may take up to 10 minutes before it times out.

## Descriptions And Comments

Jira stores rich text as ADF. The app shows it as Markdown-flavored text and writes plain text back.

Writing supports paragraphs, `#` headings, `-` bullets, numbered lists, fenced code blocks, and `---` rules. Tables, panels, mentions, nested lists, and inline marks such as bold or code cannot be written back.

`descriptionEditableLosslessly` on a ticket is `false` when its description uses anything the editor cannot write. The edit form then requires an explicit confirmation before saving a changed description, because the save replaces the formatted description with plain text.

## Agent Contract

The agent runs through local `codex exec`, like the note parser. It has no Jira access of its own.

Each turn the app sends a prompt with instructions, context, and the stored conversation. Codex must return JSON matching `schemas/jira-agent-turn.schema.json`:

```json
{
  "reply": "Moving ABC-12 once you approve.",
  "toolCalls": [
    {
      "tool": "transition_issue",
      "issueKey": "ABC-12",
      "status": "In Progress",
      "jql": null,
      "projectKey": null,
      "issueType": null,
      "summary": null,
      "description": null,
      "comment": null,
      "assignee": null,
      "title": null
    }
  ]
}
```

Tools:

| Tool | Kind | Required fields |
| --- | --- | --- |
| `show_issues` | view | `jql`; optional `title` (default "Agent search") |
| `open_issue` | view | `issueKey` |
| `search_issues` | lookup | `jql` |
| `get_issue` | lookup | `issueKey` |
| `create_issue` | change | `summary`; optional `issueType` (default Task), `description`, `projectKey` |
| `update_issue` | change | `issueKey` and `summary` and/or `description` |
| `transition_issue` | change | `issueKey`, `status` |
| `add_comment` | change | `issueKey`, `comment` |
| `assign_issue` | change | `issueKey`, `assignee` (`me` or `unassigned`) |

View calls change what the app shows and never change Jira:

- `show_issues` runs the search first. A query Jira rejects is reported back to the agent and the ticket list is left alone. On success the backend emits `jira-view:action`, the Jira store shows the search as a temporary filter beside the saved ones, and the agent is called again with the tickets (the first 50 are listed).
- `open_issue` emits `jira-view:action` and the store opens that ticket. The agent is not called again for it.

## Trust Rules

- Lookups run immediately. Their results are stored in the conversation and the agent is called again.
- View calls run immediately because they only change the app's own view.
- Changes never run when the agent asks for them. Each one is stored as a pending proposal.
- A proposal reaches Jira only when the user approves it in the chat. Rejecting sends nothing.
- A proposal is claimed in one SQL statement before Jira is called, so it can run at most once.
- A Jira failure on an approved proposal is recorded on the proposal as `failed`. The proposal is not retried.
- A tool call the app cannot accept is reported back to the agent as a failed lookup.
- One user message allows at most 6 agent turns. Then the app stops the run and says so.
- One conversation has at most one agent run at a time.
- The prompt includes the last 40 messages. Lookup results are capped at 6000 characters each.
- Do not add a path that lets the agent change Jira without a stored, user-approved proposal.

## Failure Modes To Preserve

- Missing setup lists each missing step and performs no network I/O.
- HTTP 401 reports rejected credentials.
- HTTP 404 reports a missing item or a token without `read:jira-work`.
- Other HTTP errors carry Jira's own error messages.
- A missing `codex` command, a timeout, a non-zero exit, or output outside the schema fails the turn. The user's message stays stored.
- A request for an attachment id the ticket does not have is rejected before anything is downloaded.
- A failed attachment download or file write reports `attachment download failed` and leaves no partial file.
- Creating a ticket from a note checks the note exists first. If linking fails after creation, the error names the created ticket.

## Manual Checks

Two ignored Rust tests exercise real services. Both need the email and token variables and a site URL:

```powershell
$env:WORK_NOTES_JIRA_SITE_URL = "https://<team>.atlassian.net"
scripts\cargo-test.cmd live_jira -- --ignored --nocapture
scripts\cargo-test.cmd live_agent -- --ignored --nocapture
```

`live_jira` only reads from Jira. `live_agent` matches two tests that each run one real Codex conversation: one answers from lookups, one checks that a request to see tickets puts them in the ticket list. Both are read-only and never approve a proposal. Set `WORK_NOTES_JIRA_PROJECT_KEY` to give the agent a default project.
