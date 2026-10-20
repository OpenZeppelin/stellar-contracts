# Confidential Token — Agent Guide

Scoped to `packages/confidential/`. The root `CLAUDE.md` still applies in full; this file only covers what is specific to this subtree.

## Orientation

The crate ships one token contract at its root (`src/lib.rs` / `src/storage.rs` / `src/test.rs`) plus three satellites, each with the standard `mod.rs` / `storage.rs` / `test.rs` shape:

| Path | Role |
|:---|:---|
| `src/lib.rs`, `src/storage.rs` | The `ConfidentialToken` trait — eleven entry points, of which `revoke_spender` is proofless — and the storage/orchestration layer |
| `src/verifier/` | Separate contract holding per-circuit UltraHonk verification keys |
| `src/auditor/` | Separate contract holding the auditor key registry |
| `src/compliance/` | `ComplianceHooks` — freeze, SAC passthrough, policy contract — plus the opt-in `ConfidentialClawback` trait (`clawback`, `force_revoke_spender`) |
| `circuits/` | Noir workspace, compiled by `nargo`, not `cargo` |
| `docs/` | The specification set; `docs/README.md` is its map (see below) |

Balances are Pedersen commitments on Grumpkin. Every operation that opens or re-randomizes a commitment carries a proof the contract forwards to the verifier contract; `deposit`, `merge`, and `revoke_spender` are proofless homomorphic folds.

## Errors

`ConfidentialTokenError` occupies **3500–3510**. The root guide's range list predates this module and omits it. Stay in the 3500s; do not open a new range for the satellites.

## Canonical encoding is a security boundary

The public-input blob is a positional concatenation of 32-byte big-endian `Bn254Fr` representatives, in the order given by each operation's table under `docs/protocol/operations/` (`docs/compliance.md#circuit` for the clawback circuit). Grumpkin points contribute two limbs (`x` then `y`).

Soroban's host silently reduces values `≥ r` modulo `r` rather than rejecting them, so `x` and `x + r` deserialise to the same field element. Every caller-supplied scalar and coordinate must therefore reach `verify_proof` through `append_field` / `append_point`, which call `Grumpkin::is_canonical_field` / `is_canonical_point`. Bypassing those helpers breaks byte-uniqueness of stored state and emitted events even though proofs still verify.

## Code cites the spec by path and anchor

Rust and Noir comments cite the docs as `docs/<file>.md#<heading-slug>` paths, and the docs cite each other with relative Markdown links. The `check-links` job in `.github/workflows/docs.yml` runs lychee over the Markdown and over a generated list of the code citations, and fails on a missing file or heading. Run its two steps locally (`brew install lychee`) after touching a heading, a file name, or a citation.

## Tests

Beyond the root guide's conventions:

- Proof verification is mocked. `MockVerifier` / `MockAuditor` in `src/test.rs` stand in for the real contracts, and proofs are empty `Bytes::new(e)`. Do not attempt to generate real proofs in Rust tests.
- Use `fixture_point` / `fixture_field`, not arbitrary bytes. The fixtures are canonical and on-curve; random values fail the canonicality guards before reaching any logic under test.
- Two mocks model proof semantics on purpose: the register mock in `src/test.rs` binds the first `acct_f` it sees, and `ClawbackReplayGuardVerifier` in `src/compliance/test.rs` binds the first whole blob, both standing in for UltraHonk's absorption of public inputs, so replay tests are meaningful. Keep that behaviour if either flow changes.

## Noir circuits

Run from `circuits/`; CI is `.github/workflows/noir.yml`. There is no `nargo fmt` step and no formatter config.

```bash
nargo check && nargo test
nargo info                          # per-package constraint counts
./scripts/extract_vks.sh            # regenerate vks/*.vk.json
./scripts/build_vk_bins.sh          # regenerate vks/*.vk.bin
./scripts/check_witness_lookups.sh  # no witness reaches a lookup table
```

Each change below spans several files and lands in one PR with all of its sites:

- **Adding a circuit.** The package, the `members` list in `circuits/Nargo.toml`, and the hardcoded `CIRCUITS` arrays in both `circuits/scripts/extract_vks.sh` and `circuits/scripts/build_vk_bins.sh`; then regenerate `constraints.baseline` and both VK formats.
- **Renaming a package.** Directory `transfer/` is package `circuit_transfer`. Both VK scripts derive `circuit_${name}` and `target/${pkg}.json` from the directory name, and every row of `constraints.baseline` is keyed on the package name.
- **Bumping the toolchain.** `NARGO_VERSION` and `BB_VERSION` in `noir.yml` are the source of truth and move together; the VKs are byte-sensitive to bb. Both versions are restated in the comment above `compiler_version` in every package `Nargo.toml` and in `circuits/vks/README.md`; the nargo version in the `constraints.baseline` header; the bb version in `circuits/scripts/check_witness_lookups.sh`, whose blackbox allowlist must be re-confirmed against the new bb, and in `README.md`, which names the bb version the on-chain verifier accepts. The CI installers are pinned by commit with SHA256 verification; bump URL and hash together, then regenerate the baseline and both VK formats.
- **`address_to_field` vectors.** The derivation has no Noir implementation (circuits take `addr_f` as an opaque public input), so `circuits/lib/testdata/address_to_field.json` is guarded only by `address_to_field_matches_testdata_vectors` in `src/test.rs`, which transcribes the hex values as literals rather than reading the JSON. Update both together.

