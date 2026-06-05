use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::rwa::compliance::modules::{
    storage::{self as compliance_storage},
    transfer_restrict::{storage as transfer_restrict, TransferRestrict},
    ComplianceModule,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct TransferRestrictContract;

#[contractimpl]
impl TransferRestrictContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for TransferRestrictContract {}

#[contractimpl(contracttrait)]
impl TransferRestrict for TransferRestrictContract {
    #[only_role(operator, "manager")]
    fn allow_user(e: &Env, token: Address, user: Address, operator: Address) {
        transfer_restrict::allow_user(e, &token, &user);
    }

    #[only_role(operator, "manager")]
    fn disallow_user(e: &Env, token: Address, user: Address, operator: Address) {
        transfer_restrict::disallow_user(e, &token, &user);
    }

    #[only_role(operator, "manager")]
    fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>, operator: Address) {
        transfer_restrict::batch_allow_users(e, &token, &users);
    }

    #[only_role(operator, "manager")]
    fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>, operator: Address) {
        transfer_restrict::batch_disallow_users(e, &token, &users);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for TransferRestrictContract {
    // No need to implement logic in these hooks for this module, as the
    // compliance check is only done in the can_transfer function.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    // No need to implement logic in these hooks for this module, as the
    // compliance check is only done in the can_transfer function.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    // No need to implement logic in these hooks for this module, as the
    // compliance check is only done in the can_transfer function.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        transfer_restrict::can_transfer(e, &from, &to, amount, &token)
    }

    // Mints are not restricted by this module.
    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "TransferRestrictModule")
    }

    #[only_admin]
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        compliance_storage::set_compliance_address(e, &token, &compliance);
    }
}
