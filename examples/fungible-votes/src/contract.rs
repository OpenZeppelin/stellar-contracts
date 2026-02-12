use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_governance::votes::Votes;
use stellar_macros::only_owner;
use stellar_tokens::fungible::{votes::FungibleVotes, Base, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Base::set_metadata(e, 7, String::from_str(e, "My Token"), String::from_str(e, "MTK"));
        set_owner(e, &owner);
    }

    #[only_owner]
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = FungibleVotes;
}

#[contractimpl(contracttrait)]
impl Votes for ExampleContract {}

#[contractimpl(contracttrait)]
impl Ownable for ExampleContract {}
