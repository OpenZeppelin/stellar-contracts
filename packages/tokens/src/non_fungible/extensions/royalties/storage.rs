use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::non_fungible::{
    royalties::{emit_remove_token_royalty, emit_set_default_royalty, emit_set_token_royalty},
    Base, NonFungibleTokenError, OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD,
};

/// Type-level name of the royalties extension, for use in a
/// [`crate::non_fungible::combinations::Compose`] list.
///
/// Royalties is additive: it does not override the base behavior, so listing
/// it is purely declarative and does not affect the resolved contract type.
/// Royalty support is enabled by implementing
/// [`crate::non_fungible::royalties::NonFungibleRoyalties`], whether or not
/// `Royalties` is listed.
pub enum Royalties {}

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
    /// **IMPORTANT**: This function lacks authorization controls. It should
    /// generally be invoked from a constructor or from another function with
    /// admin-only authorization.
    ///
    /// This function does not take a `token_id` and performs no token
    /// existence check, so it works the same for every contract type. That is
    /// why it lives on [`Base`] and not on
    /// [`crate::non_fungible::royalties::RoyaltySupport`].
    pub fn set_default_royalty(e: &Env, receiver: &Address, basis_points: u32) {
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
}

// ################## LOW-LEVEL HELPERS ##################

// The functions below skip the token existence check on purpose. What
// "existing" means depends on the contract type's ownership model, so the
// check belongs to [`crate::non_fungible::royalties::RoyaltySupport`]: its
// default bodies call `Self::owner_of` and then delegate here. They are
// `pub(crate)` because routing around the existence check must not be
// possible from outside the library.

/// Sets the royalty information for a specific token, **without** verifying
/// that the token exists.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_id` - The identifier of the token.
/// * `receiver` - The address that should receive royalty payments.
/// * `basis_points` - The royalty percentage in basis points (100 = 1%, 10000 =
///   100%).
///
/// # Events
///
/// * topics - `["set_token_royalty", receiver: Address, token_id: u32]`
/// * data - `[basis_points: u32]`
///
/// # Errors
///
/// * [`NonFungibleTokenError::InvalidRoyaltyAmount`] - If the royalty amount is
///   higher than 10_000 (100%) basis points.
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not verify that the token exists.
/// Callers must establish existence first, through the ownership model of
/// the contract type in use.
/// [`crate::non_fungible::royalties::RoyaltySupport::set_token_royalty`]
/// does exactly that and is the intended entry point.
pub(crate) fn set_token_royalty_unchecked(
    e: &Env,
    token_id: u32,
    receiver: &Address,
    basis_points: u32,
) {
    // check if basis points is valid
    if basis_points > 10000 {
        panic_with_error!(e, NonFungibleTokenError::InvalidRoyaltyAmount);
    }

    // Store the token royalty information
    let key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
    let royalty_info = RoyaltyInfo { receiver: receiver.clone(), basis_points };
    e.storage().persistent().set(&key, &royalty_info);

    emit_set_token_royalty(e, receiver, token_id, basis_points);
}

/// Removes token-specific royalty information, **without** verifying that
/// the token exists.
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
/// # Security Warning
///
/// **IMPORTANT**: This function does not verify that the token exists.
/// Callers must establish existence first, through the ownership model of
/// the contract type in use.
/// [`crate::non_fungible::royalties::RoyaltySupport::remove_token_royalty`]
/// does exactly that and is the intended entry point.
pub(crate) fn remove_token_royalty_unchecked(e: &Env, token_id: u32) {
    // Remove the token royalty information
    let key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
    e.storage().persistent().remove(&key);

    emit_remove_token_royalty(e, token_id);
}

/// Returns `(Address, i128)` - A tuple containing the receiver address and
/// the royalty amount, **without** verifying that the token exists. If there
/// is no token-specific royalty set, the default royalty is returned. If
/// there is no default royalty set, the contract address and zero royalty
/// are returned.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_id` - The identifier of the token.
/// * `sale_price` - The sale price for which royalties are being calculated.
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not verify that the token exists.
/// Callers must establish existence first, through the ownership model of
/// the contract type in use.
/// [`crate::non_fungible::royalties::RoyaltySupport::royalty_info`] does
/// exactly that and is the intended entry point.
pub(crate) fn royalty_info_unchecked(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
    // Check if there's a specific royalty for this token
    let token_key = NFTRoyaltiesStorageKey::TokenRoyalty(token_id);
    if let Some(royalty_info) = e.storage().persistent().get::<_, RoyaltyInfo>(&token_key) {
        e.storage().persistent().extend_ttl(&token_key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);
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
