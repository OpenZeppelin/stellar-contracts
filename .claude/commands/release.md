---
name: release
description: Run the full stellar-contracts release (READMEs, version bump, audit report, docs site, GitHub release and crates.io publish, Wizard, skill upkeep), confirming with the user at each step
user_invocable: true
---

# Release

Publishing the GitHub release `v<X.Y.Z>` creates the tag, and the tag triggers `.github/workflows/publish-crates.yml`, which publishes the crates it lists to crates.io (skipped for `-rc.` tags). A published crate can only be yanked, never deleted. No release branch is needed, except for a patch on an older version line.

## Rules

- **One gate per step**: show the evidence (is the step needed?) and the plan together, then wait for "proceed", "skip" or changes. Steps 1–3 share one gate.
- **Outward-facing actions** (open or merge a PR, push, create or publish a release) need their own "yes", with the exact command shown.
- Re-print the checklist after each step: `[x]` done, `[~]` skipped (reason), `[ ]` pending.
- Companion skills live in their own repos. Read them from the user's local checkout, or else from GitHub. If neither has the file, ask the user; don't improvise.

| Repo (default branch)        | Companion skill                                    |
| ---------------------------- | -------------------------------------------------- |
| stellar-contracts (`main`)   | `.claude/commands/version-bump.md`                 |
| docs (`main`)                | `.claude/skills/update-stellar-docs.md`            |
| contracts-wizard (`master`)  | `.claude/skills/stellar-release-update/SKILL.md`   |
| Dev3 (`main`)                | pointer skills in `languages/soroban/`             |

## Inputs and preflight

Ask for: the new version (`-rc.N` means RC), the release line (`main`, or a patch on an older line), and the audit PDF path (or "not available").

- Patch on an older line: reuse that line's branch if one exists, otherwise create one from the line's last tag (pushing it is outward-facing).
- `git status` is clean; `git fetch origin --tags`; `v<X.Y.Z>` exists neither as a tag nor as a release.
- Record `PREFLIGHT_SHA` = tip of `origin/main` (or of the line's branch).
- `PREV_TAG` = newest non-RC tag that is an ancestor of it: `git tag --merged <PREFLIGHT_SHA> --sort=-v:refname | grep -v -- -rc | head -1`. Confirm it with the user.
- Release diff, reused as evidence by every gate: `git log --oneline` and `git diff --stat` over `<PREV_TAG>..<PREFLIGHT_SHA>`.

## Checklist

```text
[ ] 1. Update READMEs, Architecture.md and the CLAUDE.md repository map
[ ] 2. Bump the version and refresh the Cargo.lock files
[ ] 3. Add the audit report to audits/
[ ] 4. Update docs.openzeppelin.com
[ ] 5. Merge the release PR and record RELEASE_SHA
[ ] 6. Draft the GitHub release
[ ] 7. Publish the GitHub release (creates the tag, publishes to crates.io)
[ ] 8. Merge the Wizard PR
[ ] 9. Update the release skills
```

### 1–3. Prepare the release PR

On a new branch from `PREFLIGHT_SHA`: follow `version-bump` for steps 1–2. Step 3: copy the PDF into `audits/`, matching the naming pattern already used there; skip if there's no report or it's an unaudited RC.

If a package is new, or about to be published for the first time, add it to `publish-crates.yml`: a publish step after the crates it depends on, and a dry-run step (or a `# Skipped:` note if it depends on unpublished workspace versions). Ask the user which packages should be published.

Open the PR.

### 4. Update the docs site

Follow the docs companion skill. Its plan of page changes is this gate. The changes go up as a PR to the docs repo, which can merge before the release is published.

### 5. Merge the release PR and record RELEASE_SHA

Once the release PR is merged: fetch, set `RELEASE_SHA` = tip of the base branch, and show `git log <PREFLIGHT_SHA>..<RELEASE_SHA> --oneline`. Those commits weren't in the evidence, so ask whether they belong in the release. Every later command uses `RELEASE_SHA`.

### 6. Draft the release

Generate the detailed notes:

```bash
gh api repos/OpenZeppelin/stellar-contracts/releases/generate-notes \
  -f tag_name=v<X.Y.Z> -f target_commitish=<RELEASE_SHA> -f previous_tag_name=<PREV_TAG> --jq .body
```

Above that, write sections in the same format as the last release (`gh release view <PREV_TAG>`): summary, breaking changes, security. Show the full draft, then:

```bash
gh release create v<X.Y.Z> --draft --target <RELEASE_SHA> --title v<X.Y.Z> --notes-file <file> \
  [--prerelease]  # RC
  [--latest=false]  # patch on an older line
```

A draft does not create the tag.

### 7. Publish the release

Before asking, show: the draft targets `RELEASE_SHA`, the tag doesn't exist yet, and CI is green on `RELEASE_SHA`. Then ask for "yes, publish v<X.Y.Z>".

Run `gh release edit v<X.Y.Z> --draft=false`, then check that the tag points at `RELEASE_SHA` and watch the `publish-crates.yml` run. If it fails partway, report which crates were published and stop.

### 8. Merge the Wizard PR

The Wizard's CI compiles against crates.io, so its PR merges only after step 7. Look for the PR by version (`v<X.Y.Z>` in the title, branch `wizard/stellar-v<X.Y.Z>`), not by "stellar". If there's no PR, follow the Wizard companion skill. RCs normally skip this step.

### 9. Update the release skills

List what didn't match the skills during this run: moved files, changed conventions, obsolete or missing steps. Propose edits to the affected skill files only; skip this step if nothing came up. For Dev3, edit `languages/soroban/`, not `build/`.

## Wrap-up

Print the final checklist, `RELEASE_SHA`, and links to every PR, the release, and the publish run. List what's left for the user.
