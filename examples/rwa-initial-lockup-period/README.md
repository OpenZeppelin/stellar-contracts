# Initial Lockup Period Module

Concrete deployable example of the `InitialLockupPeriod` compliance module for
Stellar RWA tokens.

## What it enforces

This module applies a lockup period to tokens received through primary
emissions. When tokens are minted, the minted amount is locked until the
configured release timestamp.

The example follows the library semantics:

- minted tokens are subject to lockup
- peer-to-peer transfers do not create new lockups for the recipient
- transfers and burns can consume only unlocked balance

## How it stays in sync

The module maintains internal balances plus lock records and therefore must be
wired to all of the hooks it depends on:

- `CanTransfer`
- `Created`
- `Transferred`
- `Destroyed`

After those hooks are registered, `verify_hook_wiring()` must be called once so
the module marks itself as armed before transfer validation starts.

## Authorization model

This example uses the bootstrap-admin pattern introduced in this port:

- The constructor stores a one-time `admin`
- Before `set_compliance_address`, configuration calls require that admin's
  auth
- After `set_compliance_address`, privileged calls require auth from the bound
  Compliance contract
- `set_compliance_address` itself remains a one-time admin action

This allows the module to be configured from the CLI before handing control to
Compliance.

## Main entrypoints

- `__constructor(admin)` initializes the bootstrap admin
- `set_lockup_period(token, lockup_seconds)` configures the mint lockup window
- `pre_set_lockup_state(token, wallet, balance, locks)` seeds an existing
  holder's mirrored balance and active lock entries
- `required_hooks()` returns the required hook set
- `verify_hook_wiring()` marks the module as armed after registration
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- Storage is token-scoped, so one deployed module can be reused across many
  tokens
- The module stores detailed lock entries plus aggregate locked totals
- If the module is attached after live minting, seed existing balances and any
  still-active lock entries before relying on transfer or burn enforcement
- Transfer and burn flows consume unlocked balance first, then matured locks if
  needed
