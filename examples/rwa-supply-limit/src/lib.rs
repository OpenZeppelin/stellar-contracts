#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{
            add_i128_or_panic, set_compliance_address, sub_i128_or_panic, verify_required_hooks,
            ComplianceModuleStorageKey,
        },
        supply_limit::{
            storage::{
                get_internal_supply, get_supply_limit, set_internal_supply, set_supply_limit,
            },
            SupplyLimit, SupplyLimitSet,
        },
    },
    ComplianceHook,
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct SupplyLimitContract;

fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
}

fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).expect("admin must be set")
}

fn require_module_admin_or_compliance_auth(e: &Env) {
    if let Some(compliance) =
        e.storage().instance().get::<_, Address>(&ComplianceModuleStorageKey::Compliance)
    {
        compliance.require_auth();
    } else {
        get_admin(e).require_auth();
    }
}

#[contractimpl]
impl SupplyLimitContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl SupplyLimit for SupplyLimitContract {
    fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, limit);
        set_supply_limit(e, &token, limit);
        SupplyLimitSet { token, limit }.publish(e);
    }

    fn pre_set_internal_supply(e: &Env, token: Address, supply: i128) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, supply);
        set_internal_supply(e, &token, supply);
    }

    fn get_supply_limit(e: &Env, token: Address) -> i128 {
        get_supply_limit(e, &token)
    }

    fn get_internal_supply(e: &Env, token: Address) -> i128 {
        get_internal_supply(e, &token)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        vec![e, ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed]
    }

    fn verify_hook_wiring(e: &Env) {
        verify_required_hooks(e, Self::required_hooks(e));
    }

    fn on_created(e: &Env, _to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);
        let current = get_internal_supply(e, &token);
        set_internal_supply(e, &token, add_i128_or_panic(e, current, amount));
    }

    fn on_destroyed(e: &Env, _from: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        stellar_tokens::rwa::compliance::modules::storage::require_non_negative_amount(e, amount);
        let current = get_internal_supply(e, &token);
        set_internal_supply(e, &token, sub_i128_or_panic(e, current, amount));
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
