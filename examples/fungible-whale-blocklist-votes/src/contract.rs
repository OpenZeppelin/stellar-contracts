//! Whale BlockList Votes Example Contract.
//!
//! Demonstrates a governance token with total supply tracking, burning, and a
//! custom transfer policy that cannot be expressed by a curated contract
//! type: a blocklist with a "whale exemption" — a blocked account may still
//! transfer, approve or burn while it holds more than
//! [`WHALE_THRESHOLD`] tokens.
//!
//! The contract type is the curated `Compose<(BlockList, FungibleVotes)>`,
//! whose `BlockListContractType` marker makes `FungibleBlockList` a regular
//! trait implementation, alongside `FungibleTotalSupply` (served from the
//! voting checkpoints), `Votes` and `FungibleBurnable`.
//!
//! The curated combination enforces the *strict* blocklist, so the whale
//! exemption still has to displace it with method-level overrides: the
//! bodies of `transfer`, `transfer_from`, `approve`, `burn` and `burn_from`
//! run the whale check and then delegate to the votes-aware flow. The
//! contract type's own policy code never executes on those paths — it
//! provides the marker, the total supply routing, and a fail-closed backstop:
//! removing one of the overrides reverts that path to strict blocking rather
//! than to no blocking.

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, MuxedAddress, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_governance::votes::Votes;
use stellar_macros::only_owner;
use stellar_tokens::fungible::{
    blocklist::{BlockList, FungibleBlockList},
    burnable::FungibleBurnable,
    total_supply::FungibleTotalSupply,
    votes::FungibleVotes,
    Base, Compose, FungibleToken, FungibleTokenError,
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

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Base::set_metadata(
            e,
            7,
            String::from_str(e, "Whale BlockList Votes Token"),
            String::from_str(e, "WBVT"),
        );
        set_owner(e, &owner);
    }

    #[only_owner]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        FungibleVotes::mint(e, &to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = Compose<(BlockList, FungibleVotes)>;

    fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
        enforce_whale_check(e, &[&from, &to.address()]);
        FungibleVotes::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        enforce_whale_check(e, &[&from, &to]);
        FungibleVotes::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        enforce_whale_check(e, &[&owner]);
        Base::approve(e, &owner, &spender, amount, live_until_ledger);
    }
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

#[contractimpl(contracttrait)]
impl FungibleBurnable for ExampleContract {
    fn burn(e: &Env, from: Address, amount: i128) {
        enforce_whale_check(e, &[&from]);
        FungibleVotes::burn(e, &from, amount);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        enforce_whale_check(e, &[&from]);
        FungibleVotes::burn_from(e, &spender, &from, amount);
    }
}

// The total supply is served from the voting checkpoints.
#[contractimpl(contracttrait)]
impl FungibleTotalSupply for ExampleContract {}

#[contractimpl(contracttrait)]
impl Votes for ExampleContract {}

#[contractimpl(contracttrait)]
impl Ownable for ExampleContract {}
