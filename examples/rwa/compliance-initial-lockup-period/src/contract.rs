use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::{
    modules::{
        initial_lockup_period::{
            storage as initial_lockup_period, InitialLockupPeriod, LockedDetails, LockedTokens,
        },
        storage::{self as compliance_storage},
        ComplianceModule,
    },
    AccountSnapshot, TransferKind,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct InitialLockupPeriodContract;

#[contractimpl]
impl InitialLockupPeriodContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for InitialLockupPeriodContract {}

#[contractimpl(contracttrait)]
impl InitialLockupPeriod for InitialLockupPeriodContract {
    #[only_role(operator, "manager")]
    fn set_lockup_period(e: &Env, token: Address, period: u32, operator: Address) {
        initial_lockup_period::set_lockup_period(e, &token, period);
    }

    #[only_role(operator, "manager")]
    fn preset_locks(
        e: &Env,
        token: Address,
        wallet: Address,
        locks: Vec<LockedTokens>,
        operator: Address,
    ) {
        initial_lockup_period::preset_locks(e, &token, &wallet, &locks);
    }

    #[only_role(operator, "manager")]
    fn mark_preset_completed(e: &Env, token: Address, operator: Address) {
        initial_lockup_period::mark_preset_completed(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for InitialLockupPeriodContract {
    // Enforces the lockup: panics with `InsufficientUnlockedBalance` when
    // the transfer exceeds the sender's unlocked holdings (forced transfers
    // consume locks instead of being rejected).
    fn on_transfer(
        e: &Env,
        from: AccountSnapshot,
        _to: AccountSnapshot,
        amount: i128,
        kind: TransferKind,
        token: Address,
    ) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        initial_lockup_period::on_transfer(e, &from.address, from.balance, amount, &kind, &token);
    }

    fn on_created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        initial_lockup_period::on_created(e, &to.address, amount, &token);
    }

    fn on_destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        initial_lockup_period::on_destroyed(e, &from.address, from.balance, amount, &token);
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "InitialLockupPeriodModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
