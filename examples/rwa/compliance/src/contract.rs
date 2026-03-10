//! RWA Compliance Example Contract.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::{
    compliance::{self as compliance, Compliance, ComplianceHook},
    utils::token_binder::{self as binder, TokenBinder},
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct ComplianceContract;

#[contractimpl]
impl ComplianceContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }

    #[only_role(operator, "manager")]
    pub fn bind_tokens(e: &Env, tokens: Vec<Address>, operator: Address) {
        binder::bind_tokens(e, &tokens);
    }

    pub fn get_token_index(e: &Env, token: Address) -> u32 {
        binder::get_token_index(e, &token)
    }
}

#[contractimpl]
impl TokenBinder for ComplianceContract {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        binder::linked_tokens(e)
    }

    #[only_role(operator, "manager")]
    fn bind_token(e: &Env, token: Address, operator: Address) {
        binder::bind_token(e, &token);
    }

    #[only_role(operator, "manager")]
    fn unbind_token(e: &Env, token: Address, operator: Address) {
        binder::unbind_token(e, &token);
    }
}

#[contractimpl]
impl Compliance for ComplianceContract {
    #[only_role(operator, "manager")]
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        compliance::storage::add_module_to(e, hook, module);
    }

    #[only_role(operator, "manager")]
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        compliance::storage::remove_module_from(e, hook, module);
    }

    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        compliance::storage::get_modules_for_hook(e, hook)
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        compliance::storage::is_module_registered(e, hook, module)
    }

    fn transferred(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        compliance::storage::transferred(e, from, to, amount, token);
    }

    fn created(e: &Env, to: Address, amount: i128, token: Address) {
        compliance::storage::created(e, to, amount, token);
    }

    fn destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        compliance::storage::destroyed(e, from, amount, token);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        compliance::storage::can_transfer(e, from, to, amount, token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        compliance::storage::can_create(e, to, amount, token)
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ComplianceContract {}
