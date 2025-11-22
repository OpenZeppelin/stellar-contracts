use soroban_sdk::{contract, contractimpl, Address, Env};
use crate::pausable::{self, Pausable};

#[contract]
pub struct PausableContract;

#[contractimpl]
impl Pausable for PausableContract {
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);
    }
}
