//! Allowlist Votes Example Contract.
//!
//! Demonstrates a governance token restricted to an allowlist: only allowed
//! accounts can transfer, approve or burn, every balance movement updates the
//! voting checkpoints, and minting is capped.
//!
//! The overriding behaviors are combined in the contract type with
//! `Compose<(AllowList, FungibleVotes)>`; `FungibleBurnable` and the capped
//! helpers are additive and layered on top. The total supply backing the cap
//! check is served from the voting checkpoints through
//! `FungibleTotalSupply`.

use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_governance::votes::Votes;
use stellar_macros::only_owner;
use stellar_tokens::fungible::{
    allowlist::{AllowList, FungibleAllowList},
    burnable::FungibleBurnable,
    capped::{check_cap, set_cap},
    total_supply::FungibleTotalSupply,
    votes::FungibleVotes,
    Base, Compose, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address, cap: i128) {
        Base::set_metadata(
            e,
            7,
            String::from_str(e, "Allowlist Votes Token"),
            String::from_str(e, "AVT"),
        );
        set_owner(e, &owner);
        set_cap(e, cap);
        AllowList::allow_user(e, &owner);
    }

    #[only_owner]
    pub fn mint(e: &Env, to: Address, amount: i128) {
        check_cap(e, amount, Self::total_supply(e));
        FungibleVotes::mint(e, &to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = Compose<(AllowList, FungibleVotes)>;
}

#[contractimpl(contracttrait)]
impl FungibleAllowList for ExampleContract {
    #[only_owner]
    fn allow_user(e: &Env, user: Address, _operator: Address) {
        AllowList::allow_user(e, &user);
    }

    #[only_owner]
    fn disallow_user(e: &Env, user: Address, _operator: Address) {
        AllowList::disallow_user(e, &user);
    }
}

// The total supply is served from the voting checkpoints.
#[contractimpl(contracttrait)]
impl FungibleTotalSupply for ExampleContract {}

#[contractimpl(contracttrait)]
impl FungibleBurnable for ExampleContract {}

#[contractimpl(contracttrait)]
impl Votes for ExampleContract {}

#[contractimpl(contracttrait)]
impl Ownable for ExampleContract {}
