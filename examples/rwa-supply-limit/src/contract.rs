use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{
            get_compliance_address, module_name, set_compliance_address, ComplianceModuleStorageKey,
        },
        supply_limit::storage as supply_limit,
        ComplianceModule,
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

    pub fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        require_module_admin_or_compliance_auth(e);
        supply_limit::configure_supply_limit(e, &token, limit);
    }

    pub fn pre_set_supply(e: &Env, token: Address, supply: i128) {
        require_module_admin_or_compliance_auth(e);
        supply_limit::pre_set_supply(e, &token, supply);
    }

    pub fn get_supply_limit(e: &Env, token: Address) -> i128 {
        supply_limit::get_supply_limit(e, &token)
    }

    pub fn get_internal_supply(e: &Env, token: Address) -> i128 {
        supply_limit::get_internal_supply(e, &token)
    }

    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        supply_limit::required_hooks(e)
    }

    pub fn verify_hook_wiring(e: &Env) {
        supply_limit::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for SupplyLimitContract {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(e: &Env, _to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        supply_limit::on_created(e, amount, &token);
    }

    fn on_destroyed(e: &Env, _from: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        supply_limit::on_destroyed(e, amount, &token);
    }

    fn can_transfer(
        _e: &Env,
        _from: Address,
        _to: Address,
        _amount: i128,
        _token: Address,
    ) -> bool {
        true
    }

    fn can_create(e: &Env, _to: Address, amount: i128, token: Address) -> bool {
        supply_limit::can_create(e, amount, &token)
    }

    fn name(e: &Env) -> String {
        module_name(e, "SupplyLimitModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
