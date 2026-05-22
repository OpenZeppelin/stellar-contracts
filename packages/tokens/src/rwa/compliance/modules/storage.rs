//! Storage primitives and shared utilities for compliance modules.
//!
//! Holds the storage-key enum and free functions that back the per-token
//! compliance and IRS bindings exposed through the
//! [`crate::rwa::compliance::modules::ComplianceModule`] trait, plus
//! safe-arithmetic and `CountryRelation` utilities reused across module
//! implementations.

use soroban_sdk::{contracttype, panic_with_error, Address, Env, FromVal, Vec};

use super::{ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};
use crate::rwa::identity_registry_storage::{
    CountryData, CountryDataManagerClient, CountryRelation, IdentityRegistryStorageClient,
    IndividualCountryRelation, OrganizationCountryRelation,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub enum ComplianceModuleStorageKey {
    /// The IRS contract address for a specific token.
    Registry(Address),
    /// The authorized compliance dispatcher for a specific token. Used by
    /// state-mutating modules to authenticate their hook callers.
    Compliance(Address),
}

// ################## QUERY STATE ##################

/// Returns the dispatcher bound to `token`.
///
/// State-mutating modules call this and `require_auth()` on the result to
/// verify a hook call genuinely came from the authorized dispatcher.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose authorized dispatcher is being queried.
///
/// # Errors
///
/// * [`ComplianceModuleError::ComplianceNotSet`] - When no dispatcher has been
///   bound for `token`.
pub fn get_compliance_address(e: &Env, token: &Address) -> Address {
    let key = ComplianceModuleStorageKey::Compliance(token.clone());
    let compliance: Address = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::ComplianceNotSet));
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
    compliance
}

/// Returns a cross-contract client for the IRS bound to `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS client is requested.
///
/// # Errors
///
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for `token`.
pub fn get_irs_client<'a>(e: &'a Env, token: &Address) -> IdentityRegistryStorageClient<'a> {
    let irs = get_irs_address(e, token);
    IdentityRegistryStorageClient::new(e, &irs)
}

/// Returns `account`'s country data entries from the IRS bound to `token`,
/// decoded into the typed [`CountryData`] form.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS the entries are read through.
/// * `account` - The account whose country data is being read.
///
/// # Errors
///
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for `token`.
pub fn get_irs_country_data_entries(
    e: &Env,
    token: &Address,
    account: &Address,
) -> Vec<CountryData> {
    let irs = get_irs_address(e, token);
    let client = CountryDataManagerClient::new(e, &irs);
    let raw_entries = client.get_country_data_entries(account);

    Vec::from_iter(e, raw_entries.iter().map(|entry| CountryData::from_val(e, &entry)))
}

// ################## CHANGE STATE ##################

/// Binds `compliance` as the dispatcher authorized to drive hooks for
/// `token`, overwriting any prior binding.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose dispatcher binding is being configured.
/// * `compliance` - The dispatcher address authorized to call this module's
///   hooks for `token`.
///
/// # Security Warning
///
/// This function performs no authorization checks. Callers must gate access
/// (e.g. with [`stellar_macros::only_owner`]) before invoking it.
pub fn set_compliance_address(e: &Env, token: &Address, compliance: &Address) {
    let key = ComplianceModuleStorageKey::Compliance(token.clone());
    e.storage().persistent().set(&key, compliance);
}

/// Binds `irs` as the Identity Registry Storage contract for `token`,
/// overwriting any prior binding.
///
/// This is the canonical body that each module's
/// `set_identity_registry_storage` trait method wraps with its own auth check.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS is being configured.
/// * `irs` - The IRS contract address.
///
/// # Security Warning
///
/// This function performs no authorization checks. Callers must gate access
/// (e.g. with [`stellar_macros::only_owner`]) before invoking it.
pub fn set_irs_address(e: &Env, token: &Address, irs: &Address) {
    let key = ComplianceModuleStorageKey::Registry(token.clone());
    e.storage().persistent().set(&key, irs);
}

// ################## HELPERS ##################

/// Validates that `amount` is non-negative.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The amount to validate.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
pub fn require_non_negative_amount(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, ComplianceModuleError::InvalidAmount);
    }
}

/// Adds two `i128` values, panicking on overflow.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `left` - The left operand.
/// * `right` - The right operand.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathOverflow`] - When the addition overflows.
pub fn add_i128_or_panic(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_add(right)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MathOverflow))
}

/// Subtracts two `i128` values, panicking on underflow.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `left` - The left operand.
/// * `right` - The right operand.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathUnderflow`] - When the subtraction
///   underflows.
pub fn sub_i128_or_panic(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_sub(right)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MathUnderflow))
}

fn get_irs_address(e: &Env, token: &Address) -> Address {
    let key = ComplianceModuleStorageKey::Registry(token.clone());
    let irs: Address = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::IdentityRegistryNotSet));
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
    irs
}

/// Returns the ISO 3166-1 numeric country code carried by `relation`,
/// regardless of which individual- or organization-side variant it is.
///
/// # Arguments
///
/// * `relation` - The country relation to extract the code from.
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
