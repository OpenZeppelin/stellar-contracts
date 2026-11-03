---
name: version-bump
description: Bump the stellar-contracts version across READMEs and Cargo.toml files, then refresh the Cargo.lock files
user_invocable: true
---

# Version Bump

Steps 1–2 of `/release`; also runnable on its own. Ask for the new version, and confirm the old one (`[workspace.package] version` in the root `Cargo.toml`). The working tree must be clean. List every file to be changed and wait for confirmation before editing.

## 1. Docs

- READMEs: update every `"=OLD"` pin of a `stellar-*` crate. Add new packages, modules and examples, and remove deleted ones.
- New packages also go in the root README and the `CLAUDE.md` repository map. Check `Architecture.md` for added or removed modules.

## 2. Cargo.toml

- Root: `[workspace.package] version` and the `stellar-*` entries in `[workspace.dependencies]`. Never change third-party versions.
- Crates in the root `exclude` list pin the version themselves: `examples/rwa/sign-claim`, `bench/rwa-batch` (its own version and its `stellar-*` deps).

## 3. Lockfiles

`cargo build` at the root, and `cargo build --manifest-path bench/rwa-batch/Cargo.toml`. Never regenerate a lockfile from scratch.

## 4. Verify

```bash
grep -rn '"=OLD"' --include='*.md' .
grep -rnE 'version = "=?OLD"' --include=Cargo.toml . | grep -v /target/   # review hits: third-party deps may match
```

Both `Cargo.lock` files show up in `git diff`.
