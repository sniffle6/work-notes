# Work Notes Agent-Team Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development for task execution and superpowers:dispatching-parallel-agents for explicitly marked parallel waves. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the v1 Windows-only Work Notes desktop app from the approved design spec.

**Architecture:** Tauri v2 owns the Windows shell, tray, global hotkey, background worker, SQLite persistence, and Codex CLI process execution. Svelte/TypeScript owns the quick-capture, inbox, note detail, review, settings, and tokenized dark compact UI. Capture writes raw notes immediately; automatic background parsing enriches notes through local `codex exec` without OpenAI API billing.

**Tech Stack:** Tauri v2, Svelte, TypeScript, Vite, Rust, rusqlite with SQLite FTS5, Vitest, Cargo tests, local Codex CLI.

---

## Controller Operating Model

This plan is written for a controller agent dispatching an agent team. The controller coordinates branches, sequencing, reviews, and integration. Worker agents receive only their task packet, the design spec path, and any exact files they own.

Start from repo root:

```powershell
cd "C:\code\Crazy Projects Go Brrr\work-notes"
git status --short --branch
```

Expected before starting implementation:

```text
## master
```

Create an implementation branch:

```powershell
git switch -c feature/work-notes-v1
```

Expected:

```text
Switched to a new branch 'feature/work-notes-v1'
```

Agent team rules:

- Every implementer must be told: "You are not alone in this codebase. Do not revert or overwrite edits outside your assigned files. Adjust your work to accommodate other agents' changes."
- Each worker owns explicit files. Workers must not edit files outside their ownership without asking the controller.
- Scaffold is sequential. Parallel waves start only after scaffold tests pass.
- Parallel workers commit their own changes with narrow commits.
- The controller reviews each worker's diff before dispatching the next dependent wave.
- The controller resolves integration conflicts and owns shared registration files unless a task packet grants ownership.
- Use fake parser tests before invoking the real Codex CLI.
- Do not add OpenAI API integration or API-key settings.

## Dependency Graph

```text
Task 0 Scaffold
  -> Wave 1 parallel:
      Task 1 Database and repositories
      Task 2 Parser contracts and fake provider
      Task 3 Theme foundation and static UI shell
      Task 4 Native shell window/tray/hotkey skeleton
  -> Integration Gate 1
  -> Wave 2 parallel:
      Task 5 Codex parser provider
      Task 6 Frontend workflows and stores
      Task 7 Tauri command API and service wiring
  -> Integration Gate 2
  -> Task 8 Windows behavior integration
  -> Task 9 End-to-end verification and docs
```

## File Ownership Map

Controller owns shared registration files unless a task says otherwise:

- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- `package.json`
- `vite.config.ts`
- `tsconfig.json`

Task-specific ownership:

- Task 0 owns scaffold files and baseline config.
- Task 1 owns `src-tauri/src/db/**`, `src-tauri/src/domain.rs`, `src-tauri/src/repositories/**`, database migrations.
- Task 2 owns `src-tauri/src/parser/types.rs`, `src-tauri/src/parser/fake_provider.rs`, `src-tauri/src/parser/result_applier.rs`, `schemas/parse-note.schema.json`.
- Task 3 owns `src/lib/theme/**`, `src/app.css`, static Svelte layout components under `src/lib/components/**`.
- Task 4 owns `src-tauri/src/windowing/**` and native shell tests that do not require a live tray.
- Task 5 owns `src-tauri/src/parser/codex_provider.rs`, `src-tauri/src/parser/prompt.rs`, parser-provider tests.
- Task 6 owns `src/lib/api.ts`, `src/lib/stores/**`, `src/lib/types.ts`, `src/App.svelte`, workflow components, and frontend tests.
- Task 7 owns `src-tauri/src/commands.rs`, `src-tauri/src/app_state.rs`, service wiring tests.
- Task 8 owns final Tauri registration, runtime permissions, and Windows manual behavior verification.
- Task 9 owns `README.md`, verification scripts, and final QA notes.

## Shared Domain Contracts

