//! # RWA Token Contract
//!
//! A comprehensive Real World Asset token implementation demonstrating
//! the full T-REX standard with identity verification, compliance rules,
//! and administrative controls.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_contract_utils::pausable::{self, Pausable};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    rwa::{RWAToken, RWA},
};

/// Role for token administrators who can mint, burn, and manage the token
pub const ADMIN_ROLE: Symbol = symbol_short!("ADMIN");
/// Role for compliance officers who can freeze addresses and manage compliance
pub const COMPLIANCE_ROLE: Symbol = symbol_short!("COMPLNC");
/// Role for recovery agents who can perform wallet recovery operations
pub const RECOVERY_ROLE: Symbol = symbol_short!("RECOVERY");

#[contract]
pub struct RWATokenContract;

#[contractimpl]
impl RWATokenContract {
    /// Initializes the RWA token with metadata and roles
    pub fn __constructor(
        e: &Env,
        admin: Address,
        compliance_officer: Address,
        recovery_agent: Address,
        name: String,
        symbol: String,
        decimals: u32,
    ) {
        // Set token metadata
        Base::set_metadata(e, decimals, name, symbol);

        // Initialize access control with admin
        access_control::set_admin(e, &admin);

        // Grant roles
        access_control::grant_role_no_auth(e, &admin, &compliance_officer, &COMPLIANCE_ROLE);
        access_control::grant_role_no_auth(e, &admin, &recovery_agent, &RECOVERY_ROLE);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for RWATokenContract {
    type ContractType = RWA;
}

#[contractimpl]
impl Pausable for RWATokenContract {
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    #[only_role(operator, "ADMIN")]
    fn pause(e: &Env, operator: Address) {
        pausable::pause(e);
    }

    #[only_role(operator, "ADMIN")]
    fn unpause(e: &Env, operator: Address) {
        pausable::unpause(e);
    }
}

#[contractimpl]
impl RWAToken for RWATokenContract {
    #[only_role(operator, "ADMIN")]
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address) {
        RWA::forced_transfer(e, &from, &to, amount);
    }

    #[only_role(operator, "ADMIN")]
    fn mint(e: &Env, to: Address, amount: i128, operator: Address) {
        RWA::mint(e, &to, amount);
    }

    #[only_role(operator, "ADMIN")]
    fn burn(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::burn(e, &user_address, amount);
    }

    #[only_role(operator, "RECOVERY")]
    fn recovery_address(
        e: &Env,
        lost_wallet: Address,
        new_wallet: Address,
        investor_onchain_id: Address,
        operator: Address,
    ) -> bool {
        RWA::recovery_address(e, &lost_wallet, &new_wallet, &investor_onchain_id)
    }

    #[only_role(operator, "COMPLNC")]
    fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address) {
        RWA::set_address_frozen(e, &operator, &user_address, freeze);
    }

    #[only_role(operator, "COMPLNC")]
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address) {
        RWA::freeze_partial_tokens(e, &user_address, amount);
    }

    #[only_role(operator, "COMPLNC")]
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

    #[only_role(operator, "ADMIN")]
    fn set_compliance(e: &Env, compliance: Address, operator: Address) {
        RWA::set_compliance(e, &compliance);
    }

    #[only_role(operator, "ADMIN")]
    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address) {
        RWA::set_claim_topics_and_issuers(e, &claim_topics_and_issuers);
    }

    #[only_role(operator, "ADMIN")]
    fn set_identity_registry_storage(
        e: &Env,
        identity_registry_storage: Address,
        operator: Address,
    ) {
        RWA::set_identity_registry_storage(e, &identity_registry_storage);
    }

    fn compliance(e: &Env) -> Address {
        RWA::compliance(e)
    }

    fn claim_topics_and_issuers(e: &Env) -> Address {
        RWA::claim_topics_and_issuers(e)
    }

    fn identity_registry_storage(e: &Env) -> Address {
        RWA::identity_registry_storage(e)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for RWATokenContract {}
