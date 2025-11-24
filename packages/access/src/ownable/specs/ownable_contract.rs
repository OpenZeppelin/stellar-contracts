use crate::ownable::{
    set_owner, Ownable,
};
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_macros::{default_impl, only_owner};

use crate as stellar_access;

#[contract]
pub struct OwnableContract;

#[contractimpl]
impl OwnableContract {
    pub fn ownable_constructor(e: &Env, owner: Address) {
        set_owner(e, &owner);
    }

    #[only_owner]
    pub fn owner_restricted_function(e: &Env) {
    }
}

#[contractimpl]
#[default_impl]
impl Ownable for OwnableContract {}