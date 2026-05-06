use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        storage::{
            get_compliance_address, module_name, set_compliance_address, ComplianceModuleStorageKey,
        },
        time_transfers_limits::{storage as ttl, Limit, TransferCounter},
        ComplianceModule,
    },
    ComplianceHook,
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct TimeTransfersLimitsContract;

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
impl TimeTransfersLimitsContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }

    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        ttl::configure_irs(e, &token, &irs);
    }

    pub fn set_time_transfer_limit(e: &Env, token: Address, limit: Limit) {
        require_module_admin_or_compliance_auth(e);
        ttl::set_time_transfer_limit(e, &token, &limit);
    }

    pub fn batch_set_time_transfer_limit(e: &Env, token: Address, limits: Vec<Limit>) {
        require_module_admin_or_compliance_auth(e);
        ttl::batch_set_time_transfer_limit(e, &token, &limits);
    }

    pub fn remove_time_transfer_limit(e: &Env, token: Address, limit_time: u64) {
        require_module_admin_or_compliance_auth(e);
        ttl::remove_time_transfer_limit(e, &token, limit_time);
    }

    pub fn batch_remove_time_transfer_limit(e: &Env, token: Address, limit_times: Vec<u64>) {
        require_module_admin_or_compliance_auth(e);
        ttl::batch_remove_time_transfer_limit(e, &token, &limit_times);
    }

    pub fn pre_set_transfer_counter(
        e: &Env,
        token: Address,
        identity: Address,
        limit_time: u64,
        counter: TransferCounter,
    ) {
        require_module_admin_or_compliance_auth(e);
        ttl::pre_set_transfer_counter(e, &token, &identity, limit_time, &counter);
    }

    pub fn get_time_transfer_limits(e: &Env, token: Address) -> Vec<Limit> {
        ttl::get_limits(e, &token)
    }

    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        ttl::required_hooks(e)
    }

    pub fn verify_hook_wiring(e: &Env) {
        ttl::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for TimeTransfersLimitsContract {
    fn on_transfer(e: &Env, from: Address, _to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
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

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
