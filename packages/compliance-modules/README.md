# RWA Compliance Modules

Independently deployable compliance modules for RWA tokens, ported from
T-REX (ERC-3643).

## Layout

```text
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
| `wire.sh`            | Registers all 7 modules on their compliance hooks (19 registrations, 4 verified)                      |
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

## Hook wiring requirements

### Background — Soroban re-entry constraint

On EVM, modules can call back into the token to read `totalSupply()` or
`balanceOf()` during a compliance hook. Soroban **forbids contract re-entry**,
so modules that need supply or balance data maintain their own internal mirrors
updated via `on_*` hooks. This makes correct hook wiring safety-critical:
**missing a hook causes the internal state to drift from the token's actual
state, leading to incorrect compliance decisions.**

### Required hooks per module

| Module                      | Required hooks                                                    | Tracks state? |
| --------------------------- | ----------------------------------------------------------------- | ------------- |
| `CountryAllowModule`        | `CanTransfer`, `CanCreate`                                        | No            |
| `CountryRestrictModule`     | `CanTransfer`, `CanCreate`                                        | No            |
| `TransferRestrictModule`    | `CanTransfer`                                                     | No            |
| `TimeTransfersLimitsModule` | `CanTransfer`, `Transferred`                                      | Yes (counter) |
| `MaxBalanceModule`          | `CanTransfer`, `CanCreate`, `Transferred`, `Created`, `Destroyed` | Yes (balance) |
| `SupplyLimitModule`         | `CanCreate`, `Created`, `Destroyed`                               | Yes (supply)  |
| `InitialLockupPeriodModule` | `CanTransfer`, `Created`, `Transferred`, `Destroyed`              | Yes (balance) |

### Runtime enforcement (stateful modules only)

The four modules marked "Yes" above (`SupplyLimitModule`,
`InitialLockupPeriodModule`, `MaxBalanceModule`, `TimeTransfersLimitsModule`)
enforce correct wiring via an **arming** mechanism that prevents the module from
operating until all required hooks have been verified:

| Step                      | What happens                                                                                                                         | When                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------- |
| 1. `required_hooks()`     | Returns the list of hooks the module needs (read-only query)                                                                         | Anytime                |
| 2. `verify_hook_wiring()` | Cross-calls the compliance contract to confirm every required hook is registered. **Arms** the module on success (result is cached). | Once, after wiring     |
| 3. Runtime guard          | `can_*` hooks panic if the module is not armed                                                                                       | Every compliance check |

#### Error messages

**If `verify_hook_wiring()` is not called** (module not armed):

```text
SupplyLimitModule: not armed — call verify_hook_wiring() after wiring hooks [CanCreate, Created, Destroyed]
TimeTransfersLimitsModule: not armed — call verify_hook_wiring() after wiring hooks [CanTransfer, Transferred]
```

**If a required hook is missing** during verification:

```text
missing required hook: Created
```

#### Usage in deployment scripts

```bash
# After wire.sh registers all hooks:
stellar contract invoke --id $SUPPLY_LIMIT     ... -- verify_hook_wiring
stellar contract invoke --id $INITIAL_LOCKUP   ... -- verify_hook_wiring
stellar contract invoke --id $MAX_BALANCE      ... -- verify_hook_wiring
stellar contract invoke --id $TIME_TRANSFERS   ... -- verify_hook_wiring
```

The `wire.sh` script already includes this verification step.

#### Design notes

This enforcement is lightweight by design — it does not introduce new traits or
change the `ComplianceModule` interface, keeping minimal drift from the original
T-REX module contracts. The `required_hooks()` and `verify_hook_wiring()`
methods are module-level public functions, not part of the trait.
