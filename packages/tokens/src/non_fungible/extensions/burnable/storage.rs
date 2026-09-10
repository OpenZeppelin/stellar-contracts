use soroban_sdk::{Address, Env};

use crate::non_fungible::{burnable::emit_burn, Base};

/// Type-level name of the burnable extension, for use in a
/// [`crate::non_fungible::combinations::Compose`] list.
///
/// Burnable is additive: it does not override the base behavior, so listing
/// it is purely declarative and does not affect the resolved contract type.
/// Burning is enabled by implementing
/// [`crate::non_fungible::burnable::NonFungibleBurnable`], whether or not
/// `Burnable` is listed.
pub enum Burnable {}

impl Base {
    /// Destroys the token with `token_id` from `from`, ensuring ownership
    /// checks, and emits a `burn` event.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn burn(e: &Env, from: &Address, token_id: u32) {
        from.require_auth();
        Base::update(e, Some(from), None, token_id);
        emit_burn(e, from, token_id);
    }

    /// Destroys the token with `token_id` from `from`, ensuring ownership
    /// and approval checks, and emits a `burn` event.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to burn the token on behalf of
    ///   the owner.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::check_spender_approval`] errors.
    /// * refer to [`Base::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        spender.require_auth();
        Base::check_spender_approval(e, spender, from, token_id);
        Base::update(e, Some(from), None, token_id);
        emit_burn(e, from, token_id);
    }
}
