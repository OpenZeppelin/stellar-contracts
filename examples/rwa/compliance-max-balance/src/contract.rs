use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::{
    modules::{
        max_balance::{storage as max_balance, MaxBalance},
        storage::{self as compliance_storage, set_irs_address},
        ComplianceModule,
    },
    AccountSnapshot, TransferKind,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct MaxBalanceContract;

#[contractimpl]
impl MaxBalanceContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for MaxBalanceContract {}

#[contractimpl(contracttrait)]
impl MaxBalance for MaxBalanceContract {
    #[only_role(operator, "manager")]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address, operator: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_role(operator, "manager")]
    fn set_max_balance(e: &Env, token: Address, max: i128, operator: Address) {
        max_balance::set_max_balance(e, &token, max);
    }

    #[only_role(operator, "manager")]
    fn preset_id_balance(
        e: &Env,
        token: Address,
        identity: Address,
        balance: i128,
        operator: Address,
    ) {
        max_balance::preset_id_balance(e, &token, &identity, balance);
    }

    #[only_role(operator, "manager")]
    fn batch_preset_id_balances(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
        operator: Address,
    ) {
        max_balance::batch_preset_id_balances(e, &token, &identities, &balances);
    }

    #[only_role(operator, "manager")]
    fn mark_preset_completed(e: &Env, token: Address, operator: Address) {
        max_balance::mark_preset_completed(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for MaxBalanceContract {
    // Enforces the cap: panics with `MaxBalanceExceeded` when the recipient
    // identity's aggregate balance would exceed the configured maximum
    // (forced transfers bypass the check but still update the books).
    fn on_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        kind: TransferKind,
        token: Address,
    ) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_transfer(e, &from.address, &to.address, amount, &kind, &token);
    }

    // Enforces the cap: panics with `MaxBalanceExceeded` when the mint would
    // push the recipient identity's aggregate balance past the maximum.
    fn on_created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_created(e, &to.address, amount, &token);
    }

    fn on_destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_destroyed(e, &from.address, amount, &token);
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "MaxBalanceModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
