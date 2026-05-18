use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use stellar_access::access_control;
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{get_compliance_address, module_name, set_compliance_address},
        supply_limit::{storage as supply_limit, SupplyLimit},
        ComplianceModule,
    },
    ComplianceHook,
};

#[contract]
pub struct SupplyLimitContract;

fn require_compliance_auth(e: &Env) {
    get_compliance_address(e).require_auth();
}

#[contractimpl]
impl SupplyLimitContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl SupplyLimit for SupplyLimitContract {
    #[only_admin]
    fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        supply_limit::configure_supply_limit(e, &token, limit);
    }

    #[only_admin]
    fn pre_set_supply(e: &Env, token: Address, supply: i128) {
        supply_limit::pre_set_supply(e, &token, supply);
    }

    fn get_supply_limit(e: &Env, token: Address) -> i128 {
        supply_limit::get_supply_limit(e, &token)
    }

    fn get_internal_supply(e: &Env, token: Address) -> i128 {
        supply_limit::get_internal_supply(e, &token)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        supply_limit::required_hooks(e)
    }

    fn verify_hook_wiring(e: &Env) {
        supply_limit::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for SupplyLimitContract {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(e: &Env, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        supply_limit::on_created(e, amount, &token);
    }

    fn on_destroyed(e: &Env, _from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
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

    #[only_admin]
    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
