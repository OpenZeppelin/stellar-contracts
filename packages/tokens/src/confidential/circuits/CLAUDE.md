# Confidential Circuits — Agent Guide

Scoped to `packages/tokens/src/confidential/circuits/`. Read alongside the root `CLAUDE.md` and `../CLAUDE.md`.

This tree is Noir, built by `nargo` — **not** part of the Cargo workspace. Nothing in `Cargo.toml` references it and nothing here references `Cargo.toml`. `Architecture.md` §"Noir Circuits" explains the package model (`lib` vs measurement-only `bin` gadgets vs operation circuits); it is not repeated here.

## Commands

Run from this directory. CI is `.github/workflows/noir.yml`.

| Task | Command |
|:---|:---|
| Type-check | `nargo check` |
| Test | `nargo test` |
| Per-primitive constraint counts | `nargo info` |
| Compile one circuit | `nargo compile --package circuit_<name>` |
| Regenerate verification keys | `./scripts/extract_vks.sh` |

There is no `nargo fmt` step in CI and no formatter config — Noir formatting is unenforced. The workflow triggers only on `circuits/**`, so a change to the Rust-side parity test in `../test.rs` does **not** run this job.

## Traps

### `compiler_version = "=1.0.0"` is deliberate

It appears in all fourteen `Nargo.toml` files and looks like a mistake. Nargo requirements cannot name prereleases, so the real toolchain — **nargo 1.0.0-beta.11 + bb 0.87.0**, pinned in `.github/workflows/noir.yml` — is documented in the comment above it instead. Do not "correct" this to `=1.0.0-beta.11`.

### Do not prune unused public inputs

`_acct_f` in `register/src/main.nr` is referenced by no gate and looks like dead code. It is the replay binding: UltraHonk absorbs every public input into the transcript, so a proof produced for one account fails when the contract assembles the blob for another. Removing it lets anyone replay a legitimate registration's published proof and payload to mint duplicate-key accounts. Each operation circuit declares its exact public-input count in a header comment — withdraw 16, revoke_spender 19, transfer / spender_transfer 25, set_spender 26 — and the count is part of the contract with the on-chain assembler.

### Package names are load-bearing

Directory `transfer/` is package `circuit_transfer`; `gadgets/commit/` is `gadget_commit`; the library is `stellar_confidential_lib`. `scripts/extract_vks.sh` derives `circuit_${name}` and `target/${pkg}.json` from a bare name, and every row of `constraints.baseline` is keyed on the package name. Renaming a directory without matching the package name breaks both. Gadgets depend on the lib via `../../lib`, operations via `../lib`.

### Never hash raw

`poseidon_with_domain` is the only Poseidon entry point in `lib/src/lib.nr`; calling the underlying hash directly is a violation of the library contract. The domain tag is always the first absorbed element. The numeric tag values are the cross-language contract with the SDK — see `../CLAUDE.md` and `../docs/DESIGN_cont.md` §13, which is their only authoritative source.

Sponge parameters, the canonical lane assignment, and the mode-exclusivity rule that follows from a single-block absorb are normative in `../docs/DESIGN.md` §2.5; the Noir sponge must match it exactly. The obligations that section places on this code: `sponge_squeeze_2(d,s,σ)[0]` must stay equal to `poseidon_with_domain(d,[s,σ])`, and `sponge_squeeze_3(d,s,σ)[0..2]` must stay equal to `sponge_squeeze_2(d,s,σ)` — which is why `sponge_squeeze_2` is defined as the prefix of `sponge_squeeze_3` rather than as a second permutation. A divergence in either silently changes every existing mask.

`AUDITOR_SENDER` is squeezed three-wide by every circuit that escrows `lane[2]` and two-wide only by RevokeSpender (V_a3); `AUDITOR_RECIPIENT` is always two-wide; every other tag goes through `poseidon_with_domain`. Widening or narrowing a channel is a spec change, not a refactor.

`lane[2]` carries **the blinding of a commitment the operation writes, never a key** — `r'` on W_a5 / T_a9 / S_a6, `r_a'` on O_a9. Tag 17 (`ESCROWED_ALLOWANCE_BLINDING_AUDITOR`) is the same idea off-sponge: SetSpender's `lane[2]` is already taken, so S14 escrows `r_a` under a single-output pad. Do not escrow `dvk_i` here: it is permanent per `(owner, spender)` and survives revoke-then-re-delegate, so one leaked ciphertext would open every allowance state for that pair, past and future (`../docs/DESIGN_cont.md` §8.5).

