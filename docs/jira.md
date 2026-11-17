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
- `src-tauri/src/services/jira_saved.rs`: saved copies of ticket lists and tickets, and the site-and-account scope they belong to.
- `src-tauri/src/repositories/jira_cache.rs`: saved copies in SQLite.
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

The list groups tickets by status in workflow order: to do, in progress, done. The search box narrows the loaded tickets as it is typed in; that never queries Jira. To look beyond them (a filter lists at most 200), press Enter or choose the button under the results. A ticket key such as `AB-12` opens that ticket. Other text searches Jira's ticket text (`text ~`) within the project, newest first, and the results replace the list under a removable "Search:" filter beside the saved ones. The search box is cleared then, because Jira matched descriptions and comments that the box cannot see. Characters Jira's text search reads as operators are dropped from the query.

### Saved Copies

Every ticket list and ticket Jira returns is saved to local SQLite, so the view can show it at once next time.

- Opening a filter shows its saved list, if any, while Jira is asked for the current one. Opening a ticket does the same. The header says "Refreshing tickets" meanwhile.
- Jira's answer replaces the saved copy. A saved copy that arrives after Jira's answer is ignored, and refreshing a list or ticket already on screen never swaps it for a saved copy.
- A saved ticket is labelled "Saved copy". Moving, assigning, editing, and commenting wait until Jira sends the current ticket, so no change is made from out-of-date fields. Typing a comment or an edit still works.
- If Jira cannot be reached, the saved copies stay on screen with the error, and the list header says "saved copy". Refresh asks Jira again for the open ticket as well as the list.
- If Jira answers 404 for a ticket (deleted, or the account lost access), its saved copy is removed and leaves the screen. The backend reports this as `jira_not_found`.
- Each copy records when Jira was asked for it. An answer to an earlier request never replaces a copy from a later one, so requests that finish out of order cannot save an older ticket over a newer one. The site and account are read before Jira is asked, so changing the site during a request cannot file the answer under the new site.
- A change made in this view wins over loads that began before it: a ticket reload or list that started earlier cannot put the old status, summary, or fields back on screen.
- Copies belong to the Jira site and account email, so changing either never shows another's tickets. The newest 50 lists and 500 tickets are kept per site and account. A copy that cannot be read is treated as missing.
- Saved copies hold ticket content only. The API token is never saved.

### Edit Conflicts

An edit can stay unfinished for days, so saving checks that it will not overwrite someone else's change.

- Starting an edit records the ticket's summary and description. The record is kept with the unfinished edit.
- A field counts as edited when it differs from where the edit started. Fields changed in Jira meanwhile but not by the user are not sent.
- Before writing, `update_jira_issue` reads the ticket from Jira. If a field being written no longer matches where the edit started, nothing is written and it reports `jira_edit_conflict`. Writing text Jira already holds, and differences only in line endings or trailing spaces, are not conflicts.
- After a conflict the edit stays open with the user's text, and Jira's current version is loaded beside it. "Save anyway" keeps the user's changes, gives untouched fields Jira's current text, and saves against Jira's current version.
- An edit kept from before this check has no recorded start, so its first save reports a conflict for every field it writes, and "Save anyway" keeps all of the user's text.
- Jira has no conditional update, so a change made in Jira between the check and the write, a fraction of a second, can still be overwritten.
- Approving an agent proposal runs the same kind of check against the previous values its review captured: summary, description, status, or assignee. If Jira changed a field the proposal writes since then, the proposal fails, nothing is written, and the result asks the user to have the agent propose it again. A status already at the target, an assignee already cleared, or text Jira already holds is not a change. Fields the review could not capture are not checked; the review warned the user to check Jira before approving.

The ticket list and the agent panel each collapse. `[` toggles the list and `]` toggles the agent when no text field has focus. The choices are kept in browser storage.

Ticket titles use the full row width, with the key and metadata underneath. A shared assignee is shown once above the list; high and urgent priorities remain visible in rows. Status groups collapse, and searching temporarily expands matching groups. The sort selector keeps Jira's order or sorts titles within each status.

The layout responds to the Jira workspace's available width, including space taken by the app sidebar. Below 1101 px, opening the agent replaces the list. Below 760 px, the list, ticket, and agent occupy one surface at a time. Opening a ticket brings its detail forward; the list toggle returns to browsing. Toolbars remain on one line and use labeled icons when space is limited.

Unsubmitted comments, summary/description edits, and the new-ticket draft save to local SQLite after each revision. They survive navigation and app restarts, separated by Jira site and project. Successful submission clears the corresponding writing; failed submission retains it. Saves run in order. Storage failures remain visible with a retry action; failed loading blocks the workspace instead of overwriting saved writing. The browser preview uses browser storage for its sample drafts.

Ticket rows mark "Unsent comment" and "Edit in progress". "Has drafts" shows tickets with nonblank comments or an open edit, including saved writing outside the current Jira results. Those extra rows use the saved title and fetch the ticket only when opened. Search also narrows these saved titles and keys. The new-ticket draft remains available separately through "Resume draft".

The Agent button counts pending approvals across all stored conversations. The agent pane lists them oldest first, with their conversation titles. Selecting one opens its conversation and focuses the exact proposal; approval still happens on the review itself. Refresh failures keep the last known queue visible with a warning and retry. Resolved proposals leave the queue but remain in conversation history.

