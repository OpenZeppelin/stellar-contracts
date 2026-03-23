# Max Balance Module

Concrete deployable example of the `MaxBalance` compliance module for Stellar
RWA tokens.

## What it enforces

This module tracks balances per investor identity, not per wallet, and enforces
a maximum balance cap for each token.

Because the accounting is identity-based, the module must be configured with an
Identity Registry Storage (IRS) contract for each token it serves.

## How it stays in sync

The module maintains internal per-identity balances and therefore must be wired
to all of the hooks it depends on:

- `CanTransfer`
- `CanCreate`
- `Transferred`
- `Created`
- `Destroyed`

After those hooks are registered, `verify_hook_wiring()` must be called once so
the module marks itself as armed before mint and transfer validation starts.

## Authorization model

This example uses the bootstrap-admin pattern introduced in this port:

- The constructor stores a one-time `admin`
- Before `set_compliance_address`, configuration calls require that admin's
  auth
- After `set_compliance_address`, privileged calls require auth from the bound
  Compliance contract
- `set_compliance_address` itself remains a one-time admin action

This allows the module to be seeded and configured from the CLI before handing
control to Compliance.

## Main entrypoints

- `__constructor(admin)` initializes the bootstrap admin
- `set_identity_registry_storage(token, irs)` stores the IRS address for a
  token
- `set_max_balance(token, max)` configures the per-identity cap
- `pre_set_module_state(token, identity, balance)` seeds an identity balance
- `batch_pre_set_module_state(token, identities, balances)` seeds many
  identity balances
- `required_hooks()` returns the required hook set
- `verify_hook_wiring()` marks the module as armed after registration
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- Storage is token-scoped, so one deployed module can be reused across many
  tokens
- Transfers between two wallets that resolve to the same identity do not change
  the tracked balance distribution
- A configured max of `0` behaves as "no cap"
