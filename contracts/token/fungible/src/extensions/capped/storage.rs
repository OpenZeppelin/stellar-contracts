use soroban_sdk::{contracttype, symbol_short, unwrap::UnwrapOptimized, Env, String, Symbol};

use crate::storage::{INSTANCE_EXTEND_AMOUNT, INSTANCE_TTL_THRESHOLD};

/// Storage key that maps to [`Metadata`]
pub const CAP_KEY: Symbol = symbol_short!("CAP");

/// Storage container for token metadata
#[contracttype]
pub struct Metadata {
    pub cap: u128,
}

/// Query the maximum supply of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Returns
///
/// the maximum supply of tokens.
fn query_cap(e: &Env) -> u128 {
    e.storage().instance().get(&CAP_KEY).unwrap_optimized()
}

/// Checks if new `amount` of tokens will exceed the maximum supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The new amount of tokens to be added to the total supply.
///
/// # Returns
///
/// - `true` if the new amount of tokens will not exceed the maximum supply,
/// - `false` otherwise.
///
/// # Notes
///
/// We recommend using [`crate::burnable::burn_from()`] when implementing
/// this function.
fn check_cap(e: &Env, amount: i128) -> bool {
    let cap = e.storage().instance().get(&CAP_KEY).unwrap_optimized();
    e.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);
    let total_supply = e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0);
    return cap >= amount + total_supply;
}
