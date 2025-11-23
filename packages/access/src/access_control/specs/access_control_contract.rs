use crate::access_control::{AccessControl, *};
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_macros::{default_impl, has_any_role, has_role, only_admin, only_any_role, only_role};

use crate as stellar_access;

#[contract]
pub struct AccessControlContract;

#[contractimpl]
impl AccessControlContract {
    pub fn access_control_constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
    #[only_admin]
    pub fn admin_function(e: &Env) {
    }

    #[has_role(caller, "role1")]
    pub fn role1_func(e: &Env, caller: Address) {
    }

    #[only_role(caller, "role1")]
    pub fn role1_auth_func(e: &Env, caller: Address) {
    }

    #[has_any_role(caller, ["role1", "role2"])]
    pub fn role1_or_role2_func(e: &Env, caller: Address) {
    }

    #[only_any_role(caller, ["role1", "role2"])]
    pub fn role1_or_role2_auth_func(e: &Env, caller: Address) {
    }

    #[has_role(caller1, "role1")]
    #[has_role(caller2, "role2")]
    pub fn role1_and_role2_func(e: &Env, caller1: Address, caller2: Address) {
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for AccessControlContract {}