# Deploy Crates

Minimal contract crates that wire the OpenZeppelin Stellar library traits into
deployable WASMs. Each crate exists solely to compose library functionality
into a concrete contract — they contain no business logic of their own.

## Crates

| Crate         | Purpose                                                                      | Key traits / modules composed                                                   |
| ------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `irs/`        | Identity Registry Storage — stores investor identities and country data      | `IdentityRegistryStorage`, `CountryDataManager`, `TokenBinder`, `AccessControl` |
| `verifier/`   | Identity Verifier — validates that an account has a registered identity      | `IdentityVerifier`                                                              |
| `compliance/` | Compliance contract — orchestrates hook dispatch across registered modules   | `Compliance`, `TokenBinder`, `AccessControl`                                    |
| `token/`      | RWA Token — compliant fungible token with freeze, forced transfer, and pause | `FungibleToken`, `RWAToken`, `Pausable`, `AccessControl`                        |

## Why separate crates?

Soroban requires each deployable contract to live in its own crate (one
`cdylib` per WASM). These crates are thin wrappers: the actual implementation
lives in `stellar-tokens`, `stellar-access`, and `stellar-contract-utils`.

## Build

All four crates are built together by `scripts/build.sh`, which calls
`stellar contract build` for each package and places the optimized WASMs
into `deploy/wasm/`.

## Artifacts (git-ignored)

- `wasm/` — compiled WASM binaries (produced by `scripts/build.sh`)
- `testnet-addresses.json` — contract addresses from the last deployment
  (produced by `scripts/deploy.sh`)
