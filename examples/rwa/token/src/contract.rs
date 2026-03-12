//! RWA Token Example Contract.

use soroban_sdk::{
    contract, contractimpl, symbol_short, Address, Env, MuxedAddress, String, Symbol, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_admin, only_role};
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    rwa::{RWAToken, RWA},
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct RWATokenContract;

#[contractimpl]
impl RWATokenContract {
    pub fn __constructor(
        e: &Env,
        name: String,
        symbol: String,
        admin: Address,
        manager: Address,
        compliance: Address,
        identity_verifier: Address,
    ) {
        Base::set_metadata(e, 7, name, symbol);

        access_control::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);

        RWA::set_compliance(e, &compliance);
        RWA::set_identity_verifier(e, &identity_verifier);
    }
}

#[contractimpl(contracttrait)]
impl Pausable for RWATokenContract {
    #[only_admin]
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    #[only_admin]
    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for RWATokenContract {
    type ContractType = RWA;
}

#[contractimpl(contracttrait)]
impl RWAToken for RWATokenContract {
    #[only_role(operator, "manager")]
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address) {
        RWA::forced_transfer(e, &from, &to, amount);
    }

    #[only_role(operator, "manager")]
    fn mint(e: &Env, to: Address, amount: i128, operator: Address) {
        RWA::mint(e, &to, amount);
    }

    #[only_role(operator, "manager")]
    fn burn(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::burn(e, &user_address, amount);
    }

    #[only_role(operator, "manager")]
    fn recover_balance(
        e: &Env,
        old_account: Address,
        new_account: Address,
        operator: Address,
    ) -> bool {
        RWA::recover_balance(e, &old_account, &new_account)
    }

    #[only_role(operator, "manager")]
    fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address) {
        RWA::set_address_frozen(e, &user_address, freeze);
    }

    #[only_role(operator, "manager")]
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::freeze_partial_tokens(e, &user_address, amount);
    }

    #[only_role(operator, "manager")]
    fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::unfreeze_partial_tokens(e, &user_address, amount);
    }

    #[only_role(operator, "manager")]
    fn set_compliance(e: &Env, compliance: Address, operator: Address) {
        RWA::set_compliance(e, &compliance);
    }

    #[only_role(operator, "manager")]
    fn set_identity_verifier(e: &Env, identity_verifier: Address, operator: Address) {
        RWA::set_identity_verifier(e, &identity_verifier);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for RWATokenContract {}
