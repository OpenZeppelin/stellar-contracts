use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use crate::rwa::{
    compliance::{Compliance, ComplianceHook, ComplianceModule, storage},
    utils::token_binder::{TokenBinder, bind_token, linked_tokens, unbind_token},
};

pub struct ComplianceContract;

impl TokenBinder for ComplianceContract {
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

impl Compliance for ComplianceContract {
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        storage::add_module_to(e, hook, module);
    }

    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        storage::remove_module_from(e, hook, module);
    }

    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        storage::get_modules_for_hook(e, hook)
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        storage::is_module_registered(e, hook, module)
    }

    fn transferred(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        storage::transferred(e, from, to, amount, token);
    }

    fn created(e: &Env, to: Address, amount: i128, token: Address) {
        storage::created(e, to, amount, token);
    }

    fn destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        storage::destroyed(e, from, amount, token);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        storage::can_transfer(e, from, to, amount, token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        storage::can_create(e, to, amount, token)
    }
}

impl ComplianceModule for ComplianceContract {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        // TODO: implement something?
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        // TODO: implement something?
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        // TODO: implement something?
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
       storage::can_transfer(e, from, to, amount, token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        storage::can_create(e, to, amount, token)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        todo!()
    }

    fn get_compliance_address(e: &Env) -> Address {
        todo!()
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        todo!()
    }
}