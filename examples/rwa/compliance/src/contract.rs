//! RWA Compliance Example Contract.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::{
    compliance::{self as compliance, AccountSnapshot, Compliance, ComplianceHook, TransferKind},
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
}

#[contractimpl(contracttrait)]
impl TokenBinder for ComplianceContract {
    #[only_role(operator, "manager")]
    fn bind_token(e: &Env, token: Address, operator: Address) {
        binder::bind_token(e, &token);
    }

    #[only_role(operator, "manager")]
    fn unbind_token(e: &Env, token: Address, operator: Address) {
        binder::unbind_token(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl Compliance for ComplianceContract {
    #[only_role(operator, "manager")]
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        compliance::storage::add_module_to(e, hook, module);
    }

    #[only_role(operator, "manager")]
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        compliance::storage::remove_module_from(e, hook, module);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ComplianceContract {}
