use soroban_sdk::{contracttype, panic_with_error, Address, Env};

#[cfg(not(feature = "certora"))]
use crate::non_fungible::{
    royalties::{emit_set_default_royalty, emit_set_token_royalty},
};


use crate::non_fungible::{
    Base, NonFungibleTokenError, OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD,
};

/// Storage container for royalty information
#[contracttype]
pub struct RoyaltyInfo {
    pub receiver: Address,
    pub basis_points: u32,
}

/// Storage keys for royalty data
#[contracttype]
pub enum NFTRoyaltiesStorageKey {
    DefaultRoyalty,
    TokenRoyalty(u32),
}

impl Base {
    /// Sets the global default royalty information for the entire collection.
    /// This will be used for all tokens that don't have specific royalty
    /// information.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%,
    ///   10000 = 100%).
    ///
    /// # Events
    ///
    /// * topics - `["set_default_royalty", receiver: Address]`
    /// * data - `[basis_points: u32]`
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidRoyaltyAmount`] - If the royalty
    ///   amount is higher than 10_000 (100%) basis points.
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. Most likely,
    /// you want to invoke it from a constructor or from another function
    /// with admin-only authorization.
    pub fn set_default_royalty(e: &Env, receiver: &Address, basis_points: u32) {
        // check if basis points is valid
        if basis_points > 10000 {
            panic_with_error!(e, NonFungibleTokenError::InvalidRoyaltyAmount);
        }

        // Store the default royalty information
        let key = NFTRoyaltiesStorageKey::DefaultRoyalty;
        let royalty_info = RoyaltyInfo { receiver: receiver.clone(), basis_points };
        e.storage().instance().set(&key, &royalty_info);
        #[cfg(not(feature = "certora"))]
        emit_set_default_royalty(e, receiver, basis_points);
    }

    /// Sets the royalty information for a specific token.
    /// This must be called during minting, as royalties are immutable after
    /// minting.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `receiver` - The address that should receive royalty payments.
    /// * `basis_points` - The royalty percentage in basis points (100 = 1%,
    ///   10000 = 100%).
    ///
    /// # Events
    ///
    /// * topics - `["set_token_royalty", receiver: Address, token_id: u32]`
    /// * data - `[basis_points: u32]`
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidRoyaltyAmount`] - If the royalty
    ///   amount is higher than 10_000 (100%) basis points.
    /// * refer to [`Base::owner_of`] errors.
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. Most likely,
    /// you want to invoke it from a constructor or from another function
    /// with admin-only authorization.
    pub fn set_token_royalty(e: &Env, token_id: u32, receiver: &Address, basis_points: u32) {
        // check if basis points is valid
        if basis_points > 10000 {
            panic_with_error!(e, NonFungibleTokenError::InvalidRoyaltyAmount);
        }

        // Verify token exists by checking owner
        let _ = Base::owner_of(e, token_id);

        // Store the token royalty information
        let key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
        let royalty_info = RoyaltyInfo { receiver: receiver.clone(), basis_points };
        e.storage().persistent().set(&key, &royalty_info);
        #[cfg(not(feature = "certora"))]
        emit_set_token_royalty(e, receiver, token_id, basis_points);
    }

    /// Removes token-specific royalty information, allowing the token to fall
    /// back to the collection-wide default royalty settings.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    ///
    /// # Events
    ///
    /// * topics - `["remove_token_royalty", token_id: u32]`
    /// * data - `[]`
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. Most likely,
    /// you want to invoke it from a function with admin-only authorization.
    pub fn remove_token_royalty(e: &Env, token_id: u32) {
        // Verify token exists by checking owner
        let _ = Base::owner_of(e, token_id);

        // Remove the token royalty information
        let key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
        e.storage().persistent().remove(&key);
        #[cfg(not(feature = "certora"))]
        super::emit_remove_token_royalty(e, token_id);
    }

    /// Returns `(Address, u32)` - A tuple containing the receiver address and
    /// the royalty amount. If there is no token-specific royalty set, it
    /// returns the default royalty. If there is no default royalty set, it
    /// returns the contract address and zero royalty.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    /// * `sale_price` - The sale price for which royalties are being
    ///   calculated.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    /// * refer to [`Base::owner_of`] errors.
    pub fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
        // Verify token exists by checking owner
        let _ = Base::owner_of(e, token_id);

        // Check if there's a specific royalty for this token
        let token_key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
        if let Some(royalty_info) = e.storage().persistent().get::<_, RoyaltyInfo>(&token_key) {
            e.storage().persistent().extend_ttl(
                &token_key,
                OWNER_TTL_THRESHOLD,
                OWNER_EXTEND_AMOUNT,
            );
            let royalty_amount = sale_price * royalty_info.basis_points as i128 / 10000;
            return (royalty_info.receiver, royalty_amount);
        }

        // Fall back to default royalty if no token-specific royalty is set
        let default_key = NFTRoyaltiesStorageKey::DefaultRoyalty;
        if let Some(royalty_info) = e.storage().instance().get::<_, RoyaltyInfo>(&default_key) {
            let royalty_amount = sale_price * royalty_info.basis_points as i128 / 10000;
            return (royalty_info.receiver, royalty_amount);
        }

        // No royalty set, return zero royalty
        (e.current_contract_address(), 0)
    }
}
