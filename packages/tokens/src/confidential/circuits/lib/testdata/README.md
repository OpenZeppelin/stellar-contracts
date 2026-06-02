# Cross-language test vectors

## What this is

Each JSON file in this directory pins one primitive's output for a fixed input. They are the durable, language-agnostic contract for everything that has to reproduce this library's cryptography off-chain:

- the TypeScript SDK that builds proof inputs from wallet state,
- on-chain Rust integration tests that mirror a primitive to assert end-to-end consistency,
- any future port (mobile, hardware wallet, indexer).

A consumer in any language is correct *iff*, given the inputs documented below, it reproduces every output in every JSON file byte-for-byte.

## How they're produced

Run the `print_fixtures` test inside the Noir library and read its output:

```bash
nargo test print_fixtures --package stellar_confidential_lib --show-output
```

That test (in `lib/src/tests.nr`) feeds a fixed input set through every primitive and prints `name = 0x...` for each. The JSON files in this directory mirror those outputs, one file per primitive — they are not generated automatically; if a primitive's behavior is changed, one must (a) re-run `print_fixtures`, (b) update the matching `*.json`, and (c) update the hard-coded expected values inside the `fixtures_match_testdata` test so it stays in lockstep.

`fixtures_match_testdata` is the in-Noir guard: it asserts that every value documented here is still what the lib produces. If one changes a primitive without updating both this directory *and* that test, CI fails. If a primitive is intentionally changed, the cross-language contract is broken — every downstream consumer must be updated and the change should bump a version (see `Cargo.toml` once the SDK lands).

## Inputs (shared across fixtures)

| Symbol | Value (hex) | Meaning |
|:--|:--|:--|
| `sk` | `0xdead` | spending key scalar |
| `addr_f` | `0xbeef` | contract address as a field element |
| `sigma` | `0x01` | salt for owner-side derivations |
| `sigma_a` | `0x02` | salt for allowance-side derivations |
| `op_i` | `0xabcd` | spender address (as `Field`) |
| `v` | `1000` (`0x3e8`) | balance value |
| `r` | `42` (`0x2a`) | balance randomness |
| `v_tx` | `100` (`0x64`) | transfer amount |
| `v_a` | `500` (`0x1f4`) | allowance amount |
| `r_e` | `0xfeedface` | ephemeral ECDH scalar |
| `s` | `0x12345` | ECDH shared-secret `x` (treated as opaque scalar input) |

All field values are BN254 scalar field elements written as zero-padded hex.
