# RWA batch cost benchmark

Measurement harness behind the design decisions for the RWA batch functions
(issue #767). It is not a test suite: it prints cost curves rather than
asserting behaviour, and it takes roughly two minutes to run. That is why it
sits outside the root workspace (listed in `workspace.exclude`), so
`cargo test --workspace` and `cargo llvm-cov --workspace` leave it alone.

```bash
cargo test --manifest-path bench/rwa-batch/Cargo.toml -- --nocapture
```

## What it registers

The real library stack, not mocks:

```text
BenchToken ──> BenchCompliance ──> BenchSupplyLimit          (storage only)
     │                        └──> BenchMaxBalance ──> BenchIrs
     └───────> BenchVerifier ──┬──> BenchIrs                 (wallet -> identity)
                              ├──> BenchCti                  (required topics)
                              └──> BenchIdentity ──> BenchIssuer (ed25519 verify)
```

Access control is omitted from the bench contracts: an RBAC check is a
per-call constant that a batch amortises by construction, so it cannot move
the naive-versus-hoisted comparison.

## Results (one required claim topic unless noted)

`bench/rwa-batch/Cargo.lock` pins `soroban-sdk` to the same 27.0.2 the
workspace lock pins, so these figures are measured against the build the
library ships. The crate is workspace-`exclude`d and CI never rebuilds it, so
that pin has to be maintained by hand: re-run `cargo update -p soroban-sdk
--precise <version>` here whenever the workspace SDK moves, and re-measure
before trusting the numbers below.

Hoisting loop-invariant work out of the per-item body:

| batch | hoisted work | saving |
| --- | --- | --- |
| `mint` | `paused` + two instance address reads | 0.8% |
| `transfer` | sender identity verification | 28% (32% with two claim topics) |

That is why `batch_mint` ships as a plain loop and `batch_transfer` hoists.

The `transfer SHIPPED` row measures `RWA::batch_transfer` itself, which hoists
only the sender's identity verification: the cheap `paused` and
`is_frozen(from)` checks stay in the loop, because leaving them there costs
under 1% and keeps the set of assumptions the hoist rests on as small as
possible.

| n | naive (1 topic) | shipped (1 topic) | saving | saving, 2 topics |
| --- | --- | --- | --- | --- |
| 1 | 1,960,401 | 1,960,504 | 0% | 0% |
| 2 | 3,883,461 | 3,208,635 | 17% | 20% |
| 5 | 10,077,882 | 7,336,557 | 27% | 31% |
| 10 | 21,600,751 | 15,360,997 | 29% | 33% |
| 20 | 49,029,314 | 35,561,762 | 27% | 31% |

A batch of one costs the same as a single transfer, since the one sender
verification happens either way.

Isolated probes explaining the mint result: an instance read costs ~4,900 CPU
and a repeated write to an already-written key ~12,900 CPU, against a per-item
mint cost of ~1.2M. A repeated write to one key never costs more than one
write entry in the footprint.

## Batch ceilings

Against the **live** mainnet config settings, fetched with
`stellar network settings --rpc-url https://mainnet.sorobanrpc.com`
(`tx_max_footprint_entries` 400, `tx_max_write_ledger_entries` 200,
`tx_max_instructions` 400M, `tx_max_contract_events_size_bytes` 16,384). The
SDK's `InvocationResourceLimits::mainnet()` is a stale snapshot (footprint
100, writes 50, instructions 600M) and is deliberately not used.

| function | largest batch | binding limit |
| --- | --- | --- |
| `batch_mint` | 38 | footprint |
| `batch_transfer` | 37 | footprint |
| `batch_add_identity` | 37 | events size |
| `batch_freeze_partial_tokens` | 99 | events size |

Footprint per minted recipient is 8 distinct keys, attributed by removing one
contributor at a time: 4 for the token and registry (`Balance` read-write,
`AddressFrozen`, `FrozenTokens`, the IRS `Identity` mapping), 3 for the
investor's own identity contract (instance, claim-ids-by-topic, the claim),
and 1 for the max-balance module's per-identity aggregate. Each additional
required claim topic adds about 2.

Caveats, all pushing the real ceiling down rather than up:

- The footprint figures use the host's in-test accounting, which sums read and
  write entries and so counts every read-write key twice. Under the literal
  reading (each footprint key once) mint and transfer both reach 47.
- Native test contracts are not charged for wasm code entries; mainnet adds one
  per distinct wasm, about 9 for this stack.
- Native test contracts do not meter guest wasm instructions. At the ceiling,
  mint uses 78.6M of the 400M instruction budget and transfer 108M, so a real
  wasm build could make instructions the binding limit instead of footprint.
