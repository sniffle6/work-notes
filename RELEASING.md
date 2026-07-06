# Releasing Work Notes

Prerequisites (one-time): the updater signing keypair exists, its public key is
in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`), and the CI secrets
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are set.

## Cut a release

1. Bump the version everywhere it must stay in sync, in one step:
   ```
   node scripts/bump-version.mjs X.Y.Z
   ```
   This updates `src-tauri/tauri.conf.json` (the updater's source of truth),
   `src-tauri/Cargo.toml`, `package.json`, and `src-tauri/Cargo.lock` together,
   and writes nothing if any file fails to match (no half-bumped releases).
2. Commit: `git commit -am "chore: release vX.Y.Z"`
3. Push the release commit to the branch, then tag and push the tag:
   ```
   git push origin HEAD
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   Push the branch first (`git push origin HEAD`) so the version-bump commit
   lands on the default-branch history — otherwise it is reachable only via the
   tag and the next release starts from a stale base. The tag push triggers CI.
4. GitHub Actions builds, signs, and creates a **draft** release with the NSIS
   installer, its `.sig`, and `latest.json`.
5. Open the draft release on GitHub, confirm the three assets are attached, and
   click **Publish**. Publishing makes it the "Latest" release, which activates
   the updater permalink for existing installs.

## Notes

- Installed apps check for updates on launch and via the tray "Check for
  updates" item. They read
  `https://github.com/sniffle6/work-notes/releases/latest/download/latest.json`.
- The installer is currently unsigned (no code-signing cert), so first-time
  installs show a one-time Windows SmartScreen warning. This is expected until
  a certificate is added.
