# Confidential Token — Agent Guide

Scoped to `packages/tokens/src/confidential/`. The root `CLAUDE.md` still applies in full; this file only covers what is specific to this subtree. The Noir workspace has its own guide at `circuits/CLAUDE.md`.

## Orientation

The module ships one token contract plus three satellites, each with the standard `mod.rs` / `storage.rs` / `test.rs` shape:

| Path | Role |
|:---|:---|
| `mod.rs`, `storage.rs` | The `ConfidentialToken` trait — eleven entry points, of which `revoke_spender` is proofless — and the storage/orchestration layer |
| `verifier/` | Separate contract holding per-circuit UltraHonk verification keys |
| `auditor/` | Separate contract holding the auditor key registry |
| `compliance/` | `ComplianceHooks` — freeze, SAC passthrough, policy contract — plus the opt-in `ConfidentialClawback` trait (`clawback`, `force_revoke_spender`) |
| `circuits/` | Noir workspace, compiled by `nargo`, not `cargo` |
| `docs/` | The specification set; `docs/README.md` is its map (see below) |

Balances are Pedersen commitments on Grumpkin. Every operation that opens or re-randomizes a commitment carries a proof the contract forwards to the verifier contract; `deposit`, `merge`, and `revoke_spender` are proofless homomorphic folds.

**Not production ready.** The UltraHonk backend (`rs-soroban-ultrahonk`) is unfinished and unaudited. The `# ⚠️ Not Production Ready` blocks in `mod.rs` and `verifier/mod.rs` are load-bearing — do not remove or soften them.

## Errors

`ConfidentialTokenError` occupies **3500–3510**. The root guide's range list predates this module and omits it. Stay in the 3500s; do not open a new range for the satellites.

## Canonical encoding is a security boundary

The public-input blob is a positional concatenation of 32-byte big-endian `Bn254Fr` representatives, in the order given by each operation's table under `docs/protocol/operations/` (`docs/compliance.md#circuit` for the clawback circuit). Grumpkin points contribute two limbs (`x` then `y`).

Soroban's host silently reduces values `≥ r` modulo `r` rather than rejecting them, so `x` and `x + r` deserialise to the same field element. Every caller-supplied scalar and coordinate must therefore reach `verify_proof` through `append_field` / `append_point`, which call `Grumpkin::is_canonical_field` / `is_canonical_point`. Bypassing those helpers breaks byte-uniqueness of stored state and emitted events even though proofs still verify.

## Code cites the spec by path and anchor

Rust and Noir comments cite the docs as `docs/<file>.md#<heading-slug>` paths, and the docs cite each other with relative Markdown links. `docs/check_links.py` resolves every such reference across the module and fails CI on a missing file or heading, a duplicate heading within a file, or a file over the LaTeX budget. Run `python3 docs/check_links.py` after touching a heading, a file name, or a citation.

## Tests

Beyond the root guide's conventions:

- Proof verification is mocked. `MockVerifier` / `MockAuditor` in `test.rs` stand in for the real contracts, and proofs are empty `Bytes::new(e)`. Do not attempt to generate real proofs in Rust tests.
- Use `fixture_point` / `fixture_field`, not arbitrary bytes. The fixtures are canonical and on-curve; random values fail the canonicality guards before reaching any logic under test.
- One mock models proof semantics on purpose: the register mock binds the first `acct_f` it sees, standing in for UltraHonk's absorption of public inputs, so replay tests are meaningful. Keep that behaviour if the register flow changes.

## The documentation set

`docs/` is a specification, not commentary, and it is the single largest maintenance hazard in this module — doc-consistency fixes outnumber code commits on this branch.

### Layout and normativity

`docs/README.md` maps the set, states which files are normative, and gives the precedence rules (`circuits/lib/src/lib.nr` outranks the docs on primitives; `#[contracttype]` definitions outrank them on shapes). One topic per file: new material goes into the file that owns its topic, and a new file is warranted only by a new topic. GitHub stops rendering LaTeX after roughly 750 expressions per page; the checker caps each file at 500, so the budget never again dictates where a section lives.

### Duplicated tables that drift

Six things exist in more than one file. Changing the normative copy means grepping for every other one:

| Content | Normative source | Copies live in |
|:---|:---|:---|
| Domain-separation tag assignments | `docs/protocol/domain-separators.md` | `docs/sdk/crypto-core.md#domain-separators`, referenced by `docs/selective-disclosure/README.md` |
| Sponge lane assignment | `docs/protocol/primitives.md#lane-assignment` | `docs/sdk/crypto-core.md#poseidon2-sponge`, `docs/sdk/auditor-client.md` |
| Per-circuit scalar-multiplication counts | `docs/protocol/proof-system.md#circuit-cost-analysis` | `docs/overview.md` |
| Checkpoint event set | `docs/protocol/wallet-state.md#recovery` | `docs/indexer.md`, `docs/sdk/README.md` |
| Replay-window anchor `T₀` (`Register`, `Merge`, `Clawback`) | `docs/protocol/wallet-state.md#recovery` | `docs/indexer.md`, `docs/overview.md`, `docs/sdk/wallet.md`, `docs/compliance.md#wallet-and-auditor-consequences` |
| ACIR opcode counts | `circuits/constraints.baseline` | `docs/protocol/proof-system.md#circuit-cost-analysis`, `circuits/CLAUDE.md` |

The tags are a cross-language wire contract. `docs/protocol/domain-separators.md` is their only authoritative source: it assigns every value, and it states which subset `circuits/lib/src/lib.nr` implements and why the remainder are absent. Changing any assigned value is a new deployment, not an upgrade.

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

- **Match the file's math style, not a global one.** `protocol/` and `selective-disclosure/` use `$$…$$` with backslash-escaped subscripts (`$$\mathbb{F}\_r$$`); `sdk/` uses `$$…$$` unescaped; `overview.md` uses single `$…$`; `indexer.md` and `compliance.md` use backticked ASCII and no LaTeX.
- **Symbols are a maintained namespace.** `sk`/`vk`/`dvk_i`/`PVK`/`Y`; `r_e` and `R_e = r_e·H`; `σ` (operation salt) is distinct from `σ_a` (per-delegation allowance salt); tilde means ciphertext; `C_spend` / `C_receive` / `C_transfer` / `C_a`; `r_a` is `C_a`'s blinding and `r_a'` the post-transfer one. An audit finding once required renaming the `tx` subscript to `transfer` across the whole module.
- Prose is full-width — no hard wrapping. One paragraph or list item per line.
