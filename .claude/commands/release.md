---
name: release
description: Orchestrate a full stellar-contracts release end-to-end (READMEs, version bump, audit report, Wizard, docs site, release branch, release notes) with a confirmation gate before every step
user_invocable: true
---

# Release Orchestrator

## Context

Publishing a new release of stellar-contracts touches several repos and services, not just this one. This skill walks the full checklist step by step. It does NOT execute everything in one go — every step passes through the Gate Pattern below.

Repos involved (adjust paths if the user's checkout differs):

| Repo               | Local path                                     | Role                                             |
| ------------------ | ---------------------------------------------- | ------------------------------------------------ |
| stellar-contracts  | `~/Developer/OpenZeppelin/stellar-contracts`   | The library being released                       |
| contracts-wizard   | `~/Developer/OpenZeppelin/contracts-wizard`    | Wizard UI (`packages/core/stellar`)              |
| docs               | `~/Developer/OpenZeppelin/docs`                | docs.openzeppelin.com content (`content/stellar-contracts/`) |

Companion skills this orchestrator delegates to. Each lives in the repo it operates on — prefer
the user's local checkout of that repo; fall back to fetching from GitHub:

| Step      | Skill (path within its repo)                                    | GitHub                                                                                                    |
| --------- | ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Steps 1–2 | `.claude/commands/version-bump.md` (this repo)                   | https://github.com/OpenZeppelin/stellar-contracts/blob/main/.claude/commands/version-bump.md               |
| Step 4    | contracts-wizard: `.claude/skills/stellar-release-update/SKILL.md` | https://github.com/OpenZeppelin/contracts-wizard/blob/master/.claude/skills/stellar-release-update/SKILL.md |
| Step 5    | docs: `.claude/skills/update-stellar-docs.md`                    | https://github.com/OpenZeppelin/docs/blob/main/.claude/skills/update-stellar-docs.md                        |

## The Gate Pattern (mandatory)

For EVERY step below, follow this three-phase gate. Never merge phases, never skip ahead to implementation.

1. **Relevance check** — First determine whether the step is needed at all. Gather the evidence (diff the release range, check for open PRs, look for new modules, etc.), present a short summary of what you found, and ask the user: *"Step N is [needed / probably not needed] because <evidence>. Should I proceed, or skip it?"*
2. **Plan** — If proceeding, inspect the affected code/files and present the big picture of what would change: which files, what kind of edits, anything risky or ambiguous. Do NOT make any changes yet. Ask for confirmation on the plan.
3. **Apply** — Only after the user confirms the plan, implement it. Then report what was done and show the updated checklist before moving to the next step's gate.

Between steps, re-print the checklist with current status (`[x]` done, `[~]` skipped with reason, `[ ]` pending) so the user always sees where the release stands.

## Inputs

At the start, ask the user for:

1. The **new version number** (e.g., `0.9.0`) and whether it is a final release or an RC
2. The **previous release tag** to diff against (e.g., `v0.8.0`) — verify it exists with `git tag`
3. Where the **final audit report PDF** is (a local path), or "not available yet"

Then run a preflight in stellar-contracts: `git status` is clean, `git fetch --tags` done, and note which branch you're on. Compute the release diff once (`git log <prev_tag>..HEAD --oneline`, `git diff <prev_tag>..HEAD --stat`) and reuse it as evidence in the gates below.

## Checklist (tracking template)

```
[ ] 1. Review GH READMEs and update if necessary
[ ] 2. Update `version` in Cargo.toml + `cargo build` to refresh Cargo.lock
[ ] 3. Upload final audit report to /audits (renamed to match pattern)
[ ] 4. Merge Wizard PR if any
[ ] 5. docs.openzeppelin.com — new module docs + version update
[ ] 6. Create `release-v<X.Y.Z>` branch (triggers Netlify publish webhook)
[ ] 7. Release notes mentioning all contributors (optional)
[ ] 8. Skill maintenance sweep — update the release skills themselves
```

## Steps

### 1 + 2. READMEs, version bump, build

These two checklist items are already covered by the `version-bump` skill in this repo. 

- **Relevance check**: always needed for a release — but confirm the version number one more time before starting.
- **Plan / Apply**: invoke the `version-bump` skill (`.claude/commands/version-bump.md`) and follow it. Keep the Gate Pattern inside it too: present the list of README edits it wants to make before making them.
- Done when: `cargo build` passes, `Cargo.lock` reflects the new version, and `grep -rn '"=OLD_VERSION"' --include='*.md' --include='Cargo.toml'` comes back clean.

### 3. Audit report

- **Relevance check**: skip (with `[~]`) if the user said the report isn't available yet, or if this is an RC that wasn't audited. Ask which audit this release corresponds to.
- **Plan**: look at `audits/` to confirm the current naming pattern. As of writing it is:
  `Stellar Contracts Library v<X.Y.Z> Audit.pdf` (with a `Re-Audit` variant for follow-ups, e.g. `Stellar Contracts Library v0.5.0 Re-Audit.pdf`). Show the user the exact target filename before copying.
- **Apply**: copy the PDF from the user-provided path into `audits/` under the confirmed name. Verify with `ls audits/`.

### 4. Wizard PR

- **Relevance check**: two things to check, in order:
  1. Is there already an open Wizard PR for this release? Check with:
     `gh pr list --repo OpenZeppelin/contracts-wizard --state open --search "stellar"`
  2. If no PR exists — does this release even need Wizard changes? Scan the release diff for changes to public traits, function signatures, or new user-facing contracts that the Wizard generates code for (`packages/core/stellar` in contracts-wizard). Internal-only changes need no Wizard update.
- **Plan**:
  - If an open PR exists: summarize the PR (title, files touched, CI status) and ask whether to merge it now (`gh pr merge`). Merging is outward-facing — never merge without explicit confirmation.
  - If no PR exists but changes are needed: follow the wizard sync skill (`.claude/skills/stellar-release-update/SKILL.md` in the contracts-wizard repo — see the companion table). Present its upstream-change analysis and update plan as the gate before touching wizard code.
- **Apply**: merge the PR, or implement the wizard changes and open a PR, per the confirmed plan.

### 5. docs.openzeppelin.com

> Note: the old checklist item "update version in `docs/antora.yml`" is obsolete — Antora docs were removed from this repo (PR #483, Oct 2025). Docs now live in the `docs` repo as `.mdx` under `content/stellar-contracts/`, with sidebar navigation in `src/navigation/stellar.json`.

- **Relevance check**: from the release diff, list new crates/modules/extensions and public API changes (breaking renames, removed APIs). If there are none, docs likely only need a version-reference sweep — say so and ask.
- **Plan / Apply**: follow the docs update skill (`.claude/skills/update-stellar-docs.md` in the docs repo — see the companion table) covering new pages, breaking-change sweeps, navigation registration, `pnpm run build` + `pnpm run check`. Present its Step-3 plan (which pages get created/edited) as the gate before writing. Docs changes go up as a PR to the docs repo — do not push directly.

### 6. Release branch

- **Relevance check**: confirm with the user that steps 1–5 are in the desired state and merged to `main`, and that they want to publish now. Creating this branch is the publish trigger — a `release-v<X.Y.Z>` branch pushed to origin fires a webhook to Netlify that publishes the new version to the docs site. Precedent: `origin/release-v0.2.0`, `origin/release-v0.3.0`, `origin/release-v0.7.2`.
- **Plan**: state the exact branch name (`release-v<X.Y.Z>`) and the commit it will point at (normally the tip of `main` after the release PR merged). 
- **Apply**: `git checkout main && git pull`, `git checkout -b release-v<X.Y.Z>`, `git push origin release-v<X.Y.Z>`. This is outward-facing and hard to reverse — require an explicit "yes" before pushing.

### 7. Release notes / contributors (optional)

- **Relevance check**: ask whether the user wants release notes drafted with a contributors section. This is optional per the checklist — skipping is fine.
- **Plan**: gather contributors for the release range:
  - `git log <prev_tag>..<new_ref> --format='%an|%ae' | sort -u` for names
  - prefer GitHub handles: `gh api repos/OpenZeppelin/stellar-contracts/compare/<prev_tag>...<new_ref> --jq '.commits[].author.login' | sort -u`
  - Draft the notes: highlights, breaking changes, new modules, audit reference, and a "Contributors" section thanking each handle. Show the full draft.
- **Apply**: on confirmation, create/update the GitHub release draft: `gh release create v<X.Y.Z> --draft --title "v<X.Y.Z>" --notes-file <draft>` (or `gh release edit` if it exists). Leave it as a draft unless the user explicitly asks to publish.

### 8. Skill maintenance sweep

The release skills themselves rot: they hardcode versions, file paths, naming patterns, and repo conventions that any release can invalidate. (Past example: this skill's docs step originally said "update `docs/antora.yml`" — a file that had been deleted from the repo entirely.) This step keeps them honest.

- **Relevance check**: run this sweep on every release — it is cheap and drift compounds. Start from what happened during THIS release run: conventions that turned out different from what a skill claimed, files that had moved or been renamed, steps that were skipped as obsolete, and manual work you did that no skill covered.
- **Plan**: read each skill in the family and diff it against the reality you just observed:
  - this repo: `.claude/commands/release.md` (this file) and `.claude/commands/version-bump.md` — paths, grep patterns, the audit filename pattern, the release-branch convention, line-number hints
  - contracts-wizard: `.claude/skills/stellar-release-update/SKILL.md` — key-file table, commands, version-file locations
  - docs repo: `.claude/skills/update-stellar-docs.md` — content paths, navigation file, build/check commands
  - Dev3 repo: the pointer skills under `languages/soroban/` (`soroban-release`, `soroban-update-wizard`, `soroban-update-external-docs`) — canonical URLs, default-branch names, repo paths
  Look specifically for: hardcoded version numbers or example tags that drifted, stale file paths, renamed traits/commands, changed default branches, changed naming patterns, and steps this run skipped or added. Present the proposed skill edits per file.
- **Apply**: make the confirmed edits in each repo's local checkout. Edits to skills in other repos go up as their own small PRs (or ride along with that repo's release-related PR) — state which route you took for each.

## Wrap-up

Print the final checklist with every item marked `[x]` or `[~] skipped: <reason>`, plus links to: the release PR(s), the Wizard PR, the docs PR, the pushed `release-v*` branch, the GitHub release draft, and any skill-maintenance PRs. Flag anything left for the user to do manually (e.g., publishing the draft release, verifying the Netlify deploy).
