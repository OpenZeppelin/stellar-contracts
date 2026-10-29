use soroban_sdk::{Address, Env};
use stellar_governance::votes::transfer_voting_units;

use crate::non_fungible::{
    overrides::{BurnableOverrides, MintOverrides, MintWithIdOverrides},
    royalties::RoyaltySupport,
    Base, ContractOverrides,
};

/// Marker for contract types that back the votes extension: transfers, mints
/// and burns update voting-unit checkpoints.
pub trait VotesContractType {}

/// Contract type that tracks voting units on every balance change (1 unit per
/// NFT).
pub struct Votes;

impl VotesContractType for Votes {}

impl ContractOverrides for Votes {
    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Votes::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Votes::transfer_from(e, spender, from, to, token_id);
    }
}

impl BurnableOverrides for Votes {
    fn burn(e: &Env, from: &Address, token_id: u32) {
        Votes::burn(e, from, token_id);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        Votes::burn_from(e, spender, from, token_id);
    }
}

// Implemented for `Votes`, so that the royalty existence check
// routes through `Votes`'s `owner_of` (one `Owner` entry per
// token).
impl RoyaltySupport for Votes {}

impl MintWithIdOverrides for Votes {
    fn mint_with_id(e: &Env, to: &Address, token_id: u32) {
        Votes::mint_with_id(e, to, token_id);
    }
}

impl MintOverrides for Votes {
    fn mint(e: &Env, to: &Address) -> u32 {
        Votes::mint(e, to)
    }
}

impl Votes {
    /// Transfers a non-fungible token from `from` to `to`.
    /// Also updates voting units for the respective delegates (1 unit per NFT).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `from` - The address holding the token.
    /// * `to` - The address receiving the transferred token.
    /// * `token_id` - The identifier of the token to be transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::transfer`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Base::transfer(e, from, to, token_id);
        transfer_voting_units(e, Some(from), Some(to), 1);
    }

    /// Transfers a non-fungible token from `from` to `to` using the
    /// approval mechanism. Also updates voting units for the respective
    /// delegates (1 unit per NFT).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer.
    /// * `from` - The address holding the token.
    /// * `to` - The address receiving the transferred token.
    /// * `token_id` - The identifier of the token to be transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::transfer_from`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Base::transfer_from(e, spender, from, to, token_id);
        transfer_voting_units(e, Some(from), Some(to), 1);
    }

    /// Creates a token with the provided `token_id` and assigns it to `to`.
    /// Also updates voting units for the recipient's delegate (1 unit per NFT).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new token.
    /// * `token_id` - The token_id of the new token.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::mint_with_id`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Security Warning
    ///
    /// This function has NO AUTHORIZATION CONTROLS.
    /// The caller must ensure proper authorization before calling.
    pub fn mint_with_id(e: &Env, to: &Address, token_id: u32) {
        Base::mint_with_id(e, to, token_id);
        transfer_voting_units(e, None, Some(to), 1);
    }

    /// Creates a token with the next available `token_id` and assigns it to
    /// `to`. Returns the `token_id` for the newly minted token.
    /// Also updates voting units for the recipient's delegate (1 unit per NFT).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new token.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::mint`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Security Warning
    ///
    /// This function has NO AUTHORIZATION CONTROLS.
    /// The caller must ensure proper authorization before calling.
    pub fn mint(e: &Env, to: &Address) -> u32 {
        let token_id = Base::mint(e, to);
        transfer_voting_units(e, None, Some(to), 1);
        token_id
    }

    /// Destroys the token with `token_id` from `from`.
    /// Also updates voting units for the owner's delegate (1 unit per NFT).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::burn`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn burn(e: &Env, from: &Address, token_id: u32) {
        Base::burn(e, from, token_id);
        transfer_voting_units(e, Some(from), None, 1);
    }

    /// Destroys the token with `token_id` from `from`, by using `spender`s
    /// approval. Also updates voting units for the owner's delegate (1 unit per
    /// NFT).
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
    /// * refer to [`Base::burn_from`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        Base::burn_from(e, spender, from, token_id);
        transfer_voting_units(e, Some(from), None, 1);
    }
}