Rust domain enums must use these names:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    Queued,
    Parsing,
    Parsed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    None,
    NeedsReview,
    Reviewed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    Person,
    Project,
    Topic,
    Urgency,
    Category,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Suggested,
    Accepted,
    Dismissed,
    Done,
}
```

Parser output JSON uses camelCase because it comes from Codex:

```json
{
  "cleanedText": "Mike said the deploy moved to Friday. Check the config flag before QA gets the build.",
  "summary": "Deploy moved to Friday; verify config flag before QA build.",
  "tags": [
    { "name": "Mike", "kind": "person", "confidence": 0.92 }
  ],
  "actionItems": [
    {
      "text": "Check the config flag before QA gets the build.",
      "owner": null,
      "dueDate": null,
      "confidence": 0.83,
      "requiresReview": true
    }
  ]
}
```

Frontend status strings use snake_case and map directly to serialized Rust enums.

## Task 0: Scaffold Tauri/Svelte Project

**Agent:** Coordinator only, sequential blocker.

**Files:**

- Modify: `.gitignore`
- Create: `package.json`
- Create: `package-lock.json`
- Create: `index.html`
- Create: `src/**`
- Create: `src-tauri/**`
- Create: `vite.config.ts`
- Create: `tsconfig.json`

- [ ] **Step 1: Confirm branch and clean state**

Run:

```powershell
git status --short --branch
```

Expected:

```text
## feature/work-notes-v1
```

- [ ] **Step 2: Scaffold into a temp directory**

Run from repo root:

```powershell
$tmp = Join-Path $PWD ".tmp-tauri-scaffold"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
npm create tauri-app@2 .tmp-tauri-scaffold -- --manager npm --template svelte-ts --yes
```

Expected:

```text
Template created!
```

- [ ] **Step 3: Move scaffold files into repo root**

Run:

```powershell
Get-ChildItem -LiteralPath ".tmp-tauri-scaffold" -Force | ForEach-Object {
  Move-Item -LiteralPath $_.FullName -Destination $PWD -Force
}
Remove-Item -Recurse -Force ".tmp-tauri-scaffold"
```

Expected: repo root contains `package.json`, `src`, `src-tauri`, `vite.config.ts`, and existing `docs`.

- [ ] **Step 4: Install baseline dependencies**

Run:

```powershell
npm install
cd src-tauri
cargo add rusqlite --features bundled,chrono,serde_json
cargo add serde --features derive
cargo add serde_json
cargo add chrono --features serde
cargo add thiserror
cargo add uuid --features v4,serde
cargo add jsonschema
cargo add tempfile
cargo add directories
cd ..
npm install -D vitest jsdom @testing-library/svelte @testing-library/jest-dom
```

Expected: commands complete without non-zero exit.

- [ ] **Step 5: Add Tauri plugins**

Run:

```powershell
npm run tauri add global-shortcut
npm run tauri add single-instance
```

Expected: plugin dependencies and permissions are added by Tauri CLI.

- [ ] **Step 6: Extend `.gitignore`**

Ensure `.gitignore` contains exactly these project-specific entries in addition to scaffold defaults:

```gitignore
.superpowers/
.tmp-tauri-scaffold/
target/
node_modules/
dist/
build/
*.db
*.sqlite
```

- [ ] **Step 7: Add test scripts**

Ensure `package.json` has these scripts:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri": "tauri"
  }
}
```

If the scaffold includes additional valid Tauri scripts, keep them and add missing scripts only.

- [ ] **Step 8: Verify scaffold**

Run:

```powershell
npm run build
cd src-tauri
cargo test
cd ..
```

Expected:

```text
npm run build exits 0
cargo test exits 0
```

- [ ] **Step 9: Commit scaffold**

Run:

```powershell
git add .gitignore package.json package-lock.json index.html src src-tauri vite.config.ts tsconfig.json
git commit -m "chore: scaffold tauri svelte app"
```

Expected: commit succeeds.

## Wave 1 Parallel Dispatch

Dispatch Tasks 1-4 after Task 0 is committed. These workers own disjoint files and can run in parallel.

Each worker prompt must include:

```text
You are not alone in this codebase. Do not revert or overwrite edits outside your assigned files. Adjust your work to accommodate other agents' changes.

Read docs/superpowers/specs/2026-05-20-work-notes-design.md for product requirements. Your assigned task owns only the files listed in your prompt. Do not edit src-tauri/src/lib.rs, tauri.conf.json, package.json, or shared registration files unless the prompt explicitly says so.
```

## Task 1: Database, Domain Models, Repositories

**Agent:** Backend persistence worker.

**Files:**

- Create: `src-tauri/src/domain.rs`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/migrations.rs`
- Create: `src-tauri/src/repositories/mod.rs`
- Create: `src-tauri/src/repositories/notes.rs`
- Create: `src-tauri/src/repositories/tags.rs`
- Create: `src-tauri/src/repositories/actions.rs`
- Create: `src-tauri/src/repositories/parse_jobs.rs`
- Test: unit tests inside the created Rust modules

- [ ] **Step 1: Write failing repository tests**

Create tests that prove:

```rust
#[test]
fn save_note_creates_raw_note_and_parse_job() {
    let db = test_db();
    let notes = NoteRepository::new(db.clone());
    let jobs = ParseJobRepository::new(db.clone());

    let note = notes.create_raw_note("Mike says deploy moved to Friday").unwrap();
    jobs.enqueue(note.id).unwrap();

    let stored = notes.get(note.id).unwrap().unwrap();
    assert_eq!(stored.raw_text, "Mike says deploy moved to Friday");
    assert_eq!(stored.parse_status, ParseStatus::Queued);
    assert_eq!(stored.review_status, ReviewStatus::None);
    assert_eq!(jobs.next_queued().unwrap().unwrap().note_id, note.id);
}
```

```rust
#[test]
fn fts_search_matches_raw_and_cleaned_text() {
    let db = test_db();
    let notes = NoteRepository::new(db);

    let note = notes.create_raw_note("cfg flag before qa").unwrap();
    notes.apply_cleaned_text(note.id, "Check the config flag before QA gets the build.", "Verify config flag.").unwrap();

    let results = notes.search("config").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, note.id);
}
```

Run:

```powershell
cd src-tauri
cargo test save_note_creates_raw_note_and_parse_job fts_search_matches_raw_and_cleaned_text
```

Expected before implementation: tests fail because modules do not exist.

- [ ] **Step 2: Implement schema migration**

`src-tauri/src/db/migrations.rs` must create these tables and FTS index:

```sql
CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY,
  raw_text TEXT NOT NULL,
  cleaned_text TEXT,
  summary TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  capture_source TEXT NOT NULL,
  parse_status TEXT NOT NULL,
  review_status TEXT NOT NULL,
  is_archived INTEGER NOT NULL DEFAULT 0
);

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
  note_id UNINDEXED,
  raw_text,
  cleaned_text,
  summary
);

CREATE TABLE IF NOT EXISTS tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(name, kind)
);

CREATE TABLE IF NOT EXISTS note_tags (
  note_id TEXT NOT NULL,
  tag_id TEXT NOT NULL,
  source TEXT NOT NULL,
  confidence REAL,
  PRIMARY KEY(note_id, tag_id, source),
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
  FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS action_items (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  text TEXT NOT NULL,
  owner TEXT,
  due_date TEXT,
  status TEXT NOT NULL,
  source TEXT NOT NULL,
  confidence REAL,
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS parse_jobs (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  status TEXT NOT NULL,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS parse_runs (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  provider TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  raw_response TEXT NOT NULL,
  parsed_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

- [ ] **Step 3: Implement repositories**

Implement:

```rust
impl NoteRepository {
    pub fn create_raw_note(&self, raw_text: &str) -> Result<Note, RepositoryError>;
    pub fn get(&self, id: NoteId) -> Result<Option<Note>, RepositoryError>;
    pub fn list_inbox(&self, filters: InboxFilters) -> Result<Vec<NoteListItem>, RepositoryError>;
    pub fn apply_cleaned_text(&self, id: NoteId, cleaned_text: &str, summary: &str) -> Result<(), RepositoryError>;
    pub fn set_parse_status(&self, id: NoteId, status: ParseStatus) -> Result<(), RepositoryError>;
    pub fn set_review_status(&self, id: NoteId, status: ReviewStatus) -> Result<(), RepositoryError>;
    pub fn search(&self, query: &str) -> Result<Vec<NoteListItem>, RepositoryError>;
}
```

Implement matching repository methods for tags, action items, and parse jobs.

- [ ] **Step 4: Run persistence tests**

Run:

```powershell
cd src-tauri
cargo test repositories db
```

Expected: all persistence tests pass.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src-tauri/src/domain.rs src-tauri/src/db src-tauri/src/repositories
git commit -m "feat: add sqlite note persistence"
```

Expected: commit succeeds.

## Task 2: Parser Contracts, Fake Provider, Result Applier

**Agent:** Parser domain worker.

**Files:**

- Create: `src-tauri/src/parser/mod.rs`
- Create: `src-tauri/src/parser/types.rs`
- Create: `src-tauri/src/parser/fake_provider.rs`
- Create: `src-tauri/src/parser/result_applier.rs`
- Create: `schemas/parse-note.schema.json`
- Test: unit tests inside parser modules

- [ ] **Step 1: Write failing parser-result tests**

Create tests proving:

```rust
#[test]
fn parser_result_deserializes_camel_case_json() {
    let json = r#"{
      "cleanedText": "Mike said the deploy moved to Friday.",
      "summary": "Deploy moved to Friday.",
      "tags": [{ "name": "Mike", "kind": "person", "confidence": 0.92 }],
      "actionItems": [{
        "text": "Check config flag.",
        "owner": null,
        "dueDate": null,
        "confidence": 0.83,
        "requiresReview": true
      }]
    }"#;

    let result: ParserResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.cleaned_text, "Mike said the deploy moved to Friday.");
    assert_eq!(result.tags[0].kind, TagKind::Person);
    assert!(result.action_items[0].requires_review);
}
```

```rust
#[test]
fn fake_provider_returns_deterministic_parse_result() {
    let provider = FakeParserProvider::default();
    let result = provider.parse("mike said deploy friday").unwrap();
    assert!(result.cleaned_text.contains("mike said deploy friday"));
    assert_eq!(result.summary, "Parsed by fake provider.");
}
```

Run:

```powershell
cd src-tauri
cargo test parser_result_deserializes_camel_case_json fake_provider_returns_deterministic_parse_result
```

Expected before implementation: tests fail because parser modules do not exist.

- [ ] **Step 2: Create JSON schema**

Create `schemas/parse-note.schema.json` with:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Work Notes Parse Result",
  "type": "object",
  "required": ["cleanedText", "summary", "tags", "actionItems"],
  "additionalProperties": false,
  "properties": {
    "cleanedText": { "type": "string", "minLength": 1 },
    "summary": { "type": "string", "minLength": 1 },
    "tags": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "kind", "confidence"],
        "additionalProperties": false,
        "properties": {
          "name": { "type": "string", "minLength": 1 },
          "kind": {
            "type": "string",
            "enum": ["person", "project", "topic", "urgency", "category", "custom"]
          },
          "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
        }
      }
    },
    "actionItems": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["text", "owner", "dueDate", "confidence", "requiresReview"],
        "additionalProperties": false,
        "properties": {
          "text": { "type": "string", "minLength": 1 },
          "owner": { "type": ["string", "null"] },
          "dueDate": { "type": ["string", "null"] },
          "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
          "requiresReview": { "type": "boolean" }
        }
      }
    }
  }
}
```

- [ ] **Step 3: Implement parser types**

Create types with serde renames from camelCase JSON to snake_case Rust fields:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParserResult {
    pub cleaned_text: String,
    pub summary: String,
    pub tags: Vec<ParsedTag>,
    pub action_items: Vec<ParsedActionItem>,
}
```

Include `ParsedTag`, `ParsedActionItem`, `ParserProvider`, and `ParserError`.

- [ ] **Step 4: Implement result applier against repository traits**

`ParserResultApplier` must apply:

- cleaned text and summary to the note
- tags with source `ai`
- action items with status `suggested`
- review status `needs_review` if any parsed action item has `requiresReview = true`

- [ ] **Step 5: Run parser contract tests**

Run:

```powershell
cd src-tauri
cargo test parser
```

Expected: parser contract and fake provider tests pass.

- [ ] **Step 6: Commit**

Run:

```powershell
git add schemas src-tauri/src/parser
git commit -m "feat: add parser contracts"
```

Expected: commit succeeds.

## Task 3: Dark Compact Theme And Static UI Shell

**Agent:** Frontend theme worker.

**Files:**

- Create: `src/lib/theme/tokens.ts`
- Create: `src/lib/theme/themes.ts`
- Create: `src/lib/theme/applyTheme.ts`
- Create: `src/lib/components/AppShell.svelte`
- Create: `src/lib/components/QuickCapturePanel.svelte`
- Create: `src/lib/components/InboxList.svelte`
- Create: `src/lib/components/StatusBadge.svelte`
- Modify: `src/app.css`
- Modify: `src/App.svelte`
- Test: `src/lib/theme/theme.test.ts`

- [ ] **Step 1: Write failing theme tests**

Create `src/lib/theme/theme.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { darkCompactTheme, toCssVariables } from "./themes";

describe("theme tokens", () => {
  it("maps every dark compact token to a CSS custom property", () => {
    const vars = toCssVariables(darkCompactTheme);

    expect(vars["--color-app-bg"]).toBe("#11151c");
    expect(vars["--color-surface-1"]).toBe("#1b212b");
    expect(vars["--color-surface-2"]).toBe("#202833");
    expect(vars["--color-surface-input"]).toBe("#111720");
    expect(vars["--color-text-primary"]).toBe("#edf4fb");
    expect(vars["--color-accent-primary"]).toBe("#2f6f7a");
  });
});
```

Run:

```powershell
npm test -- src/lib/theme/theme.test.ts
```

Expected before implementation: test fails because theme modules do not exist.

- [ ] **Step 2: Implement theme token files**

`src/lib/theme/tokens.ts` must export:

```ts
export type ThemeToken =
  | "app.bg"
  | "surface.1"
  | "surface.2"
  | "surface.input"
  | "border.default"
  | "border.strong"
  | "text.primary"
  | "text.muted"
  | "accent.primary"
  | "accent.hot"
  | "status.success"
  | "status.warning"
  | "status.error";

export type ThemeDefinition = {
  id: string;
  label: string;
  compact: boolean;
  tokens: Record<ThemeToken, string>;
};
```

`src/lib/theme/themes.ts` must export `darkCompactTheme` and `toCssVariables(theme)`.

- [ ] **Step 3: Build static components**

Build components with props only. Do not call Tauri commands from this task.

`QuickCapturePanel.svelte` props:

```ts
export let value = "";
export let saving = false;
export let error: string | null = null;
```

It must dispatch:

- `save`
- `close`
- `input`

Keyboard behavior in this component:

- `Enter` without `Shift` dispatches `save`
- `Shift+Enter` inserts newline through native textarea behavior
- `Escape` dispatches `close`

- [ ] **Step 4: Verify frontend**

Run:

```powershell
npm test
npm run build
```

Expected: tests and build pass.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src/lib/theme src/lib/components src/app.css src/App.svelte
git commit -m "feat: add dark compact ui shell"
```

Expected: commit succeeds.

## Task 4: Native Window, Tray, Hotkey Skeleton

**Agent:** Native shell worker.

**Files:**

- Create: `src-tauri/src/windowing/mod.rs`
- Create: `src-tauri/src/windowing/quick_capture.rs`
- Create: `src-tauri/src/windowing/tray.rs`
- Create: `src-tauri/src/windowing/hotkey.rs`
- Test: unit tests inside `src-tauri/src/windowing/**`

- [ ] **Step 1: Write failing position test**

Create a pure Rust test:

```rust
#[test]
fn bottom_right_position_respects_margin() {
    let monitor = WorkArea { x: 0, y: 0, width: 1920, height: 1080 };
    let window = WindowSize { width: 560, height: 260 };

    let position = bottom_right_position(monitor, window, 24);

    assert_eq!(position.x, 1336);
    assert_eq!(position.y, 796);
}
```

Run:

```powershell
cd src-tauri
cargo test bottom_right_position_respects_margin
```

Expected before implementation: test fails because windowing module does not exist.

- [ ] **Step 2: Implement pure window positioning helpers**

Implement:

```rust
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

pub fn bottom_right_position(work_area: WorkArea, window: WindowSize, margin: i32) -> WindowPosition;
```

- [ ] **Step 3: Implement Tauri shell wrappers**

Implement wrappers for:

- show quick capture window
- hide quick capture window
- focus note textarea through frontend event
- initialize tray menu
- register global shortcut

Use labels:

- `main`
- `quick-capture`

- [ ] **Step 4: Verify native skeleton**

Run:

```powershell
cd src-tauri
cargo test windowing
```

Expected: pure helper tests pass. Runtime tray/hotkey behavior is verified in Task 8.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src-tauri/src/windowing
git commit -m "feat: add native shell skeleton"
```

Expected: commit succeeds.

## Integration Gate 1

**Agent:** Controller.

- [ ] **Step 1: Register Wave 1 modules**

Modify `src-tauri/src/lib.rs` to include:

```rust
mod db;
mod domain;
mod parser;
mod repositories;
mod windowing;
```

- [ ] **Step 2: Run combined checks**

Run:

```powershell
npm test
npm run build
cd src-tauri
cargo test
cd ..
```

Expected: all checks pass.

- [ ] **Step 3: Commit integration**

Run:

```powershell
git add src-tauri/src/lib.rs
git commit -m "chore: wire wave one modules"
```

Expected: commit succeeds.

## Wave 2 Parallel Dispatch

Dispatch Tasks 5-7 after Integration Gate 1 passes. These tasks touch connected behavior, so the controller must verify file ownership before dispatch.

## Task 5: Codex Parser Provider

**Agent:** Codex parser worker.

**Files:**

- Create: `src-tauri/src/parser/codex_provider.rs`
- Create: `src-tauri/src/parser/prompt.rs`
- Create: `src-tauri/src/parser/validate.rs`
- Test: parser provider tests inside created modules

- [ ] **Step 1: Write failing provider tests**

Create tests for:

```rust
#[test]
fn builds_codex_exec_command_with_schema_and_output_file() {
    let command = CodexCommandBuilder::new("codex")
        .schema_path("schemas/parse-note.schema.json")
        .output_path("parse-result.json")
        .build();

    assert_eq!(command.program, "codex");
    assert_eq!(command.args, vec![
        "exec",
        "--ephemeral",
        "--skip-git-repo-check",
        "--output-schema",
        "schemas/parse-note.schema.json",
        "-o",
        "parse-result.json",
        "-"
    ]);
}
```

```rust
#[test]
fn validates_schema_and_rejects_missing_summary() {
    let invalid = serde_json::json!({
      "cleanedText": "Clean note",
      "tags": [],
      "actionItems": []
    });

    assert!(validate_parser_json(&invalid).is_err());
}
```

Run:

```powershell
cd src-tauri
cargo test builds_codex_exec_command_with_schema_and_output_file validates_schema_and_rejects_missing_summary
```

Expected before implementation: tests fail because provider modules do not exist.

- [ ] **Step 2: Implement prompt**

`src-tauri/src/parser/prompt.rs` must produce a prompt with these rules:

```text
You clean and organize a raw workplace note.
Return only JSON matching the provided schema.
Do not invent facts.
Preserve the meaning of the raw note.
Make cleanedText easier to scan and grammatically cleaner.
Use tags for people, projects, topics, urgency, category, and custom labels.
Extract action items when the note implies work to do.
Set requiresReview true for due dates, owners, commitments, or inferred obligations.
```

- [ ] **Step 3: Implement process execution**

Provider behavior:

- uses the checked-in schema path `schemas/parse-note.schema.json`
- creates a temporary output path
- pipes prompt and raw note to stdin
- enforces configured timeout
- reads output file
- validates JSON schema
- deserializes to `ParserResult`
- returns typed errors for missing command, timeout, non-zero exit, missing output, invalid JSON, and schema validation

- [ ] **Step 4: Verify provider tests**

Run:

```powershell
cd src-tauri
cargo test parser::codex_provider parser::validate parser::prompt
```

Expected: provider tests pass without invoking the real Codex CLI unless a test is explicitly marked ignored.

- [ ] **Step 5: Commit**

Run:

```powershell
git add src-tauri/src/parser/codex_provider.rs src-tauri/src/parser/prompt.rs src-tauri/src/parser/validate.rs
git commit -m "feat: add codex parser provider"
```

Expected: commit succeeds.

## Task 6: Frontend Workflows And Stores

**Agent:** Frontend workflow worker.

**Files:**

- Create: `src/lib/api.ts`
- Create: `src/lib/stores/notes.ts`
- Create: `src/lib/stores/captureDraft.ts`
- Create: `src/lib/stores/filters.ts`
- Create: `src/lib/types.ts`
- Modify: `src/App.svelte`
- Modify: `src/lib/components/QuickCapturePanel.svelte`
- Modify: `src/lib/components/InboxList.svelte`
- Create: `src/lib/components/NoteDetail.svelte`
- Create: `src/lib/components/ReviewQueue.svelte`
- Create: `src/lib/components/SettingsView.svelte`
- Test: `src/lib/stores/*.test.ts`, component tests as needed

- [ ] **Step 1: Write failing store tests**

Create tests proving:

```ts
import { describe, expect, it } from "vitest";
import { createInboxFilters, matchesNoteFilters } from "./filters";

describe("inbox filters", () => {
  it("matches notes by parse status and review status", () => {
    const filters = createInboxFilters({
      parseStatus: "failed",
      reviewStatus: "needs_review"
    });

    expect(matchesNoteFilters({
      id: "n1",
      summary: "Deploy moved",
      cleanedText: "Deploy moved to Friday",
      rawText: "deploy friday",
      tags: [],
      parseStatus: "failed",
      reviewStatus: "needs_review",
      createdAt: "2026-05-20T12:00:00Z"
    }, filters)).toBe(true);
  });
});
```

Run:

```powershell
npm test -- src/lib/stores
```

Expected before implementation: tests fail because store modules do not exist.

- [ ] **Step 2: Implement frontend types**

`src/lib/types.ts` must define:

```ts
export type ParseStatus = "queued" | "parsing" | "parsed" | "failed";
export type ReviewStatus = "none" | "needs_review" | "reviewed";
export type TagKind = "person" | "project" | "topic" | "urgency" | "category" | "custom";
export type ActionStatus = "suggested" | "accepted" | "dismissed" | "done";
```

Include `NoteListItem`, `NoteDetail`, `Tag`, `ActionItem`, and `InboxFilters`.

- [ ] **Step 3: Implement API wrapper**

`src/lib/api.ts` must wrap Tauri `invoke` calls:

```ts
export const api = {
  saveCaptureNote(rawText: string): Promise<NoteDetail>,
  listInbox(filters: InboxFilters): Promise<NoteListItem[]>,
  getNote(id: string): Promise<NoteDetail>,
  retryParse(noteId: string): Promise<void>,
  acceptActionItem(actionId: string): Promise<void>,
  dismissActionItem(actionId: string): Promise<void>,
  getSettings(): Promise<AppSettings>,
  saveSettings(settings: AppSettings): Promise<AppSettings>
};
```

- [ ] **Step 4: Implement UI workflows**

Requirements:

- Quick capture calls `api.saveCaptureNote`.
- Inbox reloads after save.
- Inbox shows parse status and review badges.
- Detail view shows raw and cleaned text.
- Review queue can accept or dismiss suggested action items.
- Settings shows hotkey, parser timeout, Codex command path, selected theme.

- [ ] **Step 5: Verify frontend**

Run:

```powershell
npm test
npm run build
```

Expected: tests and build pass.

- [ ] **Step 6: Commit**

Run:

```powershell
git add src/lib/api.ts src/lib/stores src/lib/types.ts src/lib/components src/App.svelte
git commit -m "feat: add note workflow ui"
```

Expected: commit succeeds.

## Task 7: Tauri Commands And Service Wiring

**Agent:** Backend command integration worker.

**Files:**

- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/services/mod.rs`
- Create: `src-tauri/src/services/capture.rs`
- Create: `src-tauri/src/services/draft.rs`
- Create: `src-tauri/src/services/search.rs`
- Create: `src-tauri/src/services/parse_queue.rs`
- Create: `src-tauri/src/services/settings.rs`
- Test: unit tests inside service modules

- [ ] **Step 1: Write failing service tests**

Create tests proving:

```rust
#[test]
fn capture_service_saves_raw_note_and_enqueues_parse() {
    let state = test_app_state();
    let service = CaptureService::new(state.repositories.clone());

    let note = service.capture("Mike said deploy moved to Friday").unwrap();

    assert_eq!(note.raw_text, "Mike said deploy moved to Friday");
    assert_eq!(note.parse_status, ParseStatus::Queued);
    assert!(state.repositories.parse_jobs.next_queued().unwrap().is_some());
}
```

```rust
#[test]
fn parse_queue_marks_failed_job_without_modifying_raw_note() {
    let state = test_app_state();
    let note = state.repositories.notes.create_raw_note("raw note").unwrap();
    let job = state.repositories.parse_jobs.enqueue(note.id).unwrap();

    ParseQueue::new(state.repositories.clone()).mark_failed(job.id, "invalid json").unwrap();

    let stored = state.repositories.notes.get(note.id).unwrap().unwrap();
    assert_eq!(stored.raw_text, "raw note");
    assert_eq!(stored.parse_status, ParseStatus::Failed);
}
```

Run:

```powershell
cd src-tauri
cargo test capture_service_saves_raw_note_and_enqueues_parse parse_queue_marks_failed_job_without_modifying_raw_note
```

Expected before implementation: tests fail because service modules do not exist.

- [ ] **Step 2: Implement app state**

`AppState` must hold:

- shared database owner based on `Arc<Mutex<rusqlite::Connection>>` for v1
- repositories
- settings service
- parse queue service
- parser provider configuration

- [ ] **Step 3: Implement Tauri commands**

Expose commands:

```rust
#[tauri::command]
pub async fn save_capture_note(state: tauri::State<'_, AppState>, raw_text: String) -> Result<NoteDetailDto, CommandError>;

#[tauri::command]
pub async fn list_inbox(state: tauri::State<'_, AppState>, filters: InboxFiltersDto) -> Result<Vec<NoteListItemDto>, CommandError>;

#[tauri::command]
pub async fn get_note(state: tauri::State<'_, AppState>, id: String) -> Result<NoteDetailDto, CommandError>;

#[tauri::command]
pub async fn retry_parse(state: tauri::State<'_, AppState>, note_id: String) -> Result<(), CommandError>;

#[tauri::command]
pub async fn accept_action_item(state: tauri::State<'_, AppState>, action_id: String) -> Result<(), CommandError>;

#[tauri::command]
pub async fn dismiss_action_item(state: tauri::State<'_, AppState>, action_id: String) -> Result<(), CommandError>;
```

- [ ] **Step 4: Implement background parse queue loop**

The loop must:

- claim one queued job at a time
- mark job `parsing`
- invoke configured parser provider
- store parse run
- apply parser result
- mark parsed or failed
- sleep briefly when no jobs are available
- stop cleanly when app shuts down

- [ ] **Step 5: Verify service tests**

Run:

```powershell
cd src-tauri
cargo test services commands
```

Expected: command and service tests pass.

- [ ] **Step 6: Commit**

Run:

```powershell
git add src-tauri/src/app_state.rs src-tauri/src/commands.rs src-tauri/src/services
git commit -m "feat: wire note services and commands"
```

Expected: commit succeeds.

## Integration Gate 2

**Agent:** Controller.

- [ ] **Step 1: Register Wave 2 modules and commands**

Modify `src-tauri/src/lib.rs` to include:

```rust
mod app_state;
mod commands;
mod services;
```

Register Tauri commands:

```rust
.invoke_handler(tauri::generate_handler![
    commands::save_capture_note,
    commands::list_inbox,
    commands::get_note,
    commands::retry_parse,
    commands::accept_action_item,
    commands::dismiss_action_item
])
```

- [ ] **Step 2: Register frontend command names**

Confirm `src/lib/api.ts` invokes command names exactly:

```ts
"save_capture_note"
"list_inbox"
"get_note"
"retry_parse"
"accept_action_item"
"dismiss_action_item"
```

- [ ] **Step 3: Run combined checks**

Run:

```powershell
npm test
npm run build
cd src-tauri
cargo test
cd ..
```

Expected: all checks pass.

- [ ] **Step 4: Commit integration**

Run:

```powershell
git add src-tauri/src/lib.rs src/lib/api.ts
git commit -m "chore: integrate note commands"
```

Expected: commit succeeds.

## Task 8: Windows Runtime Integration

**Agent:** Native integration worker.

**Files:**

- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/windowing/**`
- Modify: `src-tauri/src/services/settings.rs`

- [ ] **Step 1: Configure windows**

`tauri.conf.json` must include:

- `main` window for inbox/settings/detail
- `quick-capture` window hidden by default
- quick capture dimensions around `560x260`
- quick capture always-on-top behavior in Rust setup
- no initial focus theft from main window at startup

- [ ] **Step 2: Configure permissions**

`src-tauri/capabilities/default.json` must allow only needed global shortcut permissions:

```json
{
  "permissions": [
    "global-shortcut:allow-is-registered",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister"
  ]
}
```

Keep additional scaffold-required permissions if the Tauri template added them for app startup.

- [ ] **Step 3: Wire tray and hotkey**

Runtime behavior:

- single instance plugin prevents duplicate apps
- tray menu includes `Open`, `Quick Note`, and `Quit`
- global hotkey opens quick capture
- quick capture positions bottom-right on the active monitor
- quick capture focuses the note field

- [ ] **Step 4: Verify Windows behavior manually**

Run:

```powershell
npm run tauri dev
```

Manual checks:

- tray icon appears
- hotkey opens quick capture
- quick capture appears bottom-right
- `Enter` saves and closes
- `Shift+Enter` inserts newline
- `Esc` closes and draft remains
- main inbox can open from tray

- [ ] **Step 5: Run automated checks**

Run:

```powershell
npm test
npm run build
cd src-tauri
cargo test
cd ..
```

Expected: all checks pass.

- [ ] **Step 6: Commit**

Run:

```powershell
git add src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/capabilities/default.json src-tauri/src/windowing src-tauri/src/services/settings.rs
git commit -m "feat: integrate windows tray and hotkey"
```

Expected: commit succeeds.

## Task 9: Final Verification, Docs, And Release Notes

**Agent:** QA and docs worker.

**Files:**

- Create: `README.md`
- Create: `docs/development.md`
- Create: `docs/verification/2026-05-20-v1-verification.md`
- Modify: `docs/superpowers/plans/2026-05-20-work-notes-agent-team-implementation.md` only to check completed boxes if the controller wants tracked status

- [ ] **Step 1: Create README**

`README.md` must include:

- app purpose
- stack
- dev setup
- test commands
- how Codex parsing works
- statement that OpenAI API keys are not used
- Windows manual verification steps

- [ ] **Step 2: Create development doc**

`docs/development.md` must include:

```powershell
npm install
npm run tauri dev
npm test
npm run build
cd src-tauri
cargo test
```

It must also document:

- SQLite database location
- Codex CLI requirement
- parser timeout setting
- theme token rule
- how to add a developer-defined theme

- [ ] **Step 3: Create verification report**

`docs/verification/2026-05-20-v1-verification.md` must record command outputs for:

```powershell
npm test
npm run build
cd src-tauri
cargo test
cd ..
```

It must also record manual Windows checks:

- tray app starts
- hotkey opens quick capture
- quick capture appears bottom-right
- note saves instantly
- parser runs automatically
- parser failure leaves note intact
- inbox search finds raw and cleaned text
- default theme is Dark Compact

- [ ] **Step 4: Run final checks**

Run:

```powershell
npm test
npm run build
cd src-tauri
cargo test
cd ..
git status --short
```

Expected:

```text
npm test exits 0
npm run build exits 0
cargo test exits 0
git status shows only README/docs changes before commit
```

- [ ] **Step 5: Commit docs**

Run:

```powershell
git add README.md docs/development.md docs/verification/2026-05-20-v1-verification.md
git commit -m "docs: add development and verification guide"
```

Expected: commit succeeds.

## Final Team Review

After all tasks finish, the controller dispatches two review-only agents.

### Review Agent A: Spec Compliance

Prompt:

```text
Review the implementation against docs/superpowers/specs/2026-05-20-work-notes-design.md.

Focus on missing or overbuilt behavior. Confirm:
- Windows-only Tauri/Svelte/Rust/SQLite stack
- raw notes save immediately and are immutable
- quick capture is bottom-right and keyboard-driven
- automatic parsing uses local codex exec, not OpenAI API keys
- parser output auto-applies cleaned text, summary, and tags
- action items remain suggested until user action
- inbox-first organization
- Dark Compact tokenized theme

Return findings first with file/line references. If no issues, say so clearly and identify residual risk.
```

### Review Agent B: Code Quality

Prompt:

```text
Review the code quality of the Work Notes implementation.

Focus on bugs, data-loss risks, process-execution risks, unsafe UI assumptions, weak tests, and violations of SOLID/KISS/DRY/Hollywood principle.

Return findings first with file/line references. Prioritize issues that can cause data loss, parser corruption, broken hotkey/tray behavior, or hard-to-maintain service coupling.
```

## Final Acceptance Commands

Run from repo root:

```powershell
npm test
npm run build
cd src-tauri
cargo test
cd ..
git status --short --branch
git log --oneline -10
```

Expected:

```text
npm test exits 0
npm run build exits 0
cargo test exits 0
git status is clean on feature/work-notes-v1
recent commits correspond to scaffold, persistence, parser, UI, native shell, integration, docs
```

## Execution Choice

Recommended execution for this plan:

1. **Agent-Team Dispatch**: controller dispatches the scaffold task first, then parallel waves with disjoint file ownership, then integration gates and review agents.
2. **Inline Sequential**: controller executes tasks one at a time in this session if parallel execution creates too much coordination overhead.

Use Agent-Team Dispatch unless the scaffold or first integration gate exposes toolchain instability.
