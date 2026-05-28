use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::modules::{
    max_balance::{storage as max_balance, MaxBalance},
    storage::{self as compliance_storage, set_irs_address},
    ComplianceModule,
};

#[contract]
pub struct MaxBalanceContract;

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
    #[only_admin]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_admin]
    fn set_max_balance(e: &Env, token: Address, max: i128) {
        max_balance::set_max_balance(e, &token, max);
    }

    #[only_admin]
    fn preset_id_balance(e: &Env, token: Address, identity: Address, balance: i128) {
        max_balance::preset_id_balance(e, &token, &identity, balance);
    }

    #[only_admin]
    fn batch_preset_id_balances(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    ) {
        max_balance::batch_preset_id_balances(e, &token, &identities, &balances);
    }

    #[only_admin]
    fn mark_preset_completed(e: &Env, token: Address) {
        max_balance::mark_preset_completed(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for MaxBalanceContract {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_transfer(e, &from, &to, amount, &token);
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_created(e, &to, amount, &token);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_destroyed(e, &from, amount, &token);
    }

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        max_balance::can_transfer(e, &from, &to, amount, &token)
    }

    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        max_balance::can_create(e, &to, amount, &token)
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "MaxBalanceModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
