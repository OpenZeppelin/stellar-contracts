# RWA Compliance Modules

Independently deployable compliance modules for RWA tokens, ported from
T-REX (ERC-3643).

## Layout

```
packages/compliance-modules/
  common/                 → Shared helpers (IRS client, compliance auth, safe math)
  country-allow/          → CountryAllowModule
  country-restrict/       → CountryRestrictModule
  initial-lockup-period/  → InitialLockupPeriodModule
  max-balance/            → MaxBalanceModule
  supply-limit/           → SupplyLimitModule
  time-transfers-limits/  → TimeTransfersLimitsModule
  transfer-restrict/      → TransferRestrictModule
```

Each module crate produces a deployable WASM (`cdylib`) and can also be
used as a Rust library dependency (`rlib`) for integration tests.

## Dependencies

- `stellar-compliance-common` — shared helpers
- `stellar-tokens` — `ComplianceModule` trait and IRS types

## T-REX parity

| Module crate            | T-REX EVM reference         | Identity resolution |
| ----------------------- | --------------------------- | ------------------- |
| `country-allow`         | `CountryAllowModule`        | IRS → country data  |
| `country-restrict`      | `CountryRestrictModule`     | IRS → country data  |
| `max-balance`           | `MaxBalanceModule`          | IRS → identity      |
| `time-transfers-limits` | `TimeTransfersLimitsModule` | IRS → identity      |
| `transfer-restrict`     | `TransferRestrictModule`    | Address-based       |
| `supply-limit`          | `SupplyLimitModule`         | Token total supply  |
| `initial-lockup-period` | `InitialLockupPeriodModule` | Wallet-scoped       |

## Known re-entry limitation

`SupplyLimitModule` and `InitialLockupPeriodModule` call back to the token
contract (`total_supply()` / `balance()`) during compliance hooks. Soroban
forbids contract re-entry, so these modules cannot be wired to hooks that
fire during the token's own execution (e.g., `CanCreate` for supply limit,
`CanTransfer` for lockup). See the architecture documentation for proposed
solutions.
