use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        initial_lockup_period::{storage as lockup, LockedTokens},
        storage::{
            get_compliance_address, module_name, set_compliance_address, ComplianceModuleStorageKey,
        },
        ComplianceModule,
    },
    ComplianceHook,
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct InitialLockupPeriodContract;

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
impl InitialLockupPeriodContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }

    pub fn set_lockup_period(e: &Env, token: Address, lockup_seconds: u64) {
        require_module_admin_or_compliance_auth(e);
        lockup::configure_lockup_period(e, &token, lockup_seconds);
    }

    pub fn pre_set_lockup_state(
        e: &Env,
        token: Address,
        wallet: Address,
        balance: i128,
        locks: Vec<LockedTokens>,
    ) {
        require_module_admin_or_compliance_auth(e);
        lockup::pre_set_lockup_state(e, &token, &wallet, balance, &locks);
    }

    pub fn get_lockup_period(e: &Env, token: Address) -> u64 {
        lockup::get_lockup_period(e, &token)
    }

    pub fn get_total_locked(e: &Env, token: Address, wallet: Address) -> i128 {
        lockup::get_total_locked(e, &token, &wallet)
    }

    pub fn get_locked_tokens(e: &Env, token: Address, wallet: Address) -> Vec<LockedTokens> {
        lockup::get_locks(e, &token, &wallet)
    }

    pub fn get_internal_balance(e: &Env, token: Address, wallet: Address) -> i128 {
        lockup::get_internal_balance(e, &token, &wallet)
    }

    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        lockup::required_hooks(e)
    }

    pub fn verify_hook_wiring(e: &Env) {
        lockup::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for InitialLockupPeriodContract {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        lockup::on_transfer(e, &from, &to, amount, &token);
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        lockup::on_created(e, &to, amount, &token);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
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

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
