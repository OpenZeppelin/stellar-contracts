use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::rwa::compliance_modules::{
    ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

#[contracttype]
#[derive(Clone)]
pub enum SupplyLimitStorageKey {
    /// Per-token supply cap.
    SupplyLimit(Address),
    /// Per-token internal supply counter (updated via hooks).
    InternalSupply(Address),
}

/// Returns the supply limit for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_supply_limit(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Returns the supply limit for `token`, panicking if not configured.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::MissingLimit`] - When no supply limit has
///   been configured for this token.
pub fn get_supply_limit_or_panic(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    let limit: i128 = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MissingLimit));
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
    limit
}

/// Sets the supply limit for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit` - The maximum total supply.
pub fn set_supply_limit(e: &Env, token: &Address, limit: i128) {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    e.storage().persistent().set(&key, &limit);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the internal supply counter for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_internal_supply(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::InternalSupply(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the internal supply counter for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `supply` - The new supply value.
pub fn set_internal_supply(e: &Env, token: &Address, supply: i128) {
    let key = SupplyLimitStorageKey::InternalSupply(token.clone());
    e.storage().persistent().set(&key, &supply);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}
