---
name: version-bump
description: Bump the stellar-contracts version across READMEs and Cargo.toml, then build to refresh Cargo.lock
user_invocable: true
---

# Version Bump

## Context

Moving stellar-contracts to a new version number means updating every README that pins a version, the workspace `Cargo.toml`, and then building to refresh `Cargo.lock`. This skill handles those mechanical edits.

It covers steps 1–2 of the full release checklist in `.claude/commands/release.md`, and is safe to run on its own for a version bump outside a release cut.

## Workflow

When the user invokes this skill, ask for:
1. The **new version number** (e.g., `0.7.0`)
2. Whether there are any new packages since the last release

Then follow these steps:

### 1. Review and Update READMEs

Read every `README.md` in the repo (`packages/*/README.md`, root `README.md`, and any example READMEs).

For each README:
- **Version references**: Update all `"=X.Y.Z"` dependency version strings to the new version. Be careful not to change third-party dependency versions (e.g., `serde-json-core`).
- **New packages**: If new packages were added since the last release, add them to:
  - Root `README.md` project structure section
  - Root `README.md` dependency examples section
- **New modules/extensions**: If existing packages gained new modules or extensions, add them to the relevant package README (e.g., new token extensions, new governance modules).
- **Removed features**: If derive macros, traits, or modules were removed, update the README sections that reference them.
- **Examples**: If new example contracts were added, add them to the relevant package README's examples section.

### 2. Update Version in Cargo.toml

Update `version = "X.Y.Z"` in the root `Cargo.toml`:
- `[workspace.package]` version (line ~52)
- All `stellar-*` workspace dependency entries under `[workspace.dependencies]`
- **Do NOT change** third-party dependency versions (e.g., `serde-json-core`, `soroban-sdk`)

Also check for any example `Cargo.toml` files that pin the workspace version (e.g., `examples/rwa/sign-claim/Cargo.toml`).

Verify with: `grep -rn 'version = "OLD_VERSION"' --include='Cargo.toml'` to catch any stragglers (excluding third-party deps).

### 3. Run Cargo Build

Run `cargo build` to:
- Verify the version bump doesn't break compilation
- Update `Cargo.lock` with the new version numbers

Command: `cargo build`

### 4. Verify

- Grep for any remaining old version references: `grep -rn '"=OLD_VERSION"' --include='*.md' --include='Cargo.toml'`
- Confirm `Cargo.lock` was updated (check git diff)
- Confirm build succeeded

## Key Files

| Purpose             | Path                                               |
| ------------------- | -------------------------------------------------- |
| Workspace config    | `Cargo.toml`                                       |
| Root README         | `README.md`                                        |
| Package READMEs     | `packages/*/README.md`                             |
| Example Cargo files | `examples/*/Cargo.toml`, `examples/*/*/Cargo.toml` |
| Lock file           | `Cargo.lock`                                       |
