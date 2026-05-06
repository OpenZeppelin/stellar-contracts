//! RWA Token contract with full compliance and identity verification wiring.
//!
//! Implements `FungibleToken`, `RWAToken`, `Pausable`, and `AccessControl`.
//! The constructor wires compliance and identity-verifier addresses so that
//! all transfer/mint/burn operations are subject to modular compliance checks.

use soroban_sdk::{
    contract, contractimpl, symbol_short, Address, Env, MuxedAddress, String, Symbol, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::only_role;
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    rwa::{storage::RWAStorageKey, RWAToken, RWA},
};

#[contract]
pub struct RWATokenContract;

#[contractimpl]
impl RWATokenContract {
    pub fn __constructor(
        e: &Env,
        name: String,
        symbol: String,
        admin: Address,
        compliance: Address,
        identity_verifier: Address,
    ) {
        Base::set_metadata(e, 18, name, symbol);
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &symbol_short!("admin"), &admin);
        RWA::set_compliance(e, &compliance);
        RWA::set_identity_verifier(e, &identity_verifier);
        e.storage().instance().set(&RWAStorageKey::Version, &String::from_str(e, "1.0.0"));
        RWA::set_onchain_id(e, &e.current_contract_address());
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for RWATokenContract {
    type ContractType = RWA;
}

#[contractimpl]
impl RWAToken for RWATokenContract {
    #[only_role(operator, "admin")]
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address) {
        RWA::forced_transfer(e, &from, &to, amount);
    }

    #[only_role(operator, "admin")]
    fn mint(e: &Env, to: Address, amount: i128, operator: Address) {
        RWA::mint(e, &to, amount);
    }

    #[only_role(operator, "admin")]
    fn burn(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::burn(e, &user_address, amount);
    }

    #[only_role(operator, "admin")]
    fn recover_balance(
        e: &Env,
        old_account: Address,
        new_account: Address,
        operator: Address,
    ) -> bool {
        RWA::recover_balance(e, &old_account, &new_account)
    }

    #[only_role(operator, "admin")]
    fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address) {
        RWA::set_address_frozen(e, &user_address, freeze);
    }

    #[only_role(operator, "admin")]
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::freeze_partial_tokens(e, &user_address, amount);
    }

    #[only_role(operator, "admin")]
    fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::unfreeze_partial_tokens(e, &user_address, amount);
    }

    fn is_frozen(e: &Env, user_address: Address) -> bool {
        RWA::is_frozen(e, &user_address)
    }

    fn get_frozen_tokens(e: &Env, user_address: Address) -> i128 {
        RWA::get_frozen_tokens(e, &user_address)
    }

    fn version(e: &Env) -> String {
        RWA::version(e)
    }

    fn onchain_id(e: &Env) -> Address {
        RWA::onchain_id(e)
    }

    #[only_role(operator, "admin")]
    fn set_compliance(e: &Env, compliance: Address, operator: Address) {
        RWA::set_compliance(e, &compliance);
    }

    fn compliance(e: &Env) -> Address {
        RWA::compliance(e)
    }

    #[only_role(operator, "admin")]
    fn set_identity_verifier(e: &Env, identity_verifier: Address, operator: Address) {
        RWA::set_identity_verifier(e, &identity_verifier);
    }

    fn identity_verifier(e: &Env) -> Address {
        RWA::identity_verifier(e)
    }
}

#[contractimpl]
impl Pausable for RWATokenContract {
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    #[only_role(caller, "admin")]
    fn pause(e: &Env, caller: Address) {
        pausable::pause(e);
    }

    #[only_role(caller, "admin")]
    fn unpause(e: &Env, caller: Address) {
        pausable::unpause(e);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for RWATokenContract {}
