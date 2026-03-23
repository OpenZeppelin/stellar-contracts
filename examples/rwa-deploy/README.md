# RWA Deploy Crates

Minimal deployable crates used by the end-to-end RWA deployment flow.

These crates primarily wire OpenZeppelin Stellar library traits into concrete
WASMs. They stay intentionally thin, but they do contain the deployment-specific
wiring needed for constructors, access control, token binding, and hook-based
compliance orchestration.

## Crates

| Crate         | Purpose                                                                      | Key traits / modules composed                                                   |
| ------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `irs/`        | Identity Registry Storage — stores investor identities and country data      | `IdentityRegistryStorage`, `CountryDataManager`, `TokenBinder`, `AccessControl` |
| `verifier/`   | Identity Verifier — validates that an account has a registered identity      | `IdentityVerifier`, `AccessControl`                                             |
| `compliance/` | Compliance contract — orchestrates hook dispatch across registered modules   | `Compliance`, `TokenBinder`, `AccessControl`                                    |
| `token/`      | RWA Token — compliant fungible token with freeze, forced transfer, and pause | `FungibleToken`, `RWAToken`, `Pausable`, `AccessControl`                        |

## Why separate crates?

Soroban requires each deployable contract to live in its own crate (one
`cdylib` per WASM). These crates are thin wrappers: the actual implementation
lives in `stellar-tokens`, `stellar-access`, and `stellar-contract-utils`.

## Build

`scripts/build.sh` builds the full deployable stack:

- 4 infrastructure WASMs from this directory: `irs/`, `verifier/`,
  `compliance/`, and `token/`
- 7 compliance-module WASMs from the example module crates:
  `rwa-country-allow`, `rwa-country-restrict`, `rwa-initial-lockup-period`,
  `rwa-max-balance`, `rwa-supply-limit`, `rwa-time-transfers-limits`, and
  `rwa-transfer-restrict`

The script calls `stellar contract build` for each package and writes the
optimized artifacts to `examples/rwa-deploy/wasm/`.

## Deploy flow

`scripts/deploy.sh` follows the current bootstrap-admin flow introduced by the
RWA module examples:

1. Deploy IRS, verifier, compliance, and token
2. Bind the token to Compliance and IRS
3. Deploy all 7 compliance modules with a bootstrap admin
4. Configure every module while bootstrap admin auth is still active
5. Call `set_compliance_address` on each module to hand control to Compliance

After deployment, `scripts/wire.sh` registers the modules on their required
hooks, and `scripts/e2e.sh` runs the full testnet flow.

## Artifacts (git-ignored)

- `wasm/` — compiled WASM binaries produced by `scripts/build.sh`
- `testnet-addresses.json` — contract addresses from the last deployment
  (produced by `scripts/deploy.sh`)
