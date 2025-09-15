//! # Compliance Contract
//!
//! Implements the modular compliance framework for RWA tokens.
//! This contract manages compliance modules and validates transfers
//! according to registered rules.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::rwa::compliance::{
    storage::{
        add_module_to, can_create, can_transfer, created, destroyed, get_modules_for_hook,
        is_module_registered, remove_module_from, transferred,
    },
    Compliance, ComplianceHook,
};

/// Role for compliance administrators
pub const COMPLIANCE_ADMIN_ROLE: soroban_sdk::Symbol = symbol_short!("COMP_ADM");

#[contract]
pub struct ComplianceContract;

#[contractimpl]
impl Compliance for ComplianceContract {
    #[only_role(operator, "COMP_ADM")]
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        add_module_to(e, hook.clone(), module.clone());
    }

    #[only_role(operator, "COMP_ADM")]
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address) {
        remove_module_from(e, hook.clone(), module.clone());
    }

    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        get_modules_for_hook(e, hook)
    }

    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        is_module_registered(e, hook, module)
    }

    fn transferred(e: &Env, from: Address, to: Address, amount: i128) {
        transferred(e, from, to, amount);
    }

    fn created(e: &Env, to: Address, amount: i128) {
        created(e, to, amount);
    }

    fn destroyed(e: &Env, from: Address, amount: i128) {
        destroyed(e, from, amount);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool {
        can_transfer(e, from, to, amount)
    }

    fn can_create(e: &Env, to: Address, amount: i128) -> bool {
        can_create(e, to, amount)
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
