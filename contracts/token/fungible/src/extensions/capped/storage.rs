use soroban_sdk::{panic_with_error, symbol_short, Env, Symbol};

use crate::{FungibleTokenError, StorageKey};

/// Storage key
pub const CAP_KEY: Symbol = symbol_short!("CAP");

/// Set the maximum supply of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::InvalidCap`] - Occurs when the provided cap is
///   negative.
///
/// # Notes
///
/// We recommend using this function in the constructor of your smart contract.
/// Cap functionality is designed to be used in conjunction with the `mintable`
/// extension.
pub fn set_cap(e: &Env, cap: i128) {
    if cap < 0 {
        panic_with_error!(e, FungibleTokenError::InvalidCap);
    }
    e.storage().instance().set(&CAP_KEY, &cap);
}

/// Query the maximum supply of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::CapNotSet`] - Occurs when the cap has not been set.
///
/// # Returns
///
/// the maximum supply of tokens.
pub fn query_cap(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&CAP_KEY)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::CapNotSet))
}

/// Panics if new `amount` of tokens will exceed the maximum supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The new amount of tokens to be added to the total supply.
///
/// # Errors
///
/// * [`FungibleTokenError::CapNotSet`] - Occurs when the cap has not been set.
/// * [`FungibleTokenError::ExceededCap`] - Occurs when the new amount of tokens
///   will exceed the cap.
pub fn check_cap(e: &Env, amount: i128) {
    let cap: i128 = e
        .storage()
        .instance()
        .get(&CAP_KEY)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::CapNotSet));
    let total_supply = e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0);
    if cap < amount + total_supply {
        panic_with_error!(e, FungibleTokenError::ExceededCap);
    }
}
