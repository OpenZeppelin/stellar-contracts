# RWA Compliance Modules

Independently deployable compliance modules for RWA tokens, ported from
T-REX (ERC-3643).

## Layout

```
packages/compliance-modules/
  common/                          → Shared helpers (IRS client, compliance auth, safe math)
  modules/
    country-allow/                 → CountryAllowModule
    country-restrict/              → CountryRestrictModule
    initial-lockup-period/         → InitialLockupPeriodModule
    max-balance/                   → MaxBalanceModule
    supply-limit/                  → SupplyLimitModule
    time-transfers-limits/         → TimeTransfersLimitsModule
    transfer-restrict/             → TransferRestrictModule
  deploy/                          → Deployable infra crates (IRS, Verifier, Compliance, Token)
  scripts/                         → Build, deploy, wire, and test scripts
```

Each module crate produces a deployable WASM (`cdylib`) and can also be
used as a Rust library dependency (`rlib`) for integration tests.

See [`deploy/README.md`](deploy/README.md) for details on the infrastructure
crates and build artifacts.

## Scripts

The `scripts/` directory contains shell scripts for building, deploying, and
testing the full RWA compliance stack on Stellar testnet.

| Script               | Purpose                                                                                               |
| -------------------- | ----------------------------------------------------------------------------------------------------- |
| `e2e.sh`             | Master script — runs the entire flow end-to-end (build → deploy → wire → test)                        |
| `build.sh`           | Compiles all 11 WASMs (7 modules + 4 infra) via `stellar contract build`                              |
| `build-module.sh`    | Compiles a single module by name (e.g., `./build-module.sh country-allow`)                            |
| `deploy.sh`          | Deploys all contracts, configures every module, then locks admin via `set_compliance_address`         |
| `deploy-module.sh`   | Deploys and configures a single module (e.g., `./deploy-module.sh max-balance CanTransfer CanCreate`) |
| `wire.sh`            | Registers the 5 safe modules on their compliance hooks (12 registrations total)                       |
| `test-happy-path.sh` | Registers an investor identity, mints tokens, and verifies balance                                    |

### Deployment ordering

Module admin functions use `require_compliance_auth`, which is unrestricted
before `set_compliance_address` is called but requires Compliance contract
authorization after. Since the CLI cannot authorize as the Compliance contract,
the scripts follow a strict ordering:

1. **Deploy** all infrastructure and modules
2. **Configure** every module (IRS bindings, allowed countries, limits, allowlists)
3. **Lock** all modules by calling `set_compliance_address` (irreversible)
4. **Wire** modules to compliance hooks
5. **Register** investor identities and test

### Quick start

```bash
cd packages/compliance-modules/scripts

# Full end-to-end (build + deploy + wire + test):
./e2e.sh

# Or skip the build if WASMs are already compiled:
./e2e.sh --skip-build

# Or run each step individually:
./build.sh
./deploy.sh
./wire.sh
./test-happy-path.sh
```

Environment variables: `STELLAR_SOURCE` (default: `alice`),
`STELLAR_NETWORK` (default: `testnet`).

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
