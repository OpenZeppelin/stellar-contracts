use stellar_access::ownable::{
    set_owner, Ownable,
};
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_macros::{default_impl};

#[contract]
pub struct FVHarnessOwnableContract;

#[contractimpl]
impl FVHarnessOwnableContract {
    pub fn __constructor(e: &Env, owner: Address) {
        set_owner(e, &owner);
    }
}

#[default_impl]
#[contractimpl]
impl Ownable for FVHarnessOwnableContract {}