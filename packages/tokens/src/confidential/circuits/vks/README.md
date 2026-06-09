# Verification keys

UltraHonk verification keys for the per-operation circuits, two files per
circuit. These are committed artifacts -- the integration contract with the
verifier (#701).

- **`<name>.vk.json`** -- a JSON array of hex-encoded `Fr` elements, produced
  by `bb write_vk --output_format fields`. This is the human-readable,
  review-friendly form and the one CI diffs. It is *not* the byte layout the
  verifier parses. Used instead of bb's raw `bytes` format because the latter
  includes platform-dependent header bytes that spuriously break
  cross-platform reproducibility (macOS vs Linux CI).
- **`<name>.vk.bin`** -- the packed binary key the on-chain verifier actually
  consumes (`ultrahonk-soroban-verifier::load_vk_from_bytes`): a 1760-byte
  blob made of a 32-byte header (four big-endian `u64`s -- `circuit_size`,
  `log_circuit_size`, `public_inputs_size`, `pub_inputs_offset`) followed by
  27 G1 commitments at 64 bytes each (`x || y`, big-endian). It is bb's
  default `bytes` output with the redundant 4-byte trailing header field (the
  user-public-input count, which the verifier recomputes) stripped. The key
  material is bb's verbatim output -- nothing is recomputed off-chain. The
  point section is byte-identical to the field elements in the matching
  `.vk.json`.

Reproducible from the circuit sources with the pinned toolchain:

| Tool   | Version          |
|:-------|:-----------------|
| nargo  | `1.0.0-beta.11`  |
| bb     | `0.87.0`         |

Both versions are pinned in `.github/workflows/noir.yml`. CI re-runs the
extraction and diffs against the files here; any drift fails the build.

## Regenerating

```bash
cd packages/tokens/src/confidential/circuits
./scripts/extract_vks.sh     # regenerates the *.vk.json (CI-diffed)
./scripts/build_vk_bins.sh   # regenerates the *.vk.bin (the on-chain form)
```

Both scripts compile the circuits with the same pinned toolchain, so the
two formats stay in lockstep -- always run both when a circuit changes and
commit the result in the same PR. CI diffs the `.vk.json`; since each
`.vk.bin` point section is byte-identical to its `.vk.json`, that diff
transitively guards the key material in the binary too.

If the diff is intentional (the circuit changed), regenerate and commit in
the same PR. If unintentional (e.g. toolchain bumped without an explicit
decision), do **not** regenerate -- track down the source first.
