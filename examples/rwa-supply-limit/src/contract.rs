use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_tokens::rwa::compliance::modules::{
    storage::{
        get_compliance_address, module_name, set_compliance_address, ComplianceModuleStorageKey,
    },
    supply_limit::{storage as supply_limit, SupplyLimit},
    ComplianceModule,
};

#[contract]
pub struct SupplyLimitContract;

fn get_admin(e: &Env) -> Address {
    access_control::get_admin(e).expect("admin is set in the constructor")
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
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for SupplyLimitContract {}

#[contractimpl(contracttrait)]
impl SupplyLimit for SupplyLimitContract {
    fn set_supply_limit(e: &Env, token: Address, limit: i128) {
        require_module_admin_or_compliance_auth(e);
        supply_limit::set_supply_limit(e, &token, limit);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for SupplyLimitContract {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        get_compliance_address(e).require_auth();
        supply_limit::on_created(e, &to, amount, &token);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        get_compliance_address(e).require_auth();
        supply_limit::on_destroyed(e, &from, amount, &token);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        supply_limit::can_transfer(e, &from, &to, amount, &token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        supply_limit::can_create(e, &to, amount, &token)
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
