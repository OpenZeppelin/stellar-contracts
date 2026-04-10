use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::modules::{
    storage::{
        get_compliance_address, module_name, set_compliance_address, ComplianceModuleStorageKey,
    },
    transfer_restrict::storage as transfer_restrict,
    ComplianceModule,
};

#[contracttype]
enum DataKey {
    Admin,
}

#[contract]
pub struct TransferRestrictContract;

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
impl TransferRestrictContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
    }

    pub fn allow_user(e: &Env, token: Address, user: Address) {
        require_module_admin_or_compliance_auth(e);
        transfer_restrict::allow_user(e, &token, &user);
    }

    pub fn disallow_user(e: &Env, token: Address, user: Address) {
        require_module_admin_or_compliance_auth(e);
        transfer_restrict::disallow_user(e, &token, &user);
    }

    pub fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_module_admin_or_compliance_auth(e);
        transfer_restrict::batch_allow_users(e, &token, &users);
    }

    pub fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_module_admin_or_compliance_auth(e);
        transfer_restrict::batch_disallow_users(e, &token, &users);
    }

    pub fn is_user_allowed(e: &Env, token: Address, user: Address) -> bool {
        transfer_restrict::is_user_allowed(e, &token, &user)
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for TransferRestrictContract {
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(e: &Env, from: Address, to: Address, _amount: i128, token: Address) -> bool {
        transfer_restrict::can_transfer(e, &from, &to, &token)
    }

    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> String {
        module_name(e, "TransferRestrictModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
