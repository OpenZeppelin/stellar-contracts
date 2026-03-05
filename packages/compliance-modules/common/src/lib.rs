#![no_std]

//! Shared helpers for compliance modules.
//!
//! Centralizes compliance-address ownership/auth checks, safe arithmetic
//! guards, lightweight read-only client traits, and identity registry
//! storage (IRS) resolution helpers.

use soroban_sdk::{
    contractclient, contracterror, contracttype, panic_with_error, symbol_short, Address, Env,
    String, Symbol, Vec,
};

use stellar_tokens::rwa::{
    compliance::ComplianceHook,
    identity_registry_storage::{
        CountryData, CountryRelation, IndividualCountryRelation, OrganizationCountryRelation,
    },
};

const COMPLIANCE_KEY: Symbol = symbol_short!("cmpaddr");
const HOOKS_VERIFIED_KEY: Symbol = symbol_short!("hkverfd");

/// Read-only cross-contract client into the Identity Registry Storage.
///
/// Modules that need identity or country resolution store the IRS address
/// per token and call through this client at check time — mirroring the
/// T-REX pattern where modules resolve identity via the token's registry.
#[contractclient(name = "IRSReadClient")]
pub trait IRSRead {
    fn stored_identity(e: &Env, account: Address) -> Address;
    fn get_country_data_entries(e: &Env, account: Address) -> Vec<CountryData>;
}

/// Storage key for identity registry storage address, scoped per token.
#[contracttype]
#[derive(Clone)]
pub enum IRSKey {
    Registry(Address),
}

/// Contract error codes shared by all compliance modules.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ModuleError {
    ComplianceNotSet = 1,
    InvalidAmount = 2,
    MathOverflow = 3,
    MathUnderflow = 4,
    MissingLimit = 5,
    MissingCounter = 6,
    MissingCountry = 7,
    IdentityRegistryNotSet = 8,
}

/// Persists the compliance contract address that governs this module.
pub fn set_compliance_address(e: &Env, compliance: &Address) {
    e.storage().persistent().set(&COMPLIANCE_KEY, compliance);
}

/// Returns the stored compliance address, falling back to the module's
/// own address when no compliance contract has been configured yet.
pub fn get_compliance_address(e: &Env) -> Address {
    if !e.storage().persistent().has(&COMPLIANCE_KEY) {
        return e.current_contract_address();
    }
    e.storage()
        .persistent()
        .get::<_, Address>(&COMPLIANCE_KEY)
        .expect("compliance must be set")
}

/// Requires authorization from the compliance contract. Returns the
/// compliance address. Falls back to self-authorization when no
/// compliance contract is configured (useful for standalone testing).
pub fn require_compliance_auth(e: &Env) -> Address {
    if !e.storage().persistent().has(&COMPLIANCE_KEY) {
        return e.current_contract_address();
    }
    let compliance = get_compliance_address(e);
    compliance.require_auth();
    compliance
}

// ---------------------------------------------------------------------------
// Hook wiring verification
// ---------------------------------------------------------------------------

/// Minimal read-only client for querying the compliance contract's
/// hook registrations. Only exposes the `is_module_registered` view.
#[contractclient(name = "ComplianceHookCheckClient")]
pub trait ComplianceHookCheck {
    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool;
}

