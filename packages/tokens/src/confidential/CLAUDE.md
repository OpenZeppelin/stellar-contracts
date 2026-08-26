# Confidential Token — Agent Guide

Scoped to `packages/tokens/src/confidential/`. The root `CLAUDE.md` still applies in full; this file only covers what is specific to this subtree. The Noir workspace has its own guide at `circuits/CLAUDE.md`.

## Orientation

The module ships one token contract plus three satellites, each with the standard `mod.rs` / `storage.rs` / `test.rs` shape:

| Path | Role |
|:---|:---|
| `mod.rs`, `storage.rs` | The `ConfidentialToken` trait — eleven entry points — and the storage/orchestration layer |
| `verifier/` | Separate contract holding per-circuit UltraHonk verification keys |
| `auditor/` | Separate contract holding the auditor key registry |
| `compliance/` | `ComplianceHooks` — freeze, SAC passthrough, policy contract, clawback |
| `circuits/` | Noir workspace, compiled by `nargo`, not `cargo` |
| `docs/` | The protocol specification (see below) |

Balances are Pedersen commitments on Grumpkin; every operation that consumes private state carries a proof the contract forwards to the verifier contract.

**Not production ready.** The UltraHonk backend (`rs-soroban-ultrahonk`) is unfinished and unaudited. The `# ⚠️ Not Production Ready` blocks in `mod.rs` and `verifier/mod.rs` are load-bearing — do not remove or soften them.

## Errors

`ConfidentialTokenError` occupies **3500–3510**. The root guide's range list predates this module and omits it. Stay in the 3500s; do not open a new range for the satellites.

## Canonical encoding is a security boundary

The public-input blob is a positional concatenation of 32-byte big-endian `Bn254Fr` representatives, in the order given by each circuit's table in DESIGN §7. Grumpkin points contribute two limbs (`x` then `y`).

Soroban's host silently reduces values `≥ r` modulo `r` rather than rejecting them, so `x` and `x + r` deserialise to the same field element. Every caller-supplied scalar and coordinate must therefore reach `verify_proof` through `append_field` / `append_point`, which call `Grumpkin::is_canonical_field` / `is_canonical_point`. Bypassing those helpers breaks byte-uniqueness of stored state and emitted events even though proofs still verify.

## Code cites the spec by section number

Rust carries roughly sixteen `DESIGN §N` / `DESIGN_cont §N` references in doc comments. Renumbering a spec section silently invalidates them — nothing checks. Before renumbering, grep the module for the old number.

## Tests

Beyond the root guide's conventions:

- Proof verification is mocked. `MockVerifier` / `MockAuditor` in `test.rs` stand in for the real contracts, and proofs are empty `Bytes::new(e)`. Do not attempt to generate real proofs in Rust tests.
- Use `fixture_point` / `fixture_field`, not arbitrary bytes. The fixtures are canonical and on-curve; random values fail the canonicality guards before reaching any logic under test.
- One mock models proof semantics on purpose: the register mock binds the first `acct_f` it sees, standing in for UltraHonk's absorption of public inputs, so replay tests are meaningful. Keep that behaviour if the register flow changes.

## The documentation set

`docs/` is a specification, not commentary, and it is the single largest maintenance hazard in this module. Nine of the last sixteen commits on this branch were doc-consistency fixes.

### Normativity

`DESIGN.md` (§1–§7) and `DESIGN_cont.md` (§8–§13) are normative and share **one** numbering space — `DESIGN §9` resolves to `DESIGN_cont.md`. The split exists because GitHub stops rendering LaTeX after roughly 750 expressions per page; it is a rendering budget, not a topical seam.

Everything else defers by citation: `SDK.md`, `SELECTIVE_DISCLOSURE.md`, `INDEXER.md`, `COMPLIANCE.md`, and the non-normative `OVERVIEW.md`. Two exceptions run the other way — `circuits/lib/src/lib.nr` outranks the docs wherever they disagree about a primitive (`SDK.md` §4 says so explicitly), and the contract's `#[contracttype]`s are authoritative for their own shape.

`DESIGN.md` is already at roughly 778 expressions, over its own stated budget. Do not add math to §1–§7; put it in `DESIGN_cont.md`.

### Duplicated tables that drift

Five things exist in more than one file. Changing the normative copy means grepping for every other one:

| Content | Normative source | Copies live in |
|:---|:---|:---|
| The 17 domain-separation tags | `DESIGN_cont.md` §13 | `SDK.md` §4.8, referenced by `SELECTIVE_DISCLOSURE.md` |
| Sponge lane assignment (lane 0 = amount mask, lane 1 = balance/allowance/randomness, lane 2 = sender-auditor secret-escrow slot) | `DESIGN.md` §2.5 | `SDK.md` §4.3 and §11 |
| Per-circuit scalar-multiplication counts | `DESIGN_cont.md` §10.3 | `OVERVIEW.md` |
| Checkpoint event set (`Withdraw`, `Transfer` sender side, `SetSpender`, `RevokeSpender`) | `DESIGN.md` §5.2 | `INDEXER.md`, `SDK.md` |
| Replay-window anchor `T₀` | `DESIGN.md` §5.2 | `INDEXER.md`, `OVERVIEW.md` |

The tags are a cross-language wire contract. `DESIGN_cont.md` §13 assigns all seventeen and no other document may; `circuits/lib/src/lib.nr` implements 1–13 and 17, because 14 is derived off-circuit and 15–16 belong to the off-chain disclosure layer. That gap is intentional. Changing any assigned value is a new deployment, not an upgrade.

### Editing rules

- **Cite, do not restate.** Every drift bug in the recent history came from a second copy of something. When tempted to summarise a neighbouring section, write `§N` instead.
- **Match the file's math style, not a global one.** `DESIGN*.md` and `SELECTIVE_DISCLOSURE.md` use `$$…$$` with backslash-escaped subscripts (`$$\mathbb{F}\_r$$`); `SDK.md` uses `$$…$$` unescaped; `OVERVIEW.md` uses single `$…$`; `INDEXER.md` and `COMPLIANCE.md` use backticked ASCII and no LaTeX.
- **Symbols are a maintained namespace.** `sk`/`vk`/`dvk_i`/`PVK`/`Y`; `r_e` and `R_e = r_e·H`; `σ` (operation salt) is distinct from `σ_a` (per-delegation allowance salt); tilde means ciphertext; `C_spend` / `C_receive` / `C_transfer` / `C_a`. An audit finding once required renaming the `tx` subscript to `transfer` across the whole module.
- Prose is full-width — no hard wrapping. One paragraph or list item per line.
