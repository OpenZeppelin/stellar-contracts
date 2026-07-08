use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::fungible::{
    overrides::{BurnableOverrides, TotalSupplyOverrides},
    Base, ContractOverrides, FungibleTokenError, TOTAL_SUPPLY_EXTEND_AMOUNT,
    TOTAL_SUPPLY_TTL_THRESHOLD,
};

/// Contract type that layers total supply accounting on top of the vanilla
/// [`Base`] behavior.
///
/// For combining total supply tracking with the allowlist or blocklist
/// transfer policies, refer to
/// [`crate::fungible::combinations::TotalSupplyAllowList`] and
/// [`crate::fungible::combinations::TotalSupplyBlockList`].
pub struct TotalSupply;

/// Storage keys for the data associated with the total supply extension of
/// `FungibleToken`
#[contracttype]
pub enum TotalSupplyStorageKey {
    TotalSupply,
}

impl TotalSupplyOverrides for TotalSupply {}

impl ContractOverrides for TotalSupply {}

impl BurnableOverrides for TotalSupply {
    fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupply::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupply::burn_from(e, spender, from, amount);
    }
}

impl TotalSupply {
    /// Returns the total amount of tokens in circulation.
    ///
    /// refer to [`total_supply`] for the inline documentation.
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    /// Creates `amount` of tokens and assigns them to `to`, increasing the
    /// total supply accordingly.
    ///
    /// refer to [`mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        mint(e, to, amount);
    }

    /// Destroys `amount` of tokens from `from` and decreases the total supply
    /// accordingly.
    ///
    /// refer to [`crate::fungible::Base::burn`] and
    /// [`decrease_total_supply`] for the inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        Base::burn(e, from, amount);
        decrease_total_supply(e, amount);
    }

    /// Destroys `amount` of tokens from `from` using the allowance mechanism
    /// and decreases the total supply accordingly.
    ///
    /// refer to [`crate::fungible::Base::burn_from`] and
    /// [`decrease_total_supply`] for the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Base::burn_from(e, spender, from, amount);
        decrease_total_supply(e, amount);
    }
}

// ################## QUERY STATE ##################

/// Returns the total amount of tokens in circulation. Defaults to `0` if no
/// supply is recorded.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn total_supply(e: &Env) -> i128 {
    let key = TotalSupplyStorageKey::TotalSupply;
    if let Some(supply) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(
            &key,
            TOTAL_SUPPLY_TTL_THRESHOLD,
            TOTAL_SUPPLY_EXTEND_AMOUNT,
        );
        supply
    } else {
        0
    }
}

// ################## CHANGE STATE ##################

/// Creates `amount` of tokens and assigns them to `to`, increasing the total
/// supply accordingly.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
///
/// # Errors
///
/// * [`FungibleTokenError::MathOverflow`] - When the total supply overflows.
/// * refer to [`crate::fungible::Base::update`] errors.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[amount: i128]`
///
/// # Security Warning
///
/// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
///
/// It is the responsibility of the implementer to establish appropriate
/// access controls to ensure that only authorized accounts can execute
/// minting operations. Failure to implement proper authorization could
/// lead to security vulnerabilities and unauthorized token creation.
///
/// The implementation will typically look similar to the following
/// (pseudo-code):
///
/// ```ignore
/// let admin = read_administrator(e);
/// admin.require_auth();
/// ```
pub fn mint(e: &Env, to: &Address, amount: i128) {
    increase_total_supply(e, amount);
    Base::mint(e, to, amount);
}

// ################## LOW-LEVEL HELPERS ##################

/// Adds `amount` to the total supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The amount to be added to the total supply.
///
/// # Errors
///
/// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
/// * [`FungibleTokenError::MathOverflow`] - When the total supply overflows.
///
/// # Notes
///
/// This is a raw accounting helper: it does not move any balances and does
/// not emit events. [`mint`] should be preferred unless a custom minting
/// flow is being composed.
pub fn increase_total_supply(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, FungibleTokenError::LessThanZero);
    }
    let Some(new_total_supply) = total_supply(e).checked_add(amount) else {
        panic_with_error!(e, FungibleTokenError::MathOverflow);
    };
    e.storage().persistent().set(&TotalSupplyStorageKey::TotalSupply, &new_total_supply);
}

/// Subtracts `amount` from the total supply.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The amount to be subtracted from the total supply.
///
/// # Errors
///
/// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
/// * [`FungibleTokenError::MathOverflow`] - When `amount` exceeds the recorded
///   total supply.
///
/// # Notes
///
/// This is a raw accounting helper: it does not move any balances and does
/// not emit events. `amount` can only exceed the recorded supply when
/// balances were created without supply accounting (e.g. via
/// [`crate::fungible::Base::mint`]); mixing tracked and untracked flows in
/// the same contract is a bug.
pub fn decrease_total_supply(e: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, FungibleTokenError::LessThanZero);
    }
    let supply = total_supply(e);
    if amount > supply {
        panic_with_error!(e, FungibleTokenError::MathOverflow);
    }
    e.storage().persistent().set(&TotalSupplyStorageKey::TotalSupply, &(supply - amount));
}
