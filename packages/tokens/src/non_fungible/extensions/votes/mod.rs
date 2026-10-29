pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracttrait, Address, Env};
pub use storage::{Votes, VotesContractType};

use crate::non_fungible::NonFungibleToken;

/// Votes Trait for Non-Fungible Token
///
/// The `NonFungibleVotes` trait extends the `NonFungibleToken` trait to expose
/// vote tracking with delegation. Each NFT counts as one voting unit; units
/// move with every mint, burn and transfer when the contract type includes
/// [`Votes`] (alone or combined via
/// [`crate::non_fungible::combinations::Compose`]).
///
/// This trait is designed to be used in conjunction with the
/// `NonFungibleToken` trait. Implementing it without `Votes` (or a votes
/// combination) in the `Compose` list does not compile: the bound requires
/// [`VotesContractType`].
///
/// # Notes
///
/// The default bodies forward to the storage helpers in
/// [`stellar_governance::votes`]. The contract type is responsible for calling
/// `transfer_voting_units` on balance changes; an empty `impl` is enough when
/// `Compose` lists `Votes`.
#[contracttrait]
pub trait NonFungibleVotes: NonFungibleToken<ContractType: VotesContractType> {
    /// Returns the current voting power (delegated votes) of an account.
    ///
    /// Returns `0` if the account has no delegated voting power or does not
    /// exist in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query voting power for.
    fn get_votes(e: &Env, account: Address) -> u128 {
        stellar_governance::votes::get_votes(e, &account)
    }

    /// Returns the voting power (delegated votes) of an account at a specific
    /// past ledger sequence number.
    ///
    /// Returns `0` if the account had no delegated voting power at the given
    /// ledger or does not exist in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query voting power for.
    /// * `ledger` - The ledger sequence number to query (must be in the past).
    ///
    /// # Errors
    ///
    /// * [`stellar_governance::votes::VotesError::FutureLookup`] - If `ledger`
    ///   >= current ledger sequence number.
    fn get_votes_at_checkpoint(e: &Env, account: Address, ledger: u32) -> u128 {
        stellar_governance::votes::get_votes_at_checkpoint(e, &account, ledger)
    }

    /// Returns the current total supply of voting units.
    ///
    /// This tracks all voting units in circulation (regardless of delegation
    /// status), not just delegated votes.
    ///
    /// Returns `0` if no voting units exist.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_total_supply(e: &Env) -> u128 {
        stellar_governance::votes::get_total_supply(e)
    }

    /// Returns the total supply of voting units at a specific past ledger
    /// sequence number.
    ///
    /// This tracks all voting units in circulation (regardless of delegation
    /// status), not just delegated votes.
    ///
    /// Returns `0` if there were no voting units at the given ledger.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `ledger` - The ledger sequence number to query (must be in the past).
    ///
    /// # Errors
    ///
    /// * [`stellar_governance::votes::VotesError::FutureLookup`] - If `ledger`
    ///   >= current ledger sequence number.
    fn get_total_supply_at_checkpoint(e: &Env, ledger: u32) -> u128 {
        stellar_governance::votes::get_total_supply_at_checkpoint(e, ledger)
    }

    /// Returns the current delegate for an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to query the delegate for.
    ///
    /// # Returns
    ///
    /// * `Some(Address)` - The delegate address (may be the account itself if
    ///   self-delegated).
    /// * `None` - If the account has never delegated. An account whose delegate
    ///   is `None` has **no active voting power**; it must call
    ///   [`NonFungibleVotes::delegate`] (even to itself) before its votes are
    ///   counted.
    fn get_delegate(e: &Env, account: Address) -> Option<Address> {
        stellar_governance::votes::get_delegate(e, &account)
    }

    /// Delegates voting power from `account` to `delegatee`.
    ///
    /// To reclaim voting power (i.e. "undelegate"), call this with
    /// `delegatee` set to `account` (self-delegation). There is no
    /// separate undelegate operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The account delegating its voting power.
    /// * `delegatee` - The account receiving the delegated voting power.
    ///
    /// # Events
    ///
    /// * topics - `["delegate_changed", delegator: Address]`
    /// * data - `[from_delegate: Option<Address>, to_delegate: Address]`
    ///   (`from_delegate` is left out of the data when it is `None`)
    ///
    /// * topics - `["delegate_votes_changed", delegate: Address]`
    /// * data - `[previous_votes: u128, new_votes: u128]`
    ///
    /// # Notes
    ///
    /// Authorization for `account` is required.
    fn delegate(e: &Env, account: Address, delegatee: Address) {
        stellar_governance::votes::delegate(e, &account, &delegatee);
    }
}
