use soroban_sdk::{Address, Env, String};
use stellar_governance::votes::transfer_voting_units;

use crate::non_fungible::{
    extensions::{
        consecutive::{Consecutive, ConsecutiveContractType},
        enumerable::{Enumerable, EnumerableContractType},
    },
    overrides::{BurnableOverrides, ContractOverrides},
    royalties::RoyaltySupport,
};

/// Contract type combining the [`Enumerable`] bookkeeping with vote
/// checkpoints: every mint, transfer and burn updates the enumeration and
/// moves the corresponding voting unit.
pub struct EnumerableVotes;

// The combined contract type keeps backing the enumerable extension.
impl EnumerableContractType for EnumerableVotes {}

impl ContractOverrides for EnumerableVotes {
    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        EnumerableVotes::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        EnumerableVotes::transfer_from(e, spender, from, to, token_id);
    }
}

impl BurnableOverrides for EnumerableVotes {
    fn burn(e: &Env, from: &Address, token_id: u32) {
        EnumerableVotes::burn(e, from, token_id);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        EnumerableVotes::burn_from(e, spender, from, token_id);
    }
}

// Implemented for `EnumerableVotes`, so that the royalty existence check
// routes through the `Enumerable` ownership model (one `Owner` entry per
// token).
impl RoyaltySupport for EnumerableVotes {}

impl EnumerableVotes {
    /// refer to [`Enumerable::sequential_mint`] and [`transfer_voting_units`]
    /// for the inline documentation.
    pub fn sequential_mint(e: &Env, to: &Address) -> u32 {
        let token_id = Enumerable::sequential_mint(e, to);
        transfer_voting_units(e, None, Some(to), 1);
        token_id
    }

    /// refer to [`Enumerable::non_sequential_mint`] and
    /// [`transfer_voting_units`] for the inline documentation.
    pub fn non_sequential_mint(e: &Env, to: &Address, token_id: u32) {
        Enumerable::non_sequential_mint(e, to, token_id);
        transfer_voting_units(e, None, Some(to), 1);
    }

    /// refer to [`Enumerable::transfer`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Enumerable::transfer(e, from, to, token_id);
        transfer_voting_units(e, Some(from), Some(to), 1);
    }

    /// refer to [`Enumerable::transfer_from`] and [`transfer_voting_units`]
    /// for the inline documentation.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Enumerable::transfer_from(e, spender, from, to, token_id);
        transfer_voting_units(e, Some(from), Some(to), 1);
    }

    /// refer to [`Enumerable::burn`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, token_id: u32) {
        Enumerable::burn(e, from, token_id);
        transfer_voting_units(e, Some(from), None, 1);
    }

    /// refer to [`Enumerable::burn_from`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        Enumerable::burn_from(e, spender, from, token_id);
        transfer_voting_units(e, Some(from), None, 1);
    }
}

/// Contract type combining the [`Consecutive`] accounting model with vote
/// checkpoints: batch minting adds the matching amount of voting units in a
/// single checkpoint update, and every transfer and burn moves the
/// corresponding voting unit.
pub struct ConsecutiveVotes;

// The combined contract type keeps backing the consecutive extension.
impl ConsecutiveContractType for ConsecutiveVotes {}

impl ContractOverrides for ConsecutiveVotes {
    // Ownership resolution and metadata follow the consecutive bucket model
    // unchanged.
    fn owner_of(e: &Env, token_id: u32) -> Address {
        Consecutive::owner_of(e, token_id)
    }

    fn token_uri(e: &Env, token_id: u32) -> String {
        Consecutive::token_uri(e, token_id)
    }

    fn approve(
        e: &Env,
        approver: &Address,
        approved: &Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Consecutive::approve(e, approver, approved, token_id, live_until_ledger);
    }

    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        ConsecutiveVotes::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        ConsecutiveVotes::transfer_from(e, spender, from, to, token_id);
    }
}

impl BurnableOverrides for ConsecutiveVotes {
    fn burn(e: &Env, from: &Address, token_id: u32) {
        ConsecutiveVotes::burn(e, from, token_id);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        ConsecutiveVotes::burn_from(e, spender, from, token_id);
    }
}

// Implemented for `ConsecutiveVotes`, so that the royalty existence check
// routes through `Consecutive`'s `owner_of` instead of `Base`'s direct
// entry lookup.
impl RoyaltySupport for ConsecutiveVotes {}

impl ConsecutiveVotes {
    /// Creates a batch of `amount` tokens for `to` and adds the matching
    /// amount of voting units in a single checkpoint update.
    ///
    /// refer to [`Consecutive::batch_mint`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn batch_mint(e: &Env, to: &Address, amount: u32) -> u32 {
        let last_id = Consecutive::batch_mint(e, to, amount);
        transfer_voting_units(e, None, Some(to), u128::from(amount));
        last_id
    }

    /// refer to [`Consecutive::transfer`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Consecutive::transfer(e, from, to, token_id);
        transfer_voting_units(e, Some(from), Some(to), 1);
    }

    /// refer to [`Consecutive::transfer_from`] and [`transfer_voting_units`]
    /// for the inline documentation.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Consecutive::transfer_from(e, spender, from, to, token_id);
        transfer_voting_units(e, Some(from), Some(to), 1);
    }

    /// refer to [`Consecutive::burn`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, token_id: u32) {
        Consecutive::burn(e, from, token_id);
        transfer_voting_units(e, Some(from), None, 1);
    }

    /// refer to [`Consecutive::burn_from`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        Consecutive::burn_from(e, spender, from, token_id);
        transfer_voting_units(e, Some(from), None, 1);
    }
}
