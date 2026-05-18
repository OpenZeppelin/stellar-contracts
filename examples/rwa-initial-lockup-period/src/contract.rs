use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use stellar_access::access_control;
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::{
    modules::{
        initial_lockup_period::{storage as lockup, InitialLockupPeriod, LockedTokens},
        storage::{get_compliance_address, module_name, set_compliance_address},
        ComplianceModule,
    },
    ComplianceHook,
};

#[contract]
pub struct InitialLockupPeriodContract;

fn require_compliance_auth(e: &Env) {
    get_compliance_address(e).require_auth();
}

#[contractimpl]
impl InitialLockupPeriodContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl InitialLockupPeriod for InitialLockupPeriodContract {
    #[only_admin]
    fn set_lockup_period(e: &Env, token: Address, lockup_seconds: u64) {
        lockup::configure_lockup_period(e, &token, lockup_seconds);
    }

    #[only_admin]
    fn pre_set_lockup_state(
        e: &Env,
        token: Address,
        wallet: Address,
        balance: i128,
        locks: Vec<LockedTokens>,
    ) {
        lockup::pre_set_lockup_state(e, &token, &wallet, balance, &locks);
    }

    fn get_lockup_period(e: &Env, token: Address) -> u64 {
        lockup::get_lockup_period(e, &token)
    }

    fn get_total_locked(e: &Env, token: Address, wallet: Address) -> i128 {
        lockup::get_total_locked(e, &token, &wallet)
    }

    fn get_locked_tokens(e: &Env, token: Address, wallet: Address) -> Vec<LockedTokens> {
        lockup::get_locks(e, &token, &wallet)
    }

    fn get_internal_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        lockup::get_internal_balance(e, &token, &wallet)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        lockup::required_hooks(e)
    }

    fn verify_hook_wiring(e: &Env) {
        lockup::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for InitialLockupPeriodContract {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        lockup::on_transfer(e, &from, &to, amount, &token);
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        lockup::on_created(e, &to, amount, &token);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        lockup::on_destroyed(e, &from, amount, &token);
    }

    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        lockup::can_transfer(e, &from, amount, &token)
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> String {
        module_name(e, "InitialLockupPeriodModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
