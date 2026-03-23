# Supply Limit Module

Concrete deployable example of the `SupplyLimit` compliance module for Stellar
RWA tokens.

## What it enforces

This module caps the total amount of tokens that may be minted for a given
token contract.

It keeps an internal supply counter and checks that each mint would stay within
the configured per-token limit.

## How it stays in sync

The module maintains internal supply state and therefore must be wired to all
of the hooks it depends on:

- `CanCreate`
- `Created`
- `Destroyed`

After those hooks are registered, `verify_hook_wiring()` must be called once so
the module marks itself as armed before mint validation starts.

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
- `set_supply_limit(token, limit)` sets the per-token cap
- `pre_set_internal_supply(token, supply)` seeds tracked supply when wiring the
  module after historical minting
- `get_supply_limit(token)` reads the configured cap
- `get_internal_supply(token)` reads the tracked internal supply
- `required_hooks()` returns the required hook set
- `verify_hook_wiring()` marks the module as armed after registration
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- Storage is token-scoped, so one deployed module can be reused across many
  tokens
- A configured limit of `0` behaves as "no cap"
- If the module is attached after a token already has minted supply, seed the
  existing amount with `pre_set_internal_supply` before relying on `can_create`
- The internal supply is updated only through the registered `Created` and
  `Destroyed` hooks
