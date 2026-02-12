use soroban_sdk::{Address, Env, MuxedAddress};
use stellar_governance::votes::transfer_voting_units;

use crate::fungible::{Base, ContractOverrides};

pub struct FungibleVotes;

impl ContractOverrides for FungibleVotes {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        FungibleVotes::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        FungibleVotes::transfer_from(e, spender, from, to, amount);
    }
}

impl FungibleVotes {
    /// Transfers `amount` of tokens from `from` to `to`.
    /// Also updates voting units for the respective delegates.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::transfer`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        Base::transfer(e, from, to, amount);
        if amount > 0 {
            transfer_voting_units(e, Some(from), Some(&to.address()), amount as u128);
        }
    }

    /// Transfers `amount` of tokens from `from` to `to` using the
    /// allowance mechanism. Also updates voting units for the respective
    /// delegates.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::transfer_from`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Base::transfer_from(e, spender, from, to, amount);
        if amount > 0 {
            transfer_voting_units(e, Some(from), Some(to), amount as u128);
        }
    }

    /// Creates `amount` of tokens and assigns them to `to`.
    /// Also updates voting units for the recipient's delegate.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new tokens.
    /// * `amount` - The amount of tokens to mint.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::mint`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Security Warning
    ///
    /// This function has NO AUTHORIZATION CONTROLS.
    /// The caller must ensure proper authorization before calling.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        Base::mint(e, to, amount);
        if amount > 0 {
            transfer_voting_units(e, None, Some(to), amount as u128);
        }
    }

    /// Destroys `amount` of tokens from `from`. Updates the total
    /// supply accordingly. Also updates voting units for the owner's
    /// delegate.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose tokens are destroyed.
    /// * `amount` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::burn`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        Base::burn(e, from, amount);
        if amount > 0 {
            transfer_voting_units(e, Some(from), None, amount as u128);
        }
    }

    /// Destroys `amount` of tokens from `from`. Updates the total
    /// supply accordingly. Also updates voting units for the owner's
    /// delegate.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorized to burn the tokens.
    /// * `from` - The account whose tokens are destroyed.
    /// * `amount` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::burn_from`] errors.
    /// * refer to [`transfer_voting_units`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["Burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Base::burn_from(e, spender, from, amount);
        if amount > 0 {
            transfer_voting_units(e, Some(from), None, amount as u128);
        }
    }
}
