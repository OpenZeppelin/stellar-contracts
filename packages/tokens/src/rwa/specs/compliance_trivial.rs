use cvlr::nondet::{nondet, Nondet};
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use crate::rwa::{
    compliance::{Compliance, ComplianceHook, ComplianceModule, storage},
    utils::token_binder::{TokenBinder, bind_token, linked_tokens, unbind_token},
};

pub struct ComplianceTrivial;

impl TokenBinder for ComplianceTrivial {
    fn linked_tokens(e: &Env) -> Vec<Address> {
        linked_tokens(e)
    }

    fn bind_token(e: &Env, token: Address, operator: Address) {
        bind_token(e, &token);
    }

    fn unbind_token(e: &Env, token: Address, operator: Address) {
        unbind_token(e, &token);
    }
}

impl Compliance for ComplianceTrivial {
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        // do nothing
    }

    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        // do nothing
    }

    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        Vec::new(e) // no modules - todo
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        nondet()
    }

    fn transferred(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        // do nothing
    }

    fn created(e: &Env, to: Address, amount: i128, token: Address) {
        // do nothing
    }

    fn destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        // do nothing
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        nondet()
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        nondet()
    }
}
