# Time Transfers Limits Module

Concrete deployable example of the `TimeTransfersLimits` compliance module for
Stellar RWA tokens.

## What it enforces

This module limits the amount an investor identity may transfer within one or
more configured time windows.

Limits are tracked per identity, not per wallet, so the module must be
configured with an Identity Registry Storage (IRS) contract for each token it
serves.

Each limit is defined by:

- `limit_time`: the window size in seconds
- `limit_value`: the maximum transferable amount during that window

This example allows up to four active limits per token.

## How it stays in sync

The module maintains transfer counters and therefore must be wired to all of
the hooks it depends on:

- `CanTransfer`
- `Transferred`

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
- `set_identity_registry_storage(token, irs)` stores the IRS address for a
  token
- `set_time_transfer_limit(token, limit)` adds or replaces a limit window
- `batch_set_time_transfer_limit(token, limits)` updates multiple windows
- `remove_time_transfer_limit(token, limit_time)` removes a window
- `batch_remove_time_transfer_limit(token, limit_times)` removes many windows
- `pre_set_transfer_counter(token, identity, limit_time, counter)` seeds an
  in-flight rolling window when attaching the module after recent transfers
- `required_hooks()` returns the required hook set
- `verify_hook_wiring()` marks the module as armed after registration
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- Storage is token-scoped, so one deployed module can be reused across many
  tokens
- Counter resets are driven by ledger timestamps
- If the module is attached after transfers have already occurred inside an
  active window, seed the relevant identity counters before relying on
  `can_transfer`
- Only outgoing transfer volume is tracked; mint and burn hooks are not used
