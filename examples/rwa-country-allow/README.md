# Country Allow Module

Concrete deployable example of the `CountryAllow` compliance module for Stellar
RWA tokens.

## What it enforces

This module allows tokens to be minted or transferred only to recipients whose
registered identity has at least one country code that appears in the module's
per-token allowlist.

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
- `add_allowed_country(token, country)` adds an ISO 3166-1 numeric code to the
  allowlist
- `remove_allowed_country(token, country)` removes a country code
- `batch_allow_countries(token, countries)` updates multiple entries
- `batch_disallow_countries(token, countries)` removes multiple entries
- `is_country_allowed(token, country)` reads the current allowlist state
- `set_compliance_address(compliance)` performs the one-time handoff to the
  Compliance contract

## Notes

- Storage is token-scoped, so one deployed module can be reused across many
  tokens
- This module validates on the compliance read hooks used for transfers and
  mints; it does not require extra state-tracking hooks
- In the deploy example, the module is configured before binding and then wired
  to the `CanTransfer` and `CanCreate` hooks
