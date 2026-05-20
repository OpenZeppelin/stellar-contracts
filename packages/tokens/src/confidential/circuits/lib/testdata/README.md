# Cross-language test vectors

Each JSON file in this directory pins one primitive's output for a fixed input. These fixtures are the durable contract between the Noir library (this crate) and any language binding that needs to reproduce the wrapper's cryptography.

A consumer in any language is correct *iff* it reproduces every output below byte-for-byte from the same inputs.

## Regenerating

The fixtures are produced from the Noir lib itself:

```bash
nargo test print_fixtures --package stellar_confidential_lib --show-output
```

Capture the printed values into the corresponding `*.json` file (one per primitive). The `fixtures_match_testdata` test in `lib.nr` asserts the lib still produces these exact values; if it fails, either the fixtures were regenerated without updating the test (out-of-sync) or a primitive's semantics changed (breaking the cross-language contract — bump a version).

## Inputs (shared across fixtures)

| Symbol | Value (hex) | Meaning |
|:--|:--|:--|
| `sk` | `0xdead` | spending key scalar |
| `wrap` | `0xbeef` | wrapper contract address (as `Field`) |
| `sigma` | `0x01` | salt for owner-side derivations |
| `sigma_a` | `0x02` | salt for allowance-side derivations |
| `op_i` | `0xabcd` | operator address (as `Field`) |
| `v` | `1000` (`0x3e8`) | balance value |
| `r` | `42` (`0x2a`) | balance randomness |
| `v_tx` | `100` (`0x64`) | transfer amount |
| `v_a` | `500` (`0x1f4`) | allowance amount |
| `r_e` | `0xfeedface` | ephemeral ECDH scalar |
| `s` | `0x12345` | ECDH shared-secret `x` (treated as opaque scalar input) |

All field values are BN254 scalar field elements written as zero-padded hex.
