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
use stellar_access::{Ownable, OwnableExt};
use stellar_contract_utils::Pausable;
use stellar_macros::{when_not_paused, when_paused};

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

// #[contracttrait]
impl Pausable for ExampleContract {
    type Impl;

    fn paused(e: &Env) -> bool {
        Self::Impl::paused(e)
    }

    fn pause(e: &Env, caller: &soroban_sdk::Address) {
        Self::Impl::pause(e, caller)
    }

    fn unpause(e: &Env, caller: &soroban_sdk::Address) {
        Self::Impl::unpause(e, caller)
    }

    fn when_not_paused(e: &Env) {
        if Self::paused(e) {
            panic_with_error!(e, PausableError::EnforcedPause);
        }
    }

    fn when_paused(e: &Env) {
        if !Self::paused(e) {
            {
                e.panic_with_error(PausableError::ExpectedPause);
            };
        }
    }
}
