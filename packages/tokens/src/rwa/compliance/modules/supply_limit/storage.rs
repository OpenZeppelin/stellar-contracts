use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::rwa::compliance::modules::{
    storage::{add_i128_or_panic, require_non_negative_amount, sub_i128_or_panic},
    supply_limit::{emit_supply_count_updated, emit_supply_limit_set},
    ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

#[contracttype]
#[derive(Clone)]
pub enum SupplyLimitStorageKey {
    /// Per-token cap on the tracked circulating supply.
    SupplyLimit(Address),
    /// Per-token running supply counter maintained by this module.
    SupplyCount(Address),
}

// ################## QUERY STATE ##################

/// Returns the configured supply cap for `token`. Returns `0` when no limit
/// has been configured, which blocks all mints until [`set_supply_limit`]
/// is called.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_supply_limit(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    if let Some(value) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        0
    }
}

/// Returns the running supply counter tracked by this module for `token`.
/// Returns `0` when no entry exists.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_supply_count(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::SupplyCount(token.clone());
    if let Some(value) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        0
    }
}

// ################## CHANGE STATE ##################

/// Sets the supply cap for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit` - The new supply cap.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `limit` is negative.
///
/// # Events
///
/// * topics - `["supply_limit_set", token: Address]`
/// * data - `[limit: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_supply_limit(e: &Env, token: &Address, limit: i128) {
    require_non_negative_amount(e, limit);
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    e.storage().persistent().set(&key, &limit);
    emit_supply_limit_set(e, token, limit);
}

/// Records a mint of `amount` against `token`, incrementing the running
/// supply counter. Panics when the new supply would exceed the configured
/// limit.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `_to` - The recipient address. Recorded only for the emitted event.
/// * `amount` - The minted amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::SupplyLimitExceeded`] - When the new tracked
///   supply would exceed the configured limit.
/// * [`ComplianceModuleError::MathOverflow`] - When the running supply addition
///   overflows.
///
/// # Events
///
/// * topics - `["supply_count_updated", token: Address]`
/// * data - `[supply: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_created(e: &Env, _to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let next = add_i128_or_panic(e, get_supply_count(e, token), amount);
    if next > get_supply_limit(e, token) {
        panic_with_error!(e, ComplianceModuleError::SupplyLimitExceeded);
    }
    set_supply_count(e, token, next);
}

/// Records a burn of `amount` against `token`, decrementing the running
/// supply counter.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `_from` - The address whose tokens were burned. Recorded only for the
///   emitted event.
/// * `amount` - The burned amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MathUnderflow`] - When the running supply
///   subtraction underflows.
///
/// # Events
///
/// * topics - `["supply_count_updated", token: Address]`
/// * data - `[supply: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_destroyed(e: &Env, _from: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let next = sub_i128_or_panic(e, get_supply_count(e, token), amount);
    if next < 0 {
        panic_with_error!(e, ComplianceModuleError::MathUnderflow);
    }
    set_supply_count(e, token, next);
}

// ################## LOW-LEVEL HELPERS ##################

/// Writes the supply counter entry for `token` directly to persistent
/// storage, replacing any existing value, and emits a
/// [`crate::rwa::compliance::modules::supply_limit::SupplyCountUpdated`]
/// event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `supply` - The new running supply to record.
///
/// # Events
///
/// * topics - `["supply_count_updated", token: Address]`
/// * data - `[supply: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks and skips the supply-cap
/// invariant. Callers must enforce both before invoking it.
pub fn set_supply_count(e: &Env, token: &Address, supply: i128) {
    let key = SupplyLimitStorageKey::SupplyCount(token.clone());
    e.storage().persistent().set(&key, &supply);
    emit_supply_count_updated(e, token, supply);
}
