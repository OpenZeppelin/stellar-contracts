#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use stellar_tokens::rwa::compliance::modules::{
    storage::{set_compliance_address, ComplianceModuleStorageKey},
    transfer_restrict::{
        storage::{is_user_allowed, remove_user_allowed, set_user_allowed},
        TransferRestrict, UserAllowed, UserDisallowed,
    },
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
}

#[contractimpl(contracttrait)]
impl TransferRestrict for TransferRestrictContract {
    fn allow_user(e: &Env, token: Address, user: Address) {
        require_module_admin_or_compliance_auth(e);
        set_user_allowed(e, &token, &user);
        UserAllowed { token, user }.publish(e);
    }

    fn disallow_user(e: &Env, token: Address, user: Address) {
        require_module_admin_or_compliance_auth(e);
        remove_user_allowed(e, &token, &user);
        UserDisallowed { token, user }.publish(e);
    }

    fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_module_admin_or_compliance_auth(e);
        for user in users.iter() {
            set_user_allowed(e, &token, &user);
            UserAllowed { token: token.clone(), user }.publish(e);
        }
    }

    fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_module_admin_or_compliance_auth(e);
        for user in users.iter() {
            remove_user_allowed(e, &token, &user);
            UserDisallowed { token: token.clone(), user }.publish(e);
        }
    }

    fn is_user_allowed(e: &Env, token: Address, user: Address) -> bool {
        is_user_allowed(e, &token, &user)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        get_admin(e).require_auth();
        set_compliance_address(e, &compliance);
    }
}
