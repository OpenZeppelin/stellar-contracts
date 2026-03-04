//! Shared helpers for compliance modules.
//!
//! This module intentionally centralizes:
//! - compliance-address ownership/auth checks
//! - safe arithmetic guards for i128 accounting
//! - lightweight read-only client traits used by modules
//! - identity registry storage (IRS) resolution helpers
//!
//! Keeping these utilities here prevents drift across module implementations
//! and keeps each module focused on business rules only.

use soroban_sdk::{
    contractclient, contracterror, contracttype, panic_with_error, symbol_short, Address, Env,
    String, Symbol, Vec,
};

use crate::rwa::identity_registry_storage::{
    CountryData, CountryRelation, IndividualCountryRelation, OrganizationCountryRelation,
};

const COMPLIANCE_KEY: Symbol = symbol_short!("cmpaddr");

#[contractclient(name = "TokenSupplyViewClient")]
pub trait TokenSupplyView {
    fn total_supply(e: &Env) -> i128;
}

#[contractclient(name = "TokenBalanceViewClient")]
pub trait TokenBalanceView {
    fn balance(e: &Env, id: Address) -> i128;
}

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

pub fn set_compliance_address(e: &Env, compliance: &Address) {
    e.storage().persistent().set(&COMPLIANCE_KEY, compliance);
}

pub fn get_compliance_address(e: &Env) -> Address {
    if !e.storage().persistent().has(&COMPLIANCE_KEY) {
        return e.current_contract_address();
    }
    e.storage()
        .persistent()
        .get::<_, Address>(&COMPLIANCE_KEY)
        .expect("compliance must be set")
}

pub fn require_compliance_auth(e: &Env) -> Address {
    if !e.storage().persistent().has(&COMPLIANCE_KEY) {
        return e.current_contract_address();
    }
    let compliance = get_compliance_address(e);
    compliance.require_auth();
    compliance
}

pub fn require_non_negative_amount(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, ModuleError::InvalidAmount);
    }
}

pub fn checked_add_i128(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_add(right)
        .unwrap_or_else(|| panic_with_error!(e, ModuleError::MathOverflow))
}

pub fn checked_sub_i128(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_sub(right)
        .unwrap_or_else(|| panic_with_error!(e, ModuleError::MathUnderflow))
}

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
