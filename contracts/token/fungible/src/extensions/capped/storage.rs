use soroban_sdk::{panic_with_error, symbol_short, unwrap::UnwrapOptimized, Env, Symbol};

use crate::{
    storage::{INSTANCE_EXTEND_AMOUNT, INSTANCE_TTL_THRESHOLD},
    FungibleTokenError, StorageKey,
};

/// Storage key that maps to [`Cap`]
pub const CAP_KEY: Symbol = symbol_short!("CAP");

pub fn set_cap(e: &Env, cap: i128) {
    e.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);
    e.storage().instance().set(&CAP_KEY, &cap);
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
pub fn query_cap(e: &Env) -> i128 {
    e.storage().instance().get(&CAP_KEY).unwrap_optimized()
}

/// Panicks if new `amount` of tokens will exceed the maximum supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The new amount of tokens to be added to the total supply.
///
/// # Notes
///
/// We recommend using [`crate::burnable::burn_from()`] when implementing
/// this function.
pub fn check_cap(e: &Env, amount: i128) {
    let cap: i128 = e.storage().instance().get(&CAP_KEY).unwrap_optimized();
    e.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);
    let total_supply = e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0);
    if cap < amount + total_supply {
        panic_with_error!(e, FungibleTokenError::ExceededCap)
    }
}
