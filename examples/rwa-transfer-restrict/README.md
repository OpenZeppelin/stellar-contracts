# Transfer Restrict Module

Concrete deployable example of the `TransferRestrict` compliance module for
Stellar RWA tokens.

## What it enforces

This module maintains a per-token address allowlist for transfers.

It follows the T-REX semantics implemented by the library trait:

- if the sender is allowlisted, the transfer passes
- otherwise, the recipient must be allowlisted

The module is token-scoped, so one deployment can serve many tokens.

## Authorization model

This example uses the bootstrap-admin pattern introduced in this port:

- The constructor stores a one-time `admin`
- Before `set_compliance_address`, allowlist management requires that admin's
  auth
- After `set_compliance_address`, the same configuration calls require auth
  from the bound Compliance contract
- `set_compliance_address` itself remains a one-time admin action

This lets the module be configured from the CLI before it is locked to the
Compliance contract.

## Main entrypoints

- `__constructor(admin)` initializes the bootstrap admin
- `allow_user(token, user)` adds an address to the transfer allowlist
- `disallow_user(token, user)` removes an address from the transfer allowlist
- `batch_allow_users(token, users)` updates multiple entries
- `batch_disallow_users(token, users)` removes multiple entries
- `is_user_allowed(token, user)` reads the current allowlist state
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- This module validates transfers through the `CanTransfer` hook
- It does not depend on IRS or other identity infrastructure
- In the deploy example, the admin address is pre-allowlisted before binding so
  the happy-path transfer checks can succeed
