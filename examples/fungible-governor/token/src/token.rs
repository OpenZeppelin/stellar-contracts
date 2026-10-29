use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_macros::only_owner;
use stellar_tokens::fungible::{votes::{FungibleVotes, Votes}, Base, Compose, FungibleToken};

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Base::set_metadata(
            e,
            7,
            String::from_str(e, "Governance Token"),
            String::from_str(e, "GOV"),
        );
        set_owner(e, &owner);
    }

    #[only_owner]
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        <Self as FungibleToken>::ContractType::mint(e, to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for TokenContract {
    type ContractType = Compose<(Votes,)>;
}

#[contractimpl(contracttrait)]
impl FungibleVotes for TokenContract {}

#[contractimpl(contracttrait)]
impl Ownable for TokenContract {}
