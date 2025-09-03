//! RWA Example Contract.

use soroban_sdk::{contract, contractimpl, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::fungible::{Base, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env) {
        Base::set_metadata(e, 18, String::from_str(e, "Token"), String::from_str(e, "EXA"));
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = Base;
}

#[contract]
pub struct AnotherContract;

#[contractimpl]
impl AnotherContract {
    pub fn __constructor(e: &Env) {
        Base::set_metadata(e, 18, String::from_str(e, "Token2"), String::from_str(e, "EXA2"));
    }
}
