use cvlr::nondet::{nondet, Nondet};
use cvlr_soroban::nondet_address;
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

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

pub struct ComplianceModuleTrivial;

// we should probably have more than 1 compliance module
// and have ghost implementations like in other places.

impl ComplianceModule for ComplianceModuleTrivial {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        // do nothing
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        // do nothing
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        // do nothing
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        nondet()
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        nondet()
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "")
    }

    fn get_compliance_address(e: &Env) -> Address {
        nondet_address()
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        // do nothing
    }
}