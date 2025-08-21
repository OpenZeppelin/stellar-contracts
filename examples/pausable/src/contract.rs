//! Pausable Example Contract.
//!
//! Demonstrates an example usage of `stellar_pausable` moddule by
//! implementing an emergency stop mechanism that can be triggered only by the
//! owner account.
//!
//! Counter can be incremented only when `unpaused` and reset only when
//! `paused`.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use stellar_access::{Ownable, Owner};
use stellar_contract_utils::{Pausable, PausableDefault};
use stellar_macros::{only_owner, when_not_paused, when_paused};

#[contracttype]
pub enum DataKey {
    Counter,
}

impl DataKey {
    pub fn set(&self, e: &Env, i: i32) {
        e.storage().instance().set(self, &i);
    }

    pub fn get(&self, e: &Env) -> i32 {
        unsafe { e.storage().instance().get(self).unwrap_unchecked() }
    }
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        Self::set_owner(e, &owner);
        DataKey::Counter.set(e, 0);
    }

    #[when_not_paused]
    pub fn increment(e: &Env) -> i32 {
        let counter = DataKey::Counter.get(e) + 1;
        DataKey::Counter.set(e, counter);
        counter
    }

    #[when_paused]
    pub fn emergency_reset(e: &Env) {
        DataKey::Counter.set(e, 0);
    }
}

#[contractimpl]
impl Ownable for ExampleContract {
    type Impl = Owner;
}

#[contractimpl]
impl Pausable for ExampleContract {
    type Impl = PausableDefault;

    #[only_owner]
    fn pause(e: &Env, caller: &soroban_sdk::Address) {
        Self::Impl::pause(e, caller)
    }

    #[only_owner]
    fn unpause(e: &Env, caller: &soroban_sdk::Address) {
        Self::Impl::unpause(e, caller)
    }
}
