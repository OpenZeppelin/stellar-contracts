//! Pausable Example Contract.
//!
//! Demonstrates an example usage of `openzeppelin_pausable` moddule by implementing
//! an emergency stop mechanism that can be triggered only by the owner account.
//!
//! Counter can be incremented only when `unpaused` and reset only when `paused`.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env,
};

use openzeppelin_pausable::{self as pausable, Pausable};

#[contracttype]
pub enum DataKey {
    Owner,
    Counter,
}

#[contracterror]
pub enum ExampleContractError {
    // ATTENTION !!! - overwrites PausableError
    Unauthorized = 1,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: Env, owner: Address) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        e.storage().instance().set(&DataKey::Counter, &0);
    }

    pub fn increment(e: Env) -> i32 {
        pausable::when_not_paused(&e);

        let mut counter: i32 =
            e.storage().instance().get(&DataKey::Counter).expect("counter should be set");

        counter += 1;

        e.storage().instance().set(&DataKey::Counter, &counter);

        counter
    }

    pub fn emergency_reset(e: Env) {
        pausable::when_paused(&e);

        e.storage().instance().set(&DataKey::Counter, &0);
    }
}

#[contractimpl]
impl Pausable for ExampleContract {
    fn paused(e: Env) -> bool {
        pausable::paused(&e)
    }

    fn pause(e: Env, caller: Address) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        let owner: Address =
            e.storage().instance().get(&DataKey::Owner).expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }

        pausable::pause(&e, &caller);
    }

    fn unpause(e: Env, caller: Address) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        let owner: Address =
            e.storage().instance().get(&DataKey::Owner).expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }

        pausable::unpause(&e, &caller);
    }
}
