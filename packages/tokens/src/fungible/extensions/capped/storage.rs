use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::fungible::{
    overrides::{BurnableOverrides, MintOverrides, TotalSupplyOverrides},
    total_supply::{mint, total_supply, TotalSupply},
    ContractOverrides, FungibleTokenError,
};

/// Contract type that enforces a maximum total supply on top of the
/// [`TotalSupply`] accounting.
///
/// Minting through the contract type (`Self::ContractType::mint`, refer to
/// [`MintOverrides`]) checks the cap first, then mints through the supply
/// counter. Transfers, approvals and burns behave as on [`TotalSupply`].
///
/// A cap needs the supply to be tracked, so in a
/// [`crate::fungible::combinations::Compose`] list `Capped` requires
/// `TotalSupply` next to it, e.g. `Compose<(Capped, TotalSupply)>`. For
/// combining the cap with the allowlist or blocklist transfer policies, with
/// `RWA` or with `Vault`, list them together, e.g.
/// `Compose<(AllowList, Capped, TotalSupply)>`; each combination resolves to
/// its own capped contract type.
pub struct Capped;

/// Marker for the contract types that enforce the cap, backing
/// [`crate::fungible::capped::FungibleCapped`].
///
/// # Why this trait exists
///
/// `FungibleCapped` exposes `cap()` on the contract. Reporting a cap is only
/// honest if the contract type actually enforces it on every path that
/// creates tokens. This marker is implemented only by those contract types:
/// [`Capped`] and its curated combinations (`CappedAllowList`,
/// `CappedBlockList`, `CappedAllowBlockList`, `CappedRWA`, `CappedVault`).
/// `FungibleCapped` requires it on the contract's `ContractType`, so a
/// contract whose `Compose` list does not contain `Capped` cannot expose
/// `cap()`.
///
/// Contract authors never interact with this trait; it only appears as that
/// bound.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not enforce a cap, so `FungibleCapped` cannot be implemented",
    note = "add `Capped` to the `Compose` list, e.g. `Compose<(Capped, TotalSupply)>`"
)]
pub trait CappedContractType {}

impl CappedContractType for Capped {}

impl TotalSupplyOverrides for Capped {}

impl ContractOverrides for Capped {}

impl BurnableOverrides for Capped {
    fn burn(e: &Env, from: &Address, amount: i128) {
        Capped::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Capped::burn_from(e, spender, from, amount);
    }
}

impl MintOverrides for Capped {
    fn mint(e: &Env, to: &Address, amount: i128) {
        Capped::mint(e, to, amount);
    }
}

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

    /// Returns the total amount of tokens in circulation.
    ///
    /// refer to [`total_supply`] for the inline documentation.
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    /// Creates `amount` of tokens and assigns them to `to` if the cap allows
    /// it, increasing the total supply accordingly.
    ///
    /// This is the mint of the [`Capped`] contract type and of its list
    /// combinations. Contracts reach it through `Self::ContractType::mint`.
    ///
    /// refer to [`check_cap`] and [`mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        check_cap(e, amount, total_supply(e));
        mint(e, to, amount);
    }

    /// Destroys `amount` of tokens from `from` and decreases the total supply
    /// accordingly.
    ///
    /// refer to [`TotalSupply::burn`] for the inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupply::burn(e, from, amount);
    }

    /// Destroys `amount` of tokens from `from` using the allowance mechanism
    /// and decreases the total supply accordingly.
    ///
    /// refer to [`TotalSupply::burn_from`] for the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupply::burn_from(e, spender, from, amount);
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
/// * The cap is enforced by the mint of the capped contract types, refer to
///   [`Capped`].
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
