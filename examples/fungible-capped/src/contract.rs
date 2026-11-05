//! Capped Example Contract.
//!
//! Demonstrates an example usage of the `capped` extension: the maximum
//! supply is set in the constructor and exposed through `FungibleCapped`.
//! Listing `Capped` in the `Compose` list selects the `Capped` contract type,
//! whose mint enforces the cap, so the contract's own `mint` function only
//! has to mint through the contract type.
//!
//! **IMPORTANT**: this example is for demonstration purposes, and authorization
//! is not taken into consideration

use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_tokens::fungible::{
    capped::{Capped, FungibleCapped},
    total_supply::{FungibleTotalSupply, TotalSupply},
    Compose, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, cap: i128) {
        Capped::set_cap(e, cap);
    }

    pub fn mint(e: &Env, to: Address, amount: i128) {
        // Resolves to the `Capped` mint: checks the cap, then mints through the
        // supply counter.
        <Self as FungibleToken>::ContractType::mint(e, &to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = Compose<(Capped, TotalSupply)>;
}

#[contractimpl(contracttrait)]
impl FungibleTotalSupply for ExampleContract {}

#[contractimpl(contracttrait)]
impl FungibleCapped for ExampleContract {}
