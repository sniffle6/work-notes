# Releasing Work Notes

Prerequisites (one-time): the updater signing keypair exists, its public key is
in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`), and the CI secrets
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are set.

For a Codex-driven release or deployment-flow test, use
`.codex/skills/work-notes-release-build`. A dry run must not commit, tag, push,
trigger CI, edit a GitHub release, or publish.

Release notes in `docs/releases/vX.Y.Z.md` are the public GitHub release body.
Write them for Work Notes users, and keep verification evidence outside that
file.

## Cut a release

1. Bump the version everywhere it must stay in sync, in one step:
   ```
   node scripts/bump-version.mjs X.Y.Z
   ```
   This updates `src-tauri/tauri.conf.json` (the updater's source of truth),
   `src-tauri/Cargo.toml`, `package.json`, and `src-tauri/Cargo.lock` together,
   and writes nothing if any file fails to match (no half-bumped releases).
2. Draft release notes in `docs/releases/vX.Y.Z.md` before pushing the release
   tag. Include user-facing highlights, notable fixes, and any install/update
   caveats. Do not publish a release with the CI placeholder body.
3. Run release verification, then record the evidence in the Codex handoff,
   release checklist, or a separate
   `docs/verification/YYYY-MM-DD-vX.Y.Z-verification.md` file. Do not put
   verification output in `docs/releases/vX.Y.Z.md`:
   ```
   npm test
   npm run check
   npm run build
   scripts\cargo-test.cmd
   ```
4. Commit the version bump and release notes:
   ```
   git add package.json src-tauri\tauri.conf.json src-tauri\Cargo.toml src-tauri\Cargo.lock docs\releases\vX.Y.Z.md
   git commit -m "chore: release vX.Y.Z"
   ```
5. Push the release commit to the branch, then tag and push the tag:
   ```
   git push origin HEAD
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   Push the branch first (`git push origin HEAD`) so the version-bump commit
   lands on the default-branch history — otherwise it is reachable only via the
   tag and the next release starts from a stale base. The tag push triggers CI.
6. GitHub Actions builds, signs, and creates a **draft** release with the NSIS
   installer, its `.sig`, and `latest.json`.
7. Confirm the three assets are attached, replace the placeholder body with the
   release notes, and publish the draft:
   ```
   gh release view vX.Y.Z --json isDraft,assets,body
   gh release edit vX.Y.Z --notes-file docs\releases\vX.Y.Z.md --draft=false
   ```
   Publishing makes it the "Latest" release, which activates the updater
   permalink for existing installs.

## Notes

- Installed apps check for updates on launch and via the tray "Check for
  updates" item. They read
  `https://github.com/sniffle6/work-notes/releases/latest/download/latest.json`.
- The installer is currently unsigned (no code-signing cert), so first-time
  installs show a one-time Windows SmartScreen warning. This is expected until
  a certificate is added.