ECDH must absorb both `S.x` and `S.y`; x-only extraction collapses `P` and `-P`.

The `G` and `H` generators are hardcoded but provenance-checked at runtime by the `print_generators` test; re-extract with `nargo test print_generators --show-output` rather than editing the constants by hand.

## Committed artifacts

Three kinds of generated file are committed and diffed by CI: `constraints.baseline`, `vks/*.vk.json`, and `lib/testdata/*.json`. (`Architecture.md` claims `testdata/*.json` is the only one — that statement is stale.) `target/` and `Prover.toml` are gitignored.

### `constraints.baseline`

Normalized `nargo info` output. Regenerate with the command in its own header:

```bash
LC_ALL=C nargo info | grep '^|' | LC_ALL=C sort > constraints.baseline
```

`LC_ALL=C` is mandatory on **both** sides of the pipe — byte order is the only ordering stable between macOS and the Ubuntu runner. The redirect overwrites the file's header comments; re-paste them, because CI's failure message asks for them.

Two non-obvious consequences: adding or removing a **gadget** changes the baseline even when no circuit logic changed, and the ACIR opcode counts are quoted in prose at `../docs/DESIGN_cont.md` §10.3 (Register 33, Withdraw 95, RevokeSpender 123, Transfer 134, SetSpender 135, SpenderTransfer 136). Nothing enforces that second copy — update it in the same PR.

### `vks/`

`./scripts/extract_vks.sh` compiles each circuit and runs `bb write_vk -s ultra_honk --output_format fields`. The `fields` format is required, not a preference: bb's `bytes` format carries platform-dependent header bytes that break macOS↔Linux diffs.

Adding a circuit means **three** edits — the package itself, the `members` list in `Nargo.toml`, and the hardcoded `CIRCUITS` array in `scripts/extract_vks.sh` — then regenerating both the baseline and the VKs.

Proving uses non-default flags: `bb prove -s ultra_honk --oracle_hash keccak`. Keccak is required because the on-chain verifier reproduces the Fiat-Shamir transcript with Keccak while bb defaults to `poseidon2`. Do **not** pass `--zk`; the verifier implements only the non-zk `ultra_flavor`. This recipe is provisional until the verifier is finished.

Drift policy: if the circuit changed, regenerate in the same PR. If the **toolchain** drifted, do not regenerate — investigate first.

### `lib/testdata/`

Fixtures are not auto-generated. Changing a primitive is a three-step lockstep:

1. `nargo test print_fixtures --package stellar_confidential_lib --show-output`
2. Update the matching `testdata/*.json`
3. Update the hardcoded expected values in the `fixtures_match_testdata` test in `lib/src/tests.nr`

`fixtures_match_testdata` is the in-Noir guard that fails CI. The sponge vectors are additionally hoisted into `global SPONGE_SQUEEZE_2_*` / `SPONGE_SQUEEZE_3_*` constants in the same file — a fourth site.

`address_to_field.json` is the exception. That derivation has no Noir implementation at all (circuits take `addr_f` as an opaque public input), so it is the one primitive with two independent implementations. Its guard is the Rust test `address_to_field_matches_testdata_vectors` in `../test.rs`, which **transcribes the hex values as string literals** rather than reading the JSON — update both together or neither. Its inputs are 56-character SEP-23 strkeys, and the lo/hi 28-byte limbs are little-endian.

Note the standing obligation on the future TS SDK (`../docs/SDK.md` §6.1): its tests must *read* these JSON files rather than transcribe them, so that a change to `print_fixtures` output becomes a test failure instead of a silent divergence. Do not copy the Rust test's transcription pattern into new consumers.

## Version bumps

`.github/workflows/noir.yml` is the source of truth for `NARGO_VERSION` and `BB_VERSION`, and the two must be bumped together — the VK pipeline is byte-sensitive to bb. The string `1.0.0-beta.11` is duplicated across all fourteen package `Nargo.toml` files, `constraints.baseline`, `vks/README.md` (which also restates the bb version), and this file. A bump touches all of them and requires regenerating both the baseline and the VKs. The installer scripts in CI are pinned to git commits with SHA256 verification; bump URL and hash together.
