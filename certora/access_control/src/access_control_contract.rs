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
    #[only_admin]
    pub fn admin_function(e: &Env) {
    }

    #[has_role(caller, "role1")]
    pub fn role1_function(e: &Env, caller: Address) {
    }

    #[only_role(caller, "role1")]
    pub fn role1_authorized_function(e: &Env, caller: Address) {
    }

    #[has_any_role(caller, ["role1", "role2"])]
    pub fn role1_or_role2_function(e: &Env, caller: Address) {
    }

    #[only_any_role(caller, ["role1", "role2"])]
    pub fn role1_or_role2_authorized_function(e: &Env, caller: Address) {
    }

    #[has_role(caller1, "role1")]
    #[has_role(caller2, "role2")]
    pub fn role1_and_role2_function(e: &Env, caller1: Address, caller2: Address) {
    }

    #[has_role(caller, "role1")]
    #[has_role(caller, "role2")]
    pub fn role1_and_role2_on_same_addressfunction(e: &Env, caller: Address) {
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for FVHarnessAccessControlContract {}