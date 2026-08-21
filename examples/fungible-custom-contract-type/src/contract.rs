//! Custom Contract Type Example Contract.
//!
//! Implements the same token as the `fungible-whale-blocklist-votes` example
//! — total supply tracking, voting checkpoints, burning, and a blocklist with
//! a "whale exemption" (a listed account keeps operating while it holds more
//! than [`WHALE_THRESHOLD`] tokens) — through the other composition route: a
//! custom contract type defined in the contract crate.
//!
//! [`WhaleBlockListVotes`] fills the `ContractType` slot directly (the
//! `Compose` machinery cannot name downstream types — implementing
//! `Composable` for a tuple of a local type violates the orphan rule), and
//! the policy lives once in its `ContractOverrides` implementation instead of
//! being repeated across method-level overrides. Claiming the public
//! `BlockListContractType` marker keeps the `FungibleBlockList` trait
//! implementable.
//!
//! The trade against the sibling example: `FungibleBurnable` is not
//! implementable here. Its bound requires `ContractType: BurnableOverrides`,
//! and `BurnableOverrides` has no public path out of stellar-tokens, so no
//! downstream type can satisfy it — burning is exposed through inherent
//! `burn` / `burn_from` entry points with the same signatures instead.

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, MuxedAddress, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_governance::votes::Votes;
use stellar_macros::only_owner;
use stellar_tokens::fungible::{
    blocklist::{BlockList, BlockListContractType, FungibleBlockList},
    total_supply::{FungibleTotalSupply, TotalSupplyOverrides},
    votes::FungibleVotes,
    Base, ContractOverrides, FungibleToken, FungibleTokenError,
};

/// Balance above which a blocked account is exempt from the blocklist.
pub const WHALE_THRESHOLD: i128 = 100;

/// Returns `false` when `account` is blocked and holds no more than
/// [`WHALE_THRESHOLD`] tokens. The balance is read before the balance
/// change of the current invocation is applied.
fn passes(e: &Env, account: &Address) -> bool {
    !BlockList::blocked(e, account) || Base::balance(e, account) > WHALE_THRESHOLD
}

fn enforce_whale_check(e: &Env, accounts: &[&Address]) {
    for account in accounts {
        if !passes(e, account) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
    }
}

/// Contract type combining the whale-exempt blocklist policy with the voting
/// checkpoints of [`FungibleVotes`].
pub struct WhaleBlockListVotes;

// The whale check runs first, then the votes-aware flow. The check cannot be
// composed from `BlockList::transfer` (it would hard-block listed whales and
// double-apply the balance update on top of `FungibleVotes::transfer`), so
// the policy is expressed directly against the storage helpers.
impl ContractOverrides for WhaleBlockListVotes {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        enforce_whale_check(e, &[from, &to.address()]);
        FungibleVotes::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        enforce_whale_check(e, &[from, to]);
        FungibleVotes::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        enforce_whale_check(e, &[owner]);
        Base::approve(e, owner, spender, amount, live_until_ledger);
    }
}

// The marker is public and unsealed, and `WhaleBlockListVotes` is local, so
// claiming it here is legal; it is what makes `FungibleBlockList`
// implementable below.
impl BlockListContractType for WhaleBlockListVotes {}

// The voting checkpoints already track the total supply; the query must be
// routed to them explicitly. The trait's default body reads the dedicated
// supply entry instead, which this token never writes (minting goes through
// `FungibleVotes::mint`), so relying on the default would report 0 forever.
impl TotalSupplyOverrides for WhaleBlockListVotes {
    fn total_supply(e: &Env) -> i128 {
        <FungibleVotes as TotalSupplyOverrides>::total_supply(e)
    }
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Base::set_metadata(
            e,
            7,
            String::from_str(e, "Custom Contract Type Token"),
            String::from_str(e, "CCTT"),
        );
        set_owner(e, &owner);
    }

    #[only_owner]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        FungibleVotes::mint(e, &to, amount);
    }

    // `FungibleBurnable` is not implementable for this contract: its bound is
    // `FungibleToken<ContractType: BurnableOverrides>`, and
    // `BurnableOverrides` is not exported by stellar-tokens, so it cannot be
    // implemented for `WhaleBlockListVotes`. The same entry points are
    // exposed inherently instead.

    pub fn burn(e: &Env, from: Address, amount: i128) {
        enforce_whale_check(e, &[&from]);
        FungibleVotes::burn(e, &from, amount);
    }

    pub fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        enforce_whale_check(e, &[&from]);
        FungibleVotes::burn_from(e, &spender, &from, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = WhaleBlockListVotes;
}

#[contractimpl(contracttrait)]
impl FungibleBlockList for ExampleContract {
    // The query reports the effective policy rather than raw list
    // membership: a listed whale is not blocked.
    fn blocked(e: &Env, account: Address) -> bool {
        !passes(e, &account)
    }

    #[only_owner]
    fn block_user(e: &Env, user: Address, _operator: Address) {
        BlockList::block_user(e, &user);
    }

    #[only_owner]
    fn unblock_user(e: &Env, user: Address, _operator: Address) {
        BlockList::unblock_user(e, &user);
    }
}

// The total supply is served from the voting checkpoints.
#[contractimpl(contracttrait)]
impl FungibleTotalSupply for ExampleContract {}

#[contractimpl(contracttrait)]
impl Votes for ExampleContract {}

#[contractimpl(contracttrait)]
impl Ownable for ExampleContract {}
