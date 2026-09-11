use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::fungible::{
    total_supply::{mint, total_supply},
    FungibleTokenError,
};

/// Type-level name of the capped extension, for use in a
/// [`crate::fungible::combinations::Compose`] list.
///
/// Capped is additive: it does not change the resolved contract type. A cap
/// needs the supply to be tracked, though, so it requires
/// [`crate::fungible::total_supply::TotalSupply`] in the list, e.g.
/// `Compose<(Capped, TotalSupply)>`; a list with `Capped` but without
/// `TotalSupply` is rejected. The cap is exposed by implementing
/// [`crate::fungible::capped::FungibleCapped`] and enforced by minting
/// through [`Capped::mint`].
pub enum Capped {}

impl Capped {
    /// Returns the maximum supply of tokens.
    ///
    /// refer to [`query_cap`] for the inline documentation.
    pub fn cap(e: &Env) -> i128 {
        query_cap(e)
    }

    /// Sets the maximum supply of tokens.
    ///
    /// refer to [`set_cap`] for the inline documentation.
    pub fn set_cap(e: &Env, cap: i128) {
        set_cap(e, cap);
    }

    /// Creates `amount` of tokens and assigns them to `to` if the cap allows
    /// it, increasing the total supply accordingly.
    ///
    /// Mints through the total supply counter, which is the minting path of
    /// [`crate::fungible::total_supply::TotalSupply`] and of its list
    /// combinations. Contract types with a mint of their own, such as
    /// [`crate::rwa::RWA`], call [`check_cap`] before that mint instead.
    ///
    /// refer to [`check_cap`] and [`mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        check_cap(e, amount, total_supply(e));
        mint(e, to, amount);
    }
}

/// Storage key for the cap value
#[contracttype]
pub enum CapStorageKey {
    Cap,
}

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
/// * This function is best used in the constructor of the smart contract.
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
    e.storage().instance().set(&CapStorageKey::Cap, &cap);
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
        .get(&CapStorageKey::Cap)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::CapNotSet))
}

/// Panics if new `amount` of tokens added to the given `total_supply` will
/// exceed the maximum supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The new amount of tokens to be added to the total supply.
/// * `total_supply` - The current total supply of tokens.
///
/// # Errors
///
/// * [`FungibleTokenError::MathOverflow`] - Occurs when the sum of the new
///   amount and the current total supply will overflow.
/// * [`FungibleTokenError::ExceededCap`] - Occurs when the new amount of tokens
///   will exceed the cap.
/// * refer to [`query_cap`] errors.
pub fn check_cap(e: &Env, amount: i128, total_supply: i128) {
    let cap: i128 = query_cap(e);
    let Some(sum) = total_supply.checked_add(amount) else {
        panic_with_error!(e, FungibleTokenError::MathOverflow);
    };
    if cap < sum {
        panic_with_error!(e, FungibleTokenError::ExceededCap);
    }
}
