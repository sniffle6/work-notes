#!/usr/bin/env node
// Bump the app version in every place it must stay in sync, in one step.
// Usage: node scripts/bump-version.mjs <new-version>   e.g. 0.1.3
//
// Updates: package.json, src-tauri/tauri.conf.json (the updater's source of
// truth), src-tauri/Cargo.toml [package].version, and the work-notes entry in
// src-tauri/Cargo.lock. Validates every match BEFORE writing anything, so a
// bad pattern can never leave the versions half-bumped. Prints the release
// commands to run next.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const next = process.argv[2];
if (!next || !/^\d+\.\d+\.\d+$/.test(next)) {
  console.error(`usage: node scripts/bump-version.mjs <new-version>  (e.g. 0.1.3)`);
  process.exit(2);
}

const current = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
if (current === next) {
  console.error(`version is already ${next}; nothing to do`);
  process.exit(2);
}

const semver = String.raw`\d+\.\d+\.\d+`;
// First `"version": "x.y.z"` is the top-level package version in each JSON file.
const jsonVersion = new RegExp(`("version":\\s*")${semver}(")`);
// Anchor on the work-notes package name (CRLF-safe) so we never touch a
// dependency version in Cargo.toml or another package in Cargo.lock.
const cargoAnchor = new RegExp(`(name = "work-notes"\\r?\\nversion = ")${semver}(")`);

const targets = [
  { path: "package.json", pattern: jsonVersion },
  { path: "src-tauri/tauri.conf.json", pattern: jsonVersion },
  { path: "src-tauri/Cargo.toml", pattern: cargoAnchor },
  { path: "src-tauri/Cargo.lock", pattern: cargoAnchor },
];

// Phase 1: read + validate every file matches before touching disk.
const edits = targets.map(({ path, pattern }) => {
  const full = join(root, path);
  const before = readFileSync(full, "utf8");
  if (!pattern.test(before)) {
    console.error(`ERROR: no version match in ${path} (pattern ${pattern}) — nothing written`);
    process.exit(1);
  }
  return { path, full, after: before.replace(pattern, `$1${next}$2`) };
});

// Phase 2: all matched — now write.
console.log(`Bumping ${current} -> ${next}:`);
for (const { path, full, after } of edits) {
  writeFileSync(full, after);
  console.log(`  ${path}`);
}

console.log(`\nNext:`);
console.log(`  git commit -am "chore: release v${next}"`);
console.log(`  git push origin HEAD`);
console.log(`  git tag v${next} && git push origin v${next}`);
console.log(`  # then publish the draft release GitHub Actions creates`);
