//! Pausable Example Contract.
//!
//! Demonstrates an example usage of `stellar_pausable` moddule by
//! implementing an emergency stop mechanism that can be triggered only by the
//! owner account.
//!
//! Counter can be incremented only when `unpaused` and reset only when
//! `paused`.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttrait, contracttype, panic_with_error, Address,
    Env,
};
use stellar_ownable::{Ownable, OwnableExt};
use stellar_pausable::Pausable;
use stellar_pausable_macros::{when_not_paused, when_paused};

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
        Self::set_owner(e, &owner);
        e.storage().instance().set(&DataKey::Counter, &0);
    }

    #[when_not_paused]
    pub fn increment(e: &Env) -> i32 {
        let mut counter: i32 =
            e.storage().instance().get(&DataKey::Counter).expect("counter should be set");

        counter += 1;

        e.storage().instance().set(&DataKey::Counter, &counter);

        counter
    }

    #[when_paused]
    pub fn emergency_reset(e: &Env) {
        e.storage().instance().set(&DataKey::Counter, &0);
    }
}

#[contracttrait]
impl Ownable for ExampleContract {}

#[contracttrait( ext = OwnableExt)]
impl Pausable for ExampleContract {}
