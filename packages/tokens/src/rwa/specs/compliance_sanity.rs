use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::rwa::{compliance::{Compliance, ComplianceHook}, specs::compliance::ComplianceContract};

#[rule]
pub fn add_module_to_sanity(e: Env) {
    let hook = ComplianceHook::nondet();
    let module = nondet_address();
    let operator = nondet_address();
    ComplianceContract::add_module_to(&e, hook, module, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_module_from_sanity(e: Env) {
    let hook = ComplianceHook::nondet();
    let module = nondet_address();
    let operator = nondet_address();
    ComplianceContract::remove_module_from(&e, hook, module, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_modules_for_hook_sanity(e: Env) {
    let hook = ComplianceHook::nondet();
    let _ = ComplianceContract::get_modules_for_hook(&e, hook);
    cvlr_satisfy!(true);
}

#[rule]
pub fn is_module_registered_sanity(e: Env) {
    let module = nondet_address();
    let hook = ComplianceHook::nondet();
    let _ = ComplianceContract::is_module_registered(&e, hook, module);
    cvlr_satisfy!(true);
}

#[rule]
pub fn transferred_sanity(e: Env) {
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    let token = nondet_address();
    ComplianceContract::transferred(&e, from, to, amount, token);
    cvlr_satisfy!(true);
}

#[rule]
pub fn created_sanity(e: Env) {
    let to = nondet_address();
    let amount: i128 = nondet();
    let token = nondet_address();
    ComplianceContract::created(&e, to, amount, token);
    cvlr_satisfy!(true);
}

#[rule]
pub fn destroyed_sanity(e: Env) {
    let from = nondet_address();
    let amount: i128 = nondet();
    let token = nondet_address();
    ComplianceContract::destroyed(&e, from, amount, token);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_transfer_sanity(e: Env) {
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    let token = nondet_address();
    ComplianceContract::can_transfer(&e, from, to, amount, token);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_create_sanity(e: Env) {
    let to = nondet_address();
    let amount: i128 = nondet();
    let token = nondet_address();
    ComplianceContract::can_create(&e, to, amount, token);
    cvlr_satisfy!(true);
}
