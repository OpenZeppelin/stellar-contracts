use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_constants::{OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD};

use crate::{
    extensions::royalties::MAX_ROYALTY_BASIS_POINTS,
    non_fungible::NonFungibleTokenError,
    Base,
};

/// Storage container for royalty information
#[contracttype]
pub struct RoyaltyInfo {
    pub receiver: Address,
    pub basis_points: u32,
}

/// Storage keys for royalty data
#[contracttype]
pub enum RoyaltyStorageKey {
    DefaultRoyalty,
    TokenRoyalty(u32),
}

impl Base {
    /// Sets the global default royalty information for the entire collection.
    /// This will be used for all tokens that don't have specific royalty information.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 = 100%).
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::RoyaltyTooHigh`] - If the royalty percentage exceeds
    ///   the maximum allowed value.
    ///
    /// # Events
    ///
    /// * topics - `["royalty", receiver: Address]`
    /// * data - `[basis_points: u32]`
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. Most likely,
    /// you want to invoke it from a constructor or from another function
    /// with admin-only authorization.
    pub fn set_default_royalty(e: &Env, receiver: &Address, basis_points: u32) {
        // Validate the royalty percentage
        if basis_points > MAX_ROYALTY_BASIS_POINTS {
            panic_with_error!(e, NonFungibleTokenError::RoyaltyTooHigh);
        }

        // Store the default royalty information
        let key = RoyaltyStorageKey::DefaultRoyalty;
        let royalty_info = RoyaltyInfo { receiver: receiver.clone(), basis_points };
        e.storage().instance().set(&key, &royalty_info);
    }

    /// Sets the royalty information for a specific token.
    /// This must be called during minting, as royalties are immutable after minting.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 = 100%).
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not exist.
    /// * [`NonFungibleTokenError::RoyaltyTooHigh`] - If the royalty percentage exceeds
    ///   the maximum allowed value.
    /// * [`NonFungibleTokenError::RoyaltyAlreadySet`] - If attempting to set royalties
    ///   for a token that already has royalty information.
    ///
    /// # Events
    ///
    /// * topics - `["tokroyal", token_id: u32]`
    /// * data - `[receiver: Address, basis_points: u32]`
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. Most likely,
    /// you want to invoke it from a constructor or from another function
    /// with admin-only authorization.
    pub fn set_token_royalty(e: &Env, token_id: u32, receiver: &Address, basis_points: u32) {
        // Verify token exists by checking owner
        let _ = Base::owner_of(e, token_id);

        // Validate the royalty percentage
        if basis_points > MAX_ROYALTY_BASIS_POINTS {
            panic_with_error!(e, NonFungibleTokenError::RoyaltyTooHigh);
        }

        // Check if royalty is already set for this token
        let key = RoyaltyStorageKey::TokenRoyalty(token_id);
        if e.storage().persistent().has(&key) {
            panic_with_error!(e, NonFungibleTokenError::RoyaltyAlreadySet);
        }

        // Store the token royalty information
        let royalty_info = RoyaltyInfo { receiver: receiver.clone(), basis_points };
        e.storage().persistent().set(&key, &royalty_info);
        e.storage().persistent().extend_ttl(&key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);
    }

    /// Returns the royalty information for a token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `sale_price` - The sale price for which royalties are being calculated.
    ///
    /// # Returns
    ///
    /// * `(Address, u32)` - A tuple containing the receiver address and the royalty amount.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not exist.
    pub fn royalty_info(e: &Env, token_id: u32, sale_price: u32) -> (Address, u32) {
        // Verify token exists by checking owner
        let _ = Base::owner_of(e, token_id);

        // Check if there's a specific royalty for this token
        let token_key = RoyaltyStorageKey::TokenRoyalty(token_id);
        if let Some(royalty_info) = e.storage().persistent().get::<_, RoyaltyInfo>(&token_key) {
            e.storage().persistent().extend_ttl(
                &token_key,
                OWNER_TTL_THRESHOLD,
                OWNER_EXTEND_AMOUNT,
            );
            let royalty_amount =
                (sale_price as u64 * royalty_info.basis_points as u64 / 10000) as u32;
            return (royalty_info.receiver, royalty_amount);
        }

        // Fall back to default royalty if no token-specific royalty is set
        let default_key = RoyaltyStorageKey::DefaultRoyalty;
        if let Some(royalty_info) = e.storage().instance().get::<_, RoyaltyInfo>(&default_key) {
            let royalty_amount =
                (sale_price as u64 * royalty_info.basis_points as u64 / 10000) as u32;
            return (royalty_info.receiver, royalty_amount);
        }

        // No royalty set, return zero royalty
        (e.current_contract_address(), 0)
    }
}