"Copy agent prompt" copies a self-contained prompt for the open ticket: key, summary, facts, description, subtasks, linked issues, attachments, and comments. It downloads the ticket's attachments first and lists each one by local path. If the download fails, the error is shown and the prompt names the attachments as not downloaded. Subtasks and linked issues are listed by key, status, and summary; their own descriptions are not fetched. It is meant for a coding agent in another tool, which needs no Jira access to start. The prompt tells that agent to restate the ticket, question the user on anything unclear, and implement only once both agree on the work. It is separate from the Jira chat agent and changes nothing in Jira.

## Ticket Drafts

"New ticket" opens an empty form. "Create Jira ticket" on a note opens the form holding the note's title and text, shows the agent panel, and asks the agent in a new conversation to write the ticket.

- A draft is the app's own form. Nothing reaches Jira until the user presses "Create ticket".
- Browsing tickets or choosing "Keep for later" hides the form without losing it. "Resume draft" restores it; "Discard draft" removes it. A second note cannot replace an existing draft.
- After restarting, a saved new-ticket draft stays tucked away behind "Resume draft". Undo history for agent rewrites remains session-only.
- Each message sent while a draft is open carries the form as it stands, including the user's edits, the issue types the form offers, and the source note's ID. The open ticket is not sent as context then.
- The backend reads the source note's title and raw text and shows them to the agent on every turn, so a later request can reach back to the note.
- The agent rewrites the form with `set_draft`. The rewrite replaces what the form holds, including edits typed while the agent was working.
- "Undo agent changes" restores the form as it stood immediately before the last rewrite. Further typing starts a new revision and clears that undo action. A running agent cannot rewrite a replacement draft.
- The note's own text stays in the form when the agent is busy with another conversation, fails, or names nothing to change.
- A draft that was closed while the agent worked stays closed; the rewrite is dropped.

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

Jira stores rich text as ADF. The app shows it as Markdown and writes Markdown back. `src-tauri/src/jira/adf.rs` converts in both directions, and `src/lib/markdown.ts` renders the same syntax in the ticket view.

Writing supports paragraphs, `#` headings, `-` bullets and numbered lists (nested by indentation, numbering from any start), pipe tables with an optional `| --- |` header row, fenced code blocks, and `---` rules. Inline it supports `**bold**`, `_italic_`, `~~strikethrough~~`, `` `code` ``, and `[label](url)` links to `http`, `https`, and `mailto` targets. A backslash keeps a markup character literal; reading escapes plain-text characters that would otherwise be read as markup, such as the underscores in `_leading`, but leaves ordinary punctuation alone. `\|` is a literal pipe inside a table cell.

Panels, mentions, emoji, smart links, underline, colours, alignment, merged or sized table cells, blockquotes, and list items with more than one paragraph cannot be written back. Reading still shows their text.

`descriptionEditableLosslessly` on a ticket is `false` when converting its description to Markdown and back would not rebuild it. The check ignores differences Jira does not display: node ids, how text is split into nodes, the order of marks, whitespace at line ends, empty paragraphs, and default table layout attributes. When it is `false`, the edit form requires an explicit confirmation before saving a changed description, because the save replaces formatting the editor cannot write.

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
| `set_draft` | view | at least one of `summary`, `description`, `issueType`; an open ticket draft |
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
- `set_draft` rewrites the new-ticket form. A `null` field is left as it is. The backend rejects it, and reports that to the agent, when no draft is open or `issueType` is not one the form offers. On success it emits `jira-view:action`, the store updates the form, and the agent is not called again for it.

## Trust Rules

- Lookups run immediately. Their results are stored in the conversation and the agent is called again.
- View calls run immediately because they only change the app's own view. A ticket draft is part of that view: rewriting it creates nothing in Jira.
- Changes never run when the agent asks for them. Each one is stored as a pending proposal.
- New proposals store a field-by-field review alongside the executable call. Updates, transitions, and assignments read the ticket to capture previous values. The UI labels these as a snapshot, not current Jira state. Failed comparison lookups and description-formatting loss are shown explicitly. Comments and new tickets show the proposed content. Older proposals keep their original text and indicate that no comparison was captured.
- The default project for a proposed new ticket is fixed when proposed, so its review and eventual creation name the same project.
- A proposal reaches Jira only when the user approves it in the chat. Rejecting sends nothing.
- An approved proposal without a recorded result shows "Applying", not "Applied". A failed proposal shows the error and says it will not retry. A process interrupted after claiming a proposal may leave it in "Applying"; check Jira before requesting another change.
- A proposal is claimed in one SQL statement before Jira is called, so it can run at most once.
- A Jira failure on an approved proposal is recorded on the proposal as `failed`. The proposal is not retried. So is a field Jira changed since the proposal's review captured it (see Edit Conflicts).
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

Ignored Rust tests exercise real services. All but the draft test need the email and token variables and a site URL:

```powershell
$env:WORK_NOTES_JIRA_SITE_URL = "https://<team>.atlassian.net"
scripts\cargo-test.cmd live_jira -- --ignored --nocapture
scripts\cargo-test.cmd live_agent -- --ignored --nocapture
```

`live_jira` only reads from Jira. `live_agent` matches three tests that each run one real Codex conversation: one answers from lookups, one checks that a request to see tickets puts them in the ticket list, and one checks that the agent writes a ticket draft from a note. All are read-only and never approve a proposal. The draft test uses a fake Jira and needs no Jira setup. Set `WORK_NOTES_JIRA_PROJECT_KEY` to give the agent a default project.
