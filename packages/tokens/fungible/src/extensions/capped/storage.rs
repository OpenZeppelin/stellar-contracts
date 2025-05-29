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
/// * We recommend using this function in the constructor of your smart
///   contract.
/// * Cap functionality is designed to be used in the `mint` function
///   definition.
/// * This function DOES NOT enforce that the cap must be greater than or equal
///   to the current total supply. While this may deviate from common
///   assumptions (e.g., treating `supply_cap >= total_supply` as an invariant),
///   it allows for more flexible use-cases. For instance, a contract owner
///   might decide to permanently reduce the token supply by burning tokens
///   later, and setting a lower cap ahead of time effectively prevents any
///   further minting until the total supply falls below the new cap.
pub fn set_cap(e: &Env, cap: i128) {
    if cap < 0 {
        panic_with_error!(e, FungibleTokenError::InvalidCap);
    }
    e.storage().instance().set(&CAP_KEY, &cap);
}

/// Returns the maximum supply of tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::CapNotSet`] - Occurs when the cap has not been set.
pub fn query_cap(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&CAP_KEY)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::CapNotSet))
}

/// Panics if new `amount` of tokens added to the current supply will exceed the
/// maximum supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The new amount of tokens to be added to the total supply.
///
/// # Errors
///
/// * refer to [`query_cap`] errors.
/// * [`FungibleTokenError::MathOverflow`] - Occurs when the sum of the new
///   amount and the current total supply will overflow.
/// * [`FungibleTokenError::ExceededCap`] - Occurs when the new amount of tokens
///   will exceed the cap.
pub fn check_cap(e: &Env, amount: i128) {
    let cap: i128 = query_cap(e);
    let total_supply: i128 = e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0);
    let Some(sum) = total_supply.checked_add(amount) else {
        panic_with_error!(e, FungibleTokenError::MathOverflow);
    };
    if cap < sum {
        panic_with_error!(e, FungibleTokenError::ExceededCap);
    }
}
