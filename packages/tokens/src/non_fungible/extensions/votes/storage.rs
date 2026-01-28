use soroban_sdk::{Address, Env};
use stellar_governance::votes::transfer_voting_units;

use crate::non_fungible::{Base, ContractOverrides};

pub struct NonFungibleVotes;

impl ContractOverrides for NonFungibleVotes {
    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        NonFungibleVotes::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        NonFungibleVotes::transfer_from(e, spender, from, to, token_id);
    }
}

impl NonFungibleVotes {
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
    /// * topics - `["Transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["DelegateVotesChanged", delegate: Address]`
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
    /// * topics - `["Transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["DelegateVotesChanged", delegate: Address]`
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
    /// * refer to [`Base::mint`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["Mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["DelegateVotesChanged", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Security Warning
    ///
    /// This function has NO AUTHORIZATION CONTROLS.
    /// The caller must ensure proper authorization before calling.
    pub fn mint(e: &Env, to: &Address, token_id: u32) {
        Base::mint(e, to, token_id);
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
    /// * refer to [`Base::sequential_mint`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["Mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["DelegateVotesChanged", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Security Warning
    ///
    /// This function has NO AUTHORIZATION CONTROLS.
    /// The caller must ensure proper authorization before calling.
    pub fn sequential_mint(e: &Env, to: &Address) -> u32 {
        let token_id = Base::sequential_mint(e, to);
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
    /// * topics - `["Burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["DelegateVotesChanged", delegate: Address]`
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
    /// * topics - `["Burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// * topics - `["DelegateVotesChanged", delegate: Address]`
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
