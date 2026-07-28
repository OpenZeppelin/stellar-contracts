# Verification keys

UltraHonk verification keys for the per-operation circuits, one JSON file
per circuit (`<name>.vk.json`). These are committed artifacts -- the
integration contract with the verifier (#701).

**Format:** each file is a JSON array of hex-encoded `Fr` elements, produced
by `bb write_vk --output_format fields`. Used instead of bb's raw `bytes`
format because the latter includes platform-dependent header bytes that
spuriously break cross-platform reproducibility (macOS vs Linux CI).

Reproducible from the circuit sources with the pinned toolchain:

| Tool   | Version          |
|:-------|:-----------------|
| nargo  | `1.0.0-beta.11`  |
| bb     | `0.87.0`         |

Both versions are pinned in `.github/workflows/noir.yml`. CI re-runs the
extraction and diffs against the files here; any drift fails the build.

## Proving

Proofs the deployed verifier accepts must be generated with the same
pinned toolchain and non-default `bb` flags:

```bash
nargo execute --package circuit_<name> <witness_name>
bb prove -s ultra_honk --oracle_hash keccak \
   -b target/circuit_<name>.json -w target/<witness_name>.gz -o <out_dir>
```

- `--oracle_hash keccak` is required: the on-chain verifier
  ([rs-soroban-ultrahonk](https://github.com/NethermindEth/rs-soroban-ultrahonk))
  reproduces the Fiat-Shamir transcript with Keccak, while `bb` defaults to
  `poseidon2`. A proof generated with the default transcript is rejected.
- Do **not** pass `--zk`: the verifier currently implements only the non-zk
  `ultra_flavor`.

The verifier backend is unfinished (see the module-level warning in
`../../verifier/mod.rs`). This recipe is provisional and will be finalized
together with the verifier, including the zero-knowledge setting.

## Regenerating

```bash
cd packages/tokens/src/confidential/circuits
./scripts/extract_vks.sh
```

If the diff is intentional (the circuit changed), regenerate and commit in
the same PR. If unintentional (e.g. toolchain bumped without an explicit
decision), do **not** regenerate -- track down the source first.
