//! Shared storage and helper utilities for compliance modules.
//!
//! Centralizes compliance-address ownership/auth checks, safe arithmetic
//! guards, and identity registry storage (IRS) resolution helpers.

use soroban_sdk::{contracttype, panic_with_error, Address, Env, FromVal, String, Vec};

use super::{ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};
use crate::rwa::{
    compliance::{ComplianceClient, ComplianceHook},
    identity_registry_storage::{
        CountryData, CountryDataManagerClient, CountryRelation, IdentityRegistryStorageClient,
        IndividualCountryRelation, OrganizationCountryRelation,
    },
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
pub enum ComplianceModuleStorageKey {
    /// Maps to the compliance contract address for this module instance.
    Compliance,
    /// Caches successful required-hook verification for this module instance.
    HooksVerified,
    /// The IRS contract address for a specific token.
    Registry(Address),
}

// ################## QUERY STATE ##################

/// Returns the stored compliance address.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`ComplianceModuleError::ComplianceNotSet`] - When no compliance contract
///   has been configured yet.
pub fn get_compliance_address(e: &Env) -> Address {
    let key = ComplianceModuleStorageKey::Compliance;
    if let Some(addr) = e.storage().instance().get::<_, Address>(&key) {
        addr
    } else {
        panic_with_error!(e, ComplianceModuleError::ComplianceNotSet)
    }
}

/// Returns `true` if the hook wiring has already been verified for this
/// module instance (cached after the first successful check).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn hooks_verified(e: &Env) -> bool {
    let key = ComplianceModuleStorageKey::HooksVerified;
    e.storage().instance().has(&key)
}

/// Returns an IRS cross-contract client for the given token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS client is requested.
///
/// # Errors
///
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for this token.
pub fn get_irs_client<'a>(e: &'a Env, token: &Address) -> IdentityRegistryStorageClient<'a> {
    let irs = get_irs_address(e, token);
    IdentityRegistryStorageClient::new(e, &irs)
}

/// Returns the typed country data entries for `account` resolved via the
/// token's configured IRS.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS client is requested.
/// * `account` - The account whose country data should be read.
///
/// # Errors
///
/// * [`ComplianceModuleError::IdentityRegistryNotSet`] - When no IRS has been
///   configured for this token.
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

/// Persists the compliance contract address that governs this module.
///
/// This is a **one-time** operation. Once set, the compliance address cannot
/// be changed. This prevents unauthorized rebinding after initial deployment.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `compliance` - The address of the compliance contract.
///
/// # Errors
///
/// * [`ComplianceModuleError::ComplianceAlreadySet`] - When the compliance
///   address has already been set.
///
/// # Security Warning
///
/// This helper performs **no authorization checks**. It must only be called
/// during contract initialization or from entrypoints that are strictly
/// restricted to an admin or token owner. Exposing this as, or calling it
/// from, a publicly accessible module entrypoint would allow unauthorized
/// parties to bind the compliance contract for this module.
pub fn set_compliance_address(e: &Env, compliance: &Address) {
    let key = ComplianceModuleStorageKey::Compliance;
    if e.storage().instance().has(&key) {
        panic_with_error!(e, ComplianceModuleError::ComplianceAlreadySet);
    }
    e.storage().instance().set(&key, compliance);
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on every hook in `required`. Caches the result on success
/// so subsequent calls are a single storage read.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `required` - The list of hooks this module requires to be registered.
///
/// # Errors
///
/// * [`ComplianceModuleError::ComplianceNotSet`] - When the compliance contract
///   has not been configured yet.
/// * [`ComplianceModuleError::MissingRequiredHook`] - When any required hook is
///   not registered, which means the deployment is misconfigured and internal
///   state would drift.
pub fn verify_required_hooks(e: &Env, required: Vec<ComplianceHook>) {
    if hooks_verified(e) {
        return;
    }

    let compliance = get_compliance_address(e);
    let self_addr = e.current_contract_address();
    let client = ComplianceClient::new(e, &compliance);

    for hook in required.iter() {
        if !client.is_module_registered(&hook, &self_addr) {
            panic_with_error!(e, ComplianceModuleError::MissingRequiredHook);
        }
    }

    let vkey = ComplianceModuleStorageKey::HooksVerified;
    e.storage().instance().set(&vkey, &true);
}

/// Low-level helper that stores the IRS contract address for a given token.
///
/// This function **does not perform any authorization checks**. It directly
/// updates the per-token Identity Registry Storage pointer in persistent
/// storage.
///
/// SAFETY: This must only be called from initialization logic or from
/// admin-gated entrypoints that have already enforced the appropriate
/// ownership and authorization checks. Do **not** expose this helper directly
/// as a public contract method.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose IRS is being configured.
/// * `irs` - The IRS contract address.
pub fn set_irs_address(e: &Env, token: &Address, irs: &Address) {
    let key = ComplianceModuleStorageKey::Registry(token.clone());
    e.storage().persistent().set(&key, irs);
}

// ################## HELPERS ##################

/// Panics with [`ComplianceModuleError::InvalidAmount`] if `amount` is
/// negative.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The amount to validate.
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

/// Allocates a Soroban [`String`] from a static `&str` for use as a
/// module name.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The name to convert.
pub fn module_name(e: &Env, name: &str) -> String {
    String::from_str(e, name)
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

/// Extracts the numeric ISO 3166-1 country code from any
/// [`CountryRelation`] variant, regardless of individual/organization type.
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
