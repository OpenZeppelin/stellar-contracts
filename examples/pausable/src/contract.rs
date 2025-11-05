//! Pausable Example Contract.
//!
//! Demonstrates an example usage of Pausable module by
//! implementing an emergency stop mechanism that can be triggered only by the
//! owner account.
//!
//! Counter can be incremented only when `unpaused` and reset only when
//! `paused`.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env,
};
use stellar_access::ownable::{enforce_owner_auth, set_owner, Ownable};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{default_impl, when_not_paused, when_paused};

#[contracttype]
pub enum DataKey {
    Counter,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        set_owner(e, &owner);
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

#[contractimpl]
impl Pausable for ExampleContract {
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    fn pause(e: &Env, caller: Address) {
        // The `caller` parameter is required by the Pausable trait.
        // Alternatively, instead of using Ownable, you can combine Pausable
        // with the Access Control module and apply the
        // #[only_role(caller, "manager")] attribute.
        // For reference, see the `Fungible AllowList Example`.
        let owner: Address = enforce_owner_auth(e);
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized);
        }

        pausable::pause(e);
    }

    fn unpause(e: &Env, caller: Address) {
        // The `caller` parameter is required by the Pausable trait.
        // Alternatively, instead of using Ownable, you can combine Pausable
        // with the Access Control module and apply the
        // #[only_role(caller, "manager")] attribute.
        // For reference, see the `Fungible AllowList Example`.
        let owner: Address = enforce_owner_auth(e);
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized);
        }

        pausable::unpause(e);
    }
}

#[default_impl]
#[contractimpl]
impl Ownable for ExampleContract {}
