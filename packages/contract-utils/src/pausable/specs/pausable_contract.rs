use soroban_sdk::{contract, contractimpl, Address, Env};
use crate::pausable::{self, Pausable};
use stellar_macros::{when_not_paused, when_paused};

use crate as stellar_contract_utils;

#[contract]
pub struct PausableContract;

#[contractimpl]
impl PausableContract {
    pub fn __constructor(_e: &Env) {
    }

    #[when_not_paused]
    pub fn when_not_paused_func(e: &Env) {
    }

    #[when_paused]
    pub fn when_paused_func(e: &Env) {
    }
}

#[contractimpl]
impl Pausable for PausableContract {
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);
    }
}
