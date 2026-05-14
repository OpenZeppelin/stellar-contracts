use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_tokens::rwa::compliance::modules::{
    max_balance::{storage as max_balance, MaxBalance},
    storage::{
        get_compliance_address, module_name, set_compliance_address, set_irs_address,
        ComplianceModuleStorageKey,
    },
    ComplianceModule,
};

#[contract]
pub struct MaxBalanceContract;

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
impl MaxBalanceContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for MaxBalanceContract {}

#[contractimpl(contracttrait)]
impl MaxBalance for MaxBalanceContract {
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    fn set_max_balance(e: &Env, token: Address, max: i128) {
        require_module_admin_or_compliance_auth(e);
        max_balance::set_max_balance(e, &token, max);
    }

    fn preset_id_balance(e: &Env, token: Address, identity: Address, balance: i128) {
        require_module_admin_or_compliance_auth(e);
        max_balance::preset_id_balance(e, &token, &identity, balance);
    }

    fn batch_preset_id_balances(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    ) {
        require_module_admin_or_compliance_auth(e);
        max_balance::batch_preset_id_balances(e, &token, &identities, &balances);
    }

    fn mark_preset_completed(e: &Env, token: Address) {
        require_module_admin_or_compliance_auth(e);
        max_balance::mark_preset_completed(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for MaxBalanceContract {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        get_compliance_address(e).require_auth();
        max_balance::on_transfer(e, &from, &to, amount, &token);
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        get_compliance_address(e).require_auth();
        max_balance::on_created(e, &to, amount, &token);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        get_compliance_address(e).require_auth();
        max_balance::on_destroyed(e, &from, amount, &token);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        max_balance::can_transfer(e, &from, &to, amount, &token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        max_balance::can_create(e, &to, amount, &token)
    }

    fn name(e: &Env) -> String {
        module_name(e, "MaxBalanceModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
