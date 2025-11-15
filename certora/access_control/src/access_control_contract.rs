use stellar_access::access_control::{set_admin, AccessControl};
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_macros::{default_impl};

#[contract]
pub struct FVHarnessAccessControlContract;

#[contractimpl]
impl FVHarnessAccessControlContract {
     pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for FVHarnessAccessControlContract {}