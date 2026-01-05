use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;
use crate::rwa::compliance::storage;
use crate::rwa::compliance::ComplianceHook;

#[rule]
// after add_module the modules contain the module
// status: verified
pub fn add_module_to_integrity(e: Env) {
    let hook: ComplianceHook = nondet();
    let module = nondet_address();
    storage::add_module_to(&e, hook.clone(), module.clone());
    let modules = storage::get_modules_for_hook(&e, hook);
    let modules_contains_module = modules.contains(&module);
    cvlr_assert!(modules_contains_module);
}

#[rule]
// after remove_module the modules does not contain the modules
// status: spurious violation
pub fn remove_module_from_integrity(e: Env) {
    let hook: ComplianceHook = nondet();
    let module = nondet_address();
    storage::remove_module_from(&e, hook.clone(), module.clone());
    let modules = storage::get_modules_for_hook(&e, hook);
    let modules_contains_module = modules.contains(&module);
    cvlr_assert!(!modules_contains_module);
}

// todo: panic properties for these functions
// should only be called by the bound token.

#[rule]
pub fn transferred_integrity(e: Env) {
    // we would want to say all the hooks are called todo 
}

#[rule]
pub fn created_integrity(e: Env) {
    // we would want to say all the hooks are called todo 
}

#[rule]
pub fn destroyed_integrity(e: Env) {
    // we would want to say all the hooks are called todo 
}

#[rule]
pub fn can_transfer_integrity(e: Env) {
    // should return the and of all the can_transfer of all modules
}

#[rule]
pub fn can_create_integrity(e: Env) {
    // should return the and of all the can_create of all modules
}