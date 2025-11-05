use crate::ownable::{
    accept_ownership, get_owner, renounce_ownership, set_owner, transfer_ownership, Ownable,
};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use stellar_macros::{default_impl, only_owner};

#[contract]
pub struct FVHarnessContract;

#[contractimpl]
impl FVHarnessContract {
    pub fn __constructor(e: &Env, owner: Address) {
        set_owner(e, &owner);
    }
}

#[contractimpl]
impl Ownable for FVHarnessContract {
    fn get_owner(e: &Env) -> Option<Address> {
        get_owner(e)
    }

    fn transfer_ownership(
        e: &Env,
        new_owner: Address,
        live_until_ledger: u32,
    ) {
        transfer_ownership(e, &new_owner, live_until_ledger);
    }

    fn accept_ownership(e: &Env) {
        accept_ownership(e);
    }

    fn renounce_ownership(e: &Env) {
        renounce_ownership(e);
    }
}
