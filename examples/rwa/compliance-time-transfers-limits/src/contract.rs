use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{self as compliance_storage, set_irs_address},
        time_transfers_limits::{
            storage as time_transfers_limits, TimeTransfersLimits, TransferCounter, TransferLimit,
        },
        ComplianceModule,
    },
    AccountSnapshot, TransferKind,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct TimeTransfersLimitsContract;

#[contractimpl]
impl TimeTransfersLimitsContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for TimeTransfersLimitsContract {}

#[contractimpl(contracttrait)]
impl TimeTransfersLimits for TimeTransfersLimitsContract {
    #[only_role(operator, "manager")]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address, operator: Address) {
        set_irs_address(e, &token, &irs);
    }

    #[only_role(operator, "manager")]
    fn set_time_transfer_limit(e: &Env, token: Address, limit: TransferLimit, operator: Address) {
        time_transfers_limits::set_time_transfer_limit(e, &token, &limit);
    }

    #[only_role(operator, "manager")]
    fn batch_set_time_transfer_limit(
        e: &Env,
        token: Address,
        limits: Vec<TransferLimit>,
        operator: Address,
    ) {
        time_transfers_limits::batch_set_time_transfer_limit(e, &token, &limits);
    }

    #[only_role(operator, "manager")]
    fn remove_time_transfer_limit(e: &Env, token: Address, limit_duration: u32, operator: Address) {
        time_transfers_limits::remove_time_transfer_limit(e, &token, limit_duration);
    }

    #[only_role(operator, "manager")]
    fn batch_remove_time_transfer_limit(
        e: &Env,
        token: Address,
        limit_durations: Vec<u32>,
        operator: Address,
    ) {
        time_transfers_limits::batch_remove_time_transfer_limit(e, &token, &limit_durations);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for TimeTransfersLimitsContract {
    // Enforces the windows: panics with `TransferLimitExceeded` when the
    // transfer would push the sender identity's volume past a configured
    // time-window cap (forced transfers are skipped entirely).
    fn on_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        kind: TransferKind,
        token: Address,
    ) {
        compliance_storage::get_compliance_address(e, &token).require_auth();
        time_transfers_limits::on_transfer(e, &from.address, &to.address, amount, &kind, &token);
    }

    // Mints are not counted against the time-window limits; no bookkeeping
    // needed.
    fn on_created(_e: &Env, _to: AccountSnapshot, _amount: i128, _token: Address) {}

    // Burns are not counted against the time-window limits; no bookkeeping
    // needed.
    fn on_destroyed(_e: &Env, _from: AccountSnapshot, _amount: i128, _token: Address) {}

    fn name(e: &Env) -> String {
        String::from_str(e, "TimeTransfersLimitsModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
