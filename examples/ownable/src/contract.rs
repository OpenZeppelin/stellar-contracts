//! Pausable Example Contract.
//!
//! Demonstrates an example usage of `stellar_pausable` moddule by
//! implementing an emergency stop mechanism that can be triggered only by the
//! owner account.
//!
//! Counter can be incremented only when `unpaused` and reset only when
//! `paused`.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

use stellar_default_impl_macro::default_impl;
use stellar_ownable::Ownable;
use stellar_ownable_macro::only_owner;

#[contracttype]
pub enum DataKey {
    Owner,
    Counter,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        e.storage().instance().set(&DataKey::Counter, &0);
    }

    #[only_owner]
    pub fn increment(e: &Env, caller: Address) -> i32 {
        let mut counter: i32 =
            e.storage().instance().get(&DataKey::Counter).expect("counter should be set");

        counter += 1;

        e.storage().instance().set(&DataKey::Counter, &counter);

        counter
    }
}

#[default_impl]
#[contractimpl]
impl Ownable for ExampleContract {}