/// Returns `true` if the hook wiring has already been verified for this
/// module instance (cached after the first successful check).
pub fn hooks_verified(e: &Env) -> bool {
    e.storage().persistent().has(&HOOKS_VERIFIED_KEY)
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on every hook in `required`. Caches the result on success
/// so subsequent calls are a single storage read.
///
/// Skips verification if `set_compliance_address` has not been called
/// yet (the module is in unconfigured mode).
///
/// # Panics
///
/// Panics if any required hook is not registered — this means the
/// deployment is misconfigured and internal state would drift.
pub fn verify_required_hooks(e: &Env, required: Vec<ComplianceHook>) {
    if !e.storage().persistent().has(&COMPLIANCE_KEY) {
        return;
    }

    let compliance: Address = e
        .storage()
        .persistent()
        .get(&COMPLIANCE_KEY)
        .expect("compliance must be set");
    let self_addr = e.current_contract_address();
    let client = ComplianceHookCheckClient::new(e, &compliance);

    for i in 0..required.len() {
        let hook = required.get(i).unwrap();
        if !client.is_module_registered(&hook, &self_addr) {
            let name = match hook {
                ComplianceHook::CanTransfer => "CanTransfer",
                ComplianceHook::CanCreate => "CanCreate",
                ComplianceHook::Transferred => "Transferred",
                ComplianceHook::Created => "Created",
                ComplianceHook::Destroyed => "Destroyed",
            };
            panic!("missing required hook: {}", name);
        }
    }

    e.storage().persistent().set(&HOOKS_VERIFIED_KEY, &true);
}

// ---------------------------------------------------------------------------
// Amount validation
// ---------------------------------------------------------------------------

/// Panics with [`ModuleError::InvalidAmount`] if `amount` is negative.
pub fn require_non_negative_amount(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, ModuleError::InvalidAmount);
    }
}

/// Checked `i128` addition. Panics with [`ModuleError::MathOverflow`] on overflow.
pub fn checked_add_i128(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_add(right)
        .unwrap_or_else(|| panic_with_error!(e, ModuleError::MathOverflow))
}

/// Checked `i128` subtraction. Panics with [`ModuleError::MathUnderflow`] on underflow.
pub fn checked_sub_i128(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_sub(right)
        .unwrap_or_else(|| panic_with_error!(e, ModuleError::MathUnderflow))
}

/// Allocates a Soroban `String` from a static `&str` for use as a module name.
pub fn module_name(e: &Env, name: &str) -> String {
    String::from_str(e, name)
}

// ---------------------------------------------------------------------------
// Identity Registry Storage helpers
// ---------------------------------------------------------------------------

/// Stores the IRS contract address for a given token.
///
/// Called during module setup so country/identity lookups avoid
/// re-resolving the full token -> identity-verifier -> IRS chain on
/// every call (a Soroban gas optimization over the EVM pattern).
///
/// **Must be re-called if the IRS contract is rotated** (e.g., during a
/// registry migration), otherwise lookups will hit the stale address.
pub fn set_irs_address(e: &Env, token: &Address, irs: &Address) {
    e.storage().persistent().set(&IRSKey::Registry(token.clone()), irs);
}

/// Returns an IRS cross-contract client for the given token.
///
/// Panics with `IdentityRegistryNotSet` if no IRS has been configured —
/// the module is not usable until `set_irs_address` has been called.
pub fn get_irs_client<'a>(e: &'a Env, token: &Address) -> IRSReadClient<'a> {
    let irs: Address = e
        .storage()
        .persistent()
        .get(&IRSKey::Registry(token.clone()))
        .unwrap_or_else(|| panic_with_error!(e, ModuleError::IdentityRegistryNotSet));
    IRSReadClient::new(e, &irs)
}

/// Extracts the numeric ISO 3166-1 country code from any
/// [`CountryRelation`] variant, regardless of individual/organization type.
pub fn country_code(relation: &CountryRelation) -> u32 {
    match relation {
        CountryRelation::Individual(rel) => match rel {
            IndividualCountryRelation::Residence(c)
            | IndividualCountryRelation::Citizenship(c)
            | IndividualCountryRelation::SourceOfFunds(c)
            | IndividualCountryRelation::TaxResidency(c) => *c,
            IndividualCountryRelation::Custom(_, c) => *c,
        },
        CountryRelation::Organization(rel) => match rel {
            OrganizationCountryRelation::Incorporation(c)
            | OrganizationCountryRelation::OperatingJurisdiction(c)
            | OrganizationCountryRelation::TaxJurisdiction(c)
            | OrganizationCountryRelation::SourceOfFunds(c) => *c,
            OrganizationCountryRelation::Custom(_, c) => *c,
        },
    }
}
