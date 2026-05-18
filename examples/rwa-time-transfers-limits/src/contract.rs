use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use stellar_access::access_control;
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{get_compliance_address, module_name, set_compliance_address},
        time_transfers_limits::{storage as ttl, Limit, TimeTransfersLimits, TransferCounter},
        ComplianceModule,
    },
    ComplianceHook,
};

#[contract]
pub struct TimeTransfersLimitsContract;

fn require_compliance_auth(e: &Env) {
    get_compliance_address(e).require_auth();
}

#[contractimpl]
impl TimeTransfersLimitsContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl TimeTransfersLimits for TimeTransfersLimitsContract {
    #[only_admin]
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        ttl::configure_irs(e, &token, &irs);
    }

    #[only_admin]
    fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit) {
        ttl::set_time_transfer_limit(e, &token, &limit);
    }

    #[only_admin]
    fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>) {
        ttl::batch_set_time_transfer_limit(e, &token, &limits);
    }

    #[only_admin]
    fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64) {
        ttl::remove_time_transfer_limit(e, &token, limit_time);
    }

    #[only_admin]
    fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>) {
        ttl::batch_remove_time_transfer_limit(e, &token, &limit_times);
    }

    #[only_admin]
    fn pre_set_transfer_counter(
        e: &Env,
        token: Address,
        identity: Address,
        limit_time: u64,
        counter: TransferCounter,
    ) {
        ttl::pre_set_transfer_counter(e, &token, &identity, limit_time, &counter);
    }

    fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<Limit> {
        ttl::get_limits(e, &token)
    }

    fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        ttl::required_hooks(e)
    }

    fn verify_hook_wiring(e: &Env) {
        ttl::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for TimeTransfersLimitsContract {
    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_compliance_auth(e);
        ttl::on_transfer(e, &from, amount, &token);
    }

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) -> bool {
        ttl::can_transfer(e, &from, amount, &token)
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> String {
        module_name(e, "TimeTransfersLimitsModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