Regenerating `constraints.baseline` with the command in its header overwrites that header, and CI strips comments before diffing, so nothing flags the loss. Restore the header by hand.

## The documentation set

`docs/` is a specification, not commentary, and it is the single largest maintenance hazard in this module — doc-consistency fixes outnumber code commits on this branch.

### Layout and normativity

`docs/README.md` maps the set, states which files are normative, and gives the precedence rules (`circuits/lib/src/lib.nr` outranks the docs on primitives; `#[contracttype]` definitions outrank them on shapes). One topic per file: new material goes into the file that owns its topic, and a new file is warranted only by a new topic. GitHub stops rendering LaTeX after roughly 750 expressions per page; one topic per file keeps every file well inside that.

### Facts with more than one home

Most facts now have exactly one owning section and are linked from everywhere else. Three still appear in more than one place because the second copy is a different artefact, not prose. Changing the owner means updating the copies in the same PR:

| Content | Owner | Copies live in |
|:---|:---|:---|
| `Clawback` is a `T₀` anchor alongside `Merge` | `docs/protocol/wallet-state.md#recovery` | `docs/compliance.md#wallet-and-auditor-consequences`, the role column of `docs/indexer.md#events-in-scope` |
| Per-circuit public-input counts | the `Public inputs` heading of each `docs/protocol/operations/*.md` and `docs/compliance.md#circuit` | header comments of `circuits/*/src/main.nr` |
| Domain-separation tag values | `docs/protocol/domain-separators.md` | `circuits/lib/src/lib.nr`, `circuits/lib/testdata/*.json` |

The tags are a cross-language wire contract. `docs/protocol/domain-separators.md` is their only authoritative source: it assigns every value, states which layer absorbs each, and states which subset `circuits/lib/src/lib.nr` implements. Changing any assigned value is a new deployment, not an upgrade.

### Economy

Every sentence is a maintenance liability: a claim written twice has to be fixed twice, and the second copy is the one that goes stale. Doc work here trends net-negative in lines.

- **One owning section per claim.** Every fact has exactly one home; everywhere else links to it. When tempted to summarise a neighbouring section for the reader's convenience, cite it instead.
- **Say it once, then move on.** No second-register restatement, no paragraph-closing punchline, no recap of the section's own argument in its last sentence. If a paragraph's content survives deleting it, delete it.
- **Gloss a symbol at its definition site only.** Re-glossing `s` or `r_e` in each section that uses them is three more places to update when a name changes.
- **No pre-stating.** A `Note:` or lead-in that previews what the next paragraph spells out in full is a duplicate. Fold it into the argument.
- **Motivation before constraint.** State what a rule protects against, then the rule. Rationale appended after the fact invites a second copy of the rule next to it.
- **Drop concessive asides.** "…though X would not be a violation either" earns nothing and dates fast.
- **Grep before adding.** A new claim is usually a second copy of an existing one. Search `docs/protocol/` for the symbol or term before writing a sentence about it.
- **A citation is not a summary.** The link is the whole reference. `[Spender Allowance Auditing](…), which escrows the allowance blinding` re-creates the copy the citation was avoiding — name the target only when the sentence would be unparsable without it.

### Style

- **One math style.** LaTeX is written as `$$…$$` with every subscript underscore escaped (`$$\mathbb{F}\_r$$`). GitHub runs the span through its markdown inline parser before handing it to MathJax: an unescaped `_` flanked by punctuation pairs with the next one in the same paragraph, table cell, or heading, and the `<em>` that results destroys both spans. The escape is stripped again on the way to MathJax, so `\_` arrives as a subscript. LaTeX's own literal underscore inside `\text{}` is `\\\_`, and `\{`, `\}`, `\|`, `\,`, `\;` double the same way. A span needs whitespace or punctuation on both sides — `D3$$\_i$$` renders as literal text — and no span inside link text renders at all. `indexer.md` and `compliance.md` use backticked ASCII and no LaTeX.
- **Symbols are a maintained namespace.** `sk`/`vk`/`dvk_i`/`PVK`/`Y`; `r_e` and `R_e = r_e·H`; `σ` (operation salt) is distinct from `σ_a` (per-delegation allowance salt); tilde means ciphertext; `C_spend` / `C_receive` / `C_transfer` / `C_a`; `r_a` is `C_a`'s blinding and `r_a'` the post-transfer one. An audit finding once required renaming the `tx` subscript to `transfer` across the whole module.
- Prose is full-width — no hard wrapping. One paragraph or list item per line.
