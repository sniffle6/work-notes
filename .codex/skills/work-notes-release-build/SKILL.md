---
name: work-notes-release-build
description: Use when cutting, dry-running, tagging, building, drafting, publishing, testing, or otherwise pushing a Work Notes GitHub release or updater build, especially when release notes must be prepared or applied before a draft release is published.
---

# Work Notes Release Build

## Overview

Release notes are a required release artifact for Work Notes. Do not push a release tag, trigger a release build, or publish a draft release until user-facing notes exist and the GitHub release body uses them.

## Required Context

- Read `RELEASING.md` before running release commands.
- Use `docs/testing.md` to choose verification. For a real release, prefer the full handoff set unless the user explicitly narrows scope.
- Treat `master` as a repo fact to verify, not an assumption. Confirm current branch, upstream, remote, and tags before pushing.
- Preserve unrelated local changes. Do not sweep local noise into a release commit.
- Use `docs/releases/vX.Y.Z.md` as the public GitHub release body for version `X.Y.Z`. Keep verification evidence outside this file.

## Dry Run Mode

Use dry run mode when testing this skill or preparing a release plan. Unless the user explicitly asks for a live deploy, start here.

- Do not run `node scripts\bump-version.mjs`.
- Do not commit, tag, push, trigger CI, edit a GitHub release, or publish.
- Inspect current version, latest `v*` tag, branch/upstream, remote, and pending changes.
- Draft or update public-facing `docs/releases/vX.Y.Z.md` from `<previous-tag>..HEAD` if the user wants an artifact; otherwise print the proposed notes.
- Print the exact live commands that would run.
- Stop for explicit approval before any command that changes files, pushes tags, triggers CI, or publishes a release.

## Workflow

1. Orient:
   ```powershell
   git status --short --branch
   git remote -v
   git fetch --tags --prune
   ```
   Identify the target version, current version, default branch, and previous `v*` tag.

2. Draft committed public release notes before publishing anything:
   ```powershell
   git describe --tags --abbrev=0 --match "v*"
   git log --oneline <previous-tag>..HEAD
   git diff --stat <previous-tag>..HEAD
   ```
   Write `docs/releases/vX.Y.Z.md` for Work Notes users. This file is passed to `gh release edit --notes-file`, so it should describe visible product changes and install/update caveats, not internal process evidence. Use these sections:
   ```markdown
   ## Highlights
   - ...

   ## Changes
   - ...

   ## Install And Update Notes
   - Windows installer is attached to the GitHub release.
   - Existing installs update from the latest-release `latest.json` after publish.
   ```

3. Bump and verify:
   ```powershell
   node scripts\bump-version.mjs X.Y.Z
   npm test
   npm run check
   npm run build
   scripts\cargo-test.cmd
   git status --short --branch
   ```
   Record verification evidence in the Codex handoff, release checklist, or a separate `docs/verification/YYYY-MM-DD-vX.Y.Z-verification.md` file. Do not add verification output to `docs/releases/vX.Y.Z.md`.

4. Commit and push the branch before the tag:
   ```powershell
   git add package.json src-tauri\tauri.conf.json src-tauri\Cargo.toml src-tauri\Cargo.lock docs\releases\vX.Y.Z.md
   git commit -m "chore: release vX.Y.Z"
   git push origin HEAD
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. Watch the release workflow and inspect the draft:
   ```powershell
   gh run list --workflow Release --limit 5
   gh run watch <run-id> --compact --exit-status
   gh release view vX.Y.Z --json isDraft,assets,body
   ```
   Confirm the draft exists and includes the NSIS installer, its `.sig`, and `latest.json`.

6. Apply notes and publish:
   ```powershell
   gh release edit vX.Y.Z --notes-file docs\releases\vX.Y.Z.md
   gh release edit vX.Y.Z --draft=false --latest
   gh release view vX.Y.Z --json isDraft,isPrerelease,isImmutable,assets,body,url
   ```
   The body must not be the CI placeholder. If the notes are not ready, leave the release as a draft.

## Rules

- Never publish with `Release notes pending` or another generic body.
- Run dry run mode before the first live use of this skill, or when the user asks to test the deployment flow.
- Keep public release notes in `docs/releases/vX.Y.Z.md` and commit them with the release bump.
- Do not include internal verification output, CI mechanics, commit hygiene, or skill/runbook changes in public release notes unless they directly affect users.
- Never rely on GitHub's auto-generated notes without reviewing and editing them into user-facing Work Notes notes.
- Push the version-bump branch before pushing the tag, so the next release starts from the bumped version.
- If CI creates the draft but assets are missing or the release body is still a placeholder, stop before publishing.
- Mention verification that actually ran in the handoff or release verification record. Do not claim commands passed unless their output was checked.

## Common Mistakes

- Publishing the draft from the browser before replacing the placeholder body. Fix: use `gh release edit vX.Y.Z --notes-file ... --draft=false`.
- Adding a `## Verification` section to the public GitHub release body. Fix: keep verification evidence in the handoff, checklist, or `docs/verification/...`.
- Tagging before committing the version bump and release notes to the branch. Fix: commit both, push `HEAD`, then push `vX.Y.Z`.
- Using a temporary notes file that is not committed. Fix: write `docs/releases/vX.Y.Z.md` and pass that file to `gh release edit`.
- Treating a build artifact as a release. Fix: a release is not done until the GitHub release has assets, notes, and published/latest state.
