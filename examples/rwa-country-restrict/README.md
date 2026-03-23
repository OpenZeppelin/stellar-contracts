# Country Restrict Module

Concrete deployable example of the `CountryRestrict` compliance module for
Stellar RWA tokens.

## What it enforces

This module blocks tokens from being minted or transferred to recipients whose
registered identity has a country code that appears in the module's per-token
restriction list.

The country lookup is performed through the Identity Registry Storage (IRS), so
the module must be configured with an IRS contract for each token it serves.

## Authorization model

This example uses the bootstrap-admin pattern introduced in this port:

- The constructor stores a one-time `admin`
- Before `set_compliance_address`, privileged configuration calls require that
  admin's auth
- After `set_compliance_address`, the same configuration calls require auth
  from the bound Compliance contract
- `set_compliance_address` itself remains a one-time admin action

This lets the module be configured from the CLI before it is locked to the
Compliance contract.

## Main entrypoints

- `__constructor(admin)` initializes the bootstrap admin
- `set_identity_registry_storage(token, irs)` stores the IRS address for a
  token
- `add_country_restriction(token, country)` adds an ISO 3166-1 numeric code to
  the restriction list
- `remove_country_restriction(token, country)` removes a country code
- `batch_restrict_countries(token, countries)` updates multiple entries
- `batch_unrestrict_countries(token, countries)` removes multiple entries
- `is_country_restricted(token, country)` reads the current restriction state
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- Storage is token-scoped, so one deployed module can be reused across many
  tokens
- This module validates on the compliance read hooks used for transfers and
  mints; it does not require extra state-tracking hooks
- In the deploy example, the module is configured before binding and then wired
  to the `CanTransfer` and `CanCreate` hooks
