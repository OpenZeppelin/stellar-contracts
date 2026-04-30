use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::{
    modules::{
        max_balance::storage as max_balance,
        storage::{
            get_compliance_address, module_name, set_compliance_address, set_irs_address,
            ComplianceModuleStorageKey,
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
pub struct MaxBalanceContract;

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
impl MaxBalanceContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }

    pub fn set_identity_registry_storage(e: &Env, token: Address, irs: Address) {
        require_module_admin_or_compliance_auth(e);
        set_irs_address(e, &token, &irs);
    }

    pub fn set_max_balance(e: &Env, token: Address, max: i128) {
        require_module_admin_or_compliance_auth(e);
        max_balance::configure_max_balance(e, &token, max);
    }

    pub fn pre_set_identity_balance(e: &Env, token: Address, identity: Address, balance: i128) {
        require_module_admin_or_compliance_auth(e);
        max_balance::pre_set_identity_balance(e, &token, &identity, balance);
    }

    pub fn batch_pre_set_identity_balances(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    ) {
        require_module_admin_or_compliance_auth(e);
        max_balance::batch_pre_set_identity_balances(e, &token, &identities, &balances);
    }

    pub fn get_max_balance(e: &Env, token: Address) -> i128 {
        max_balance::get_max_balance(e, &token)
    }

    pub fn get_investor_balance(e: &Env, token: Address, identity: Address) -> i128 {
        max_balance::get_id_balance(e, &token, &identity)
    }

    pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
        max_balance::required_hooks(e)
    }

    pub fn verify_hook_wiring(e: &Env) {
        max_balance::verify_hook_wiring(e);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for MaxBalanceContract {
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        max_balance::on_transfer(e, &from, &to, amount, &token);
    }

    fn on_created(e: &Env, to: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
        max_balance::on_created(e, &to, amount, &token);
    }

    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        require_module_admin_or_compliance_auth(e);
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
