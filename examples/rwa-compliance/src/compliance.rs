//! Compliance dispatcher contract.
//!
//! Implements the `Compliance` trait (which extends `TokenBinder`), routing
//! hook calls to registered compliance modules. All heavy lifting is delegated
//! to the storage helpers in `stellar_tokens::rwa::compliance::storage`.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::{
    compliance::{storage as compliance_storage, Compliance, ComplianceHook},
    utils::token_binder::{self as binder, TokenBinder},
};

#[contract]
pub struct ComplianceContract;

#[contractimpl]
impl ComplianceContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &symbol_short!("admin"), &admin);
    }
}

#[contractimpl]
impl Compliance for ComplianceContract {
    #[only_role(operator, "admin")]
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        compliance_storage::add_module_to(e, hook, module);
    }

    #[only_role(operator, "admin")]
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        compliance_storage::remove_module_from(e, hook, module);
    }

    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        compliance_storage::get_modules_for_hook(e, hook)
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        compliance_storage::is_module_registered(e, hook, module)
    }

    fn transferred(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        compliance_storage::transferred(e, from, to, amount, token);
    }

    fn created(e: &Env, to: Address, amount: i128, token: Address) {
        compliance_storage::created(e, to, amount, token);
    }

    fn destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        compliance_storage::destroyed(e, from, amount, token);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        compliance_storage::can_transfer(e, from, to, amount, token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        compliance_storage::can_create(e, to, amount, token)
    }
}

#[contractimpl]
impl TokenBinder for ComplianceContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        binder::linked_tokens(e)
    }

    #[only_role(operator, "admin")]
    fn bind_token(e: &Env, token: Address, operator: Address) {
        binder::bind_token(e, &token);
    }

    #[only_role(operator, "admin")]
    fn unbind_token(e: &Env, token: Address, operator: Address) {
        binder::unbind_token(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ComplianceContract {}
