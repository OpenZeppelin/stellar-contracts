# RWA Compliance Modules Architecture Notes

This directory intentionally keeps each compliance rule in an isolated module
file with dedicated tests, while centralizing shared mechanics in
`common.rs`.

## Why this structure

- **Isolation by rule**: each module has one primary reason to change
  (its compliance policy).
- **Shared safety primitives**: compliance auth checks, arithmetic guards, and
  common helper clients live in one place.
- **Token-scoped state**: module storage keys always include token address to
  support one compliance contract serving multiple tokens.

This makes incremental additions (for example, `ConditionalTransfer` or
`TransferFees`) low-risk and avoids monolithic compliance code paths.

## T-REX parity

The modules follow the T-REX (ERC-3643) reference implementations 1:1 in
both business-rule intent and identity resolution:

| Stellar module          | T-REX EVM reference         | Identity resolution |
| ----------------------- | --------------------------- | ------------------- |
| `country_allow`         | `CountryAllowModule`        | IRS → country data  |
| `country_restrict`      | `CountryRestrictModule`     | IRS → country data  |
| `max_balance`           | `MaxBalanceModule`          | IRS → identity      |
| `time_transfers_limits` | `TimeTransfersLimitsModule` | IRS → identity      |
| `transfer_restrict`     | `TransferRestrictModule`    | Address-based       |
| `supply_limit`          | `SupplyLimitModule`         | Token total supply  |
| `initial_lockup_period` | `InitialLockupPeriodModule` | Wallet-scoped       |

### How identity resolution works

T-REX modules resolve identity/country through a chain:
`compliance → token → identityRegistry → investorCountry / identity`.

In Stellar, modules store the Identity Registry Storage (IRS) contract
address per token as configuration (`set_identity_registry_storage`), then
call the IRS cross-contract at check time. This is a Soroban gas
optimization that avoids re-resolving the full chain on every hook, while
preserving the same behavioral semantics.

### Multi-country profiles

T-REX uses a single country code per investor. Stellar's IRS supports
multiple country entries (residence, citizenship, tax residency, etc.).

The country modules handle this by:

- **CountryAllow**: passes if **any** country code is in the allowlist
- **CountryRestrict**: blocks if **any** country code is in the restriction list

This is the strictest conservative interpretation for restrictions and the
most permissive for allowlists.

### IRS address lifecycle

Each module caches the IRS contract address per token via
`set_identity_registry_storage`. If the IRS contract is ever rotated
(e.g., during a registry migration), every module that depends on it must
be reconfigured by calling `set_identity_registry_storage` again with the
new address. A stale IRS address will cause identity/country lookups to
hit the old contract.

Deployments that anticipate registry migrations should automate this by
having the compliance contract's admin function propagate the new IRS
address to all bound modules in a single transaction.

## Known differences from T-REX EVM

These are deliberate adaptations for the Stellar/Soroban architecture, not
oversights. Each is documented in the relevant module's header comment.

### Structural (applies to all modules)

| Aspect                                | T-REX EVM                                                 | Stellar                      | Rationale                                                                                |
| ------------------------------------- | --------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------- |
| `canComplianceBind` / `isPlugAndPlay` | EVM-specific module lifecycle queries                     | Not applicable               | Stellar's module binding is handled by the compliance contract, not by the module itself |
| Idempotent add/remove                 | Reverts on duplicate add or removing a non-existent entry | Silently sets `true`/`false` | Idempotent writes are simpler and safer for multi-sig/batch workflows                    |

### Per-module

| Module                  | Difference                     | Detail                                                                                                                                                        |
| ----------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `supply_limit`          | Zero-limit semantics           | T-REX blocks all mints when limit is 0 (`totalSupply + value > 0`). Stellar treats 0 as "no cap" — aligns with plug-and-play intent.                          |
| `max_balance`           | No preset status tracking      | T-REX tracks `_compliancePresetStatus` and `presetCompleted()`. Stellar does not enforce preset ordering.                                                     |
| `time_transfers_limits` | No token-agent bypass          | T-REX `moduleCheck` returns true for token agents (`_isTokenAgent`). In Stellar, agent permissions are handled by the token's RBAC layer before hooks fire.   |
| `country_restrict`      | No batch size limit            | T-REX caps batches at 195 countries. Stellar has no limit (Soroban transaction budget is the natural bound).                                                  |
| `initial_lockup_period` | Lockup period in seconds       | T-REX configures in days (`_lockupPeriodInDays * 1 days`). Stellar uses seconds directly (Soroban timestamps).                                                |
| `initial_lockup_period` | `total_locked` kept consistent | T-REX `_updateLockedTokens` removes entries without decrementing `totalLocked`. Stellar's `update_locked_tokens` also decrements `total_locked` for accuracy. |
