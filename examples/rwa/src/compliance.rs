//! # Compliance Contract
//!
//! Implements the modular compliance framework for RWA tokens.
//! This contract manages compliance modules and validates transfers
//! according to registered rules.

use soroban_sdk::{contract, contractimpl, contractmeta, symbol_short, Address, Env, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::rwa::compliance::{
    storage::ComplianceStorage,
    Compliance, ComplianceHook, ComplianceModuleClient,
    emit_module_added, emit_module_removed,
};

contractmeta!(
    key = "Description",
    val = "Modular compliance system for RWA tokens"
);

/// Role for compliance administrators
pub const COMPLIANCE_ADMIN_ROLE: soroban_sdk::Symbol = symbol_short!("COMP_ADM");

#[contract]
pub struct ComplianceContract;

#[contractimpl]
impl Compliance for ComplianceContract {
    #[only_role(operator, "COMP_ADM")]
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        ComplianceStorage::add_module_to(e, hook.clone(), module.clone());
        emit_module_added(e, hook, module);
    }

    #[only_role(operator, "COMP_ADM")]
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        ComplianceStorage::remove_module_from(e, hook.clone(), module.clone());
        emit_module_removed(e, hook, module);
    }

    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        ComplianceStorage::get_modules_for_hook(e, hook)
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        ComplianceStorage::is_module_registered(e, hook, module)
    }

    fn transferred(e: &Env, from: Address, to: Address, amount: i128) {
        let modules = ComplianceStorage::get_modules_for_hook(e, ComplianceHook::Transferred);
        for module in modules.iter() {
            let client = ComplianceModuleClient::new(e, &module);
            client.on_transfer(&from, &to, &amount);
        }
    }

    fn created(e: &Env, to: Address, amount: i128) {
        let modules = ComplianceStorage::get_modules_for_hook(e, ComplianceHook::Created);
        for module in modules.iter() {
            let client = ComplianceModuleClient::new(e, &module);
            client.on_created(&to, &amount);
        }
    }

    fn destroyed(e: &Env, from: Address, amount: i128) {
        let modules = ComplianceStorage::get_modules_for_hook(e, ComplianceHook::Destroyed);
        for module in modules.iter() {
            let client = ComplianceModuleClient::new(e, &module);
            client.on_destroyed(&from, &amount);
        }
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool {
        let modules = ComplianceStorage::get_modules_for_hook(e, ComplianceHook::CanTransfer);
        for module in modules.iter() {
            let client = ComplianceModuleClient::new(e, &module);
            if !client.can_transfer(&from, &to, &amount) {
                return false;
            }
        }
        true
    }

    fn can_create(e: &Env, to: Address, amount: i128) -> bool {
        let modules = ComplianceStorage::get_modules_for_hook(e, ComplianceHook::CanCreate);
        for module in modules.iter() {
            let client = ComplianceModuleClient::new(e, &module);
            if !client.can_create(&to, &amount) {
                return false;
            }
        }
        true
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ComplianceContract {}

#[contractimpl]
impl ComplianceContract {
    /// Initializes the compliance contract with an admin
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &admin, &COMPLIANCE_ADMIN_ROLE);
    }
}
