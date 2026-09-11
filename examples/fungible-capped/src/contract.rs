//! Capped Example Contract.
//!
//! Demonstrates an example usage of `capped` module by
//! implementing a capped mint mechanism, and setting the maximum supply
//! at the constructor.
//!
//! **IMPORTANT**: this example is for demonstration purposes, and authorization
//! is not taken into consideration

use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
use stellar_tokens::fungible::{
    capped::{check_cap, set_cap},
    total_supply::{FungibleTotalSupply, TotalSupply},
    Compose, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, cap: i128) {
        set_cap(e, cap);
    }

    pub fn mint(e: &Env, to: Address, amount: i128) {
        // Both calls are routed through the contract type so the supply is
        // read and increased consistently.
        check_cap(e, amount, <Self as FungibleToken>::ContractType::total_supply(e));
        <Self as FungibleToken>::ContractType::mint(e, &to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = Compose<(TotalSupply,)>;
}

#[contractimpl(contracttrait)]
impl FungibleTotalSupply for ExampleContract {}
