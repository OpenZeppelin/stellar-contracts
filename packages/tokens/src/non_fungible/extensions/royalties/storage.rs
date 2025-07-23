use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{
    royalties::{emit_set_default_royalty, emit_set_token_royalty, NonFungibleRoyalties},
    Base, NonFungibleTokenError,
    non_fungible::NonFungibleToken,
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

impl NonFungibleRoyalties for Base {
    type Impl = Self;

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
    fn set_default_royalty(e: &Env, receiver: &Address, basis_points: u32, _operator: &Address) {
        // check if basis points is valid
        if basis_points > 10000 {
            panic_with_error!(e, NonFungibleTokenError::InvalidRoyaltyAmount);
        }

        // Store the default royalty information
        let key = NFTRoyaltiesStorageKey::DefaultRoyalty;
        let royalty_info = RoyaltyInfo { receiver: receiver.clone(), basis_points };
        e.storage().instance().set(&key, &royalty_info);

        emit_set_default_royalty(e, receiver, basis_points);
    }

    /// Sets the royalty information for a specific token.
    fn set_token_royalty(
        e: &Env,
        token_id: u32,
        receiver: &Address,
        basis_points: u32,
        _operator: &Address,
    ) {
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

        emit_set_token_royalty(e, receiver, token_id, basis_points);
    }

    /// Removes token-specific royalty information.
    fn remove_token_royalty(e: &Env, token_id: u32, _operator: &Address) {
        // Verify token exists by checking owner
        let _ = Base::owner_of(e, token_id);

        // Remove the token royalty information
        let key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
        e.storage().persistent().remove(&key);

        super::emit_remove_token_royalty(e, token_id);
    }

    /// Returns `(Address, u32)` - A tuple containing the receiver address and
    /// the royalty amount.
    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
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
