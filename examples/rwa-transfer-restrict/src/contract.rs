use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use stellar_access::access_control;
use stellar_macros::only_admin;
use stellar_tokens::rwa::compliance::modules::{
    storage::{get_compliance_address, module_name, set_compliance_address},
    transfer_restrict::{storage as transfer_restrict, TransferRestrict},
    ComplianceModule,
};

#[contract]
pub struct TransferRestrictContract;

#[contractimpl]
impl TransferRestrictContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }
}

#[contractimpl(contracttrait)]
impl TransferRestrict for TransferRestrictContract {
    #[only_admin]
    fn allow_user(e: &Env, token: Address, user: Address) {
        transfer_restrict::allow_user(e, &token, &user);
    }

    #[only_admin]
    fn disallow_user(e: &Env, token: Address, user: Address) {
        transfer_restrict::disallow_user(e, &token, &user);
    }

    #[only_admin]
    fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>) {
        transfer_restrict::batch_allow_users(e, &token, &users);
    }

    #[only_admin]
    fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>) {
        transfer_restrict::batch_disallow_users(e, &token, &users);
    }

    fn is_user_allowed(e: &Env, token: Address, user: Address) -> bool {
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

    #[only_admin]
    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
