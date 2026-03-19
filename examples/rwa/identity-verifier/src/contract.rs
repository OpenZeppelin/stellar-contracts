//! RWA Identity Verifier Example Contract.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_admin;
use stellar_tokens::rwa::identity_verifier::{storage as identity_verifier, IdentityVerifier};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct IdentityVerifierContract;

#[contractimpl]
impl IdentityVerifierContract {
    pub fn __constructor(
        e: &Env,
        admin: Address,
        manager: Address,
        identity_registry_storage: Address,
        claim_topics_and_issuers: Address,
    ) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);

        identity_verifier::set_identity_registry_storage(e, &identity_registry_storage);
        identity_verifier::set_claim_topics_and_issuers(e, &claim_topics_and_issuers);
    }

    pub fn identity_registry_storage(e: &Env) -> Address {
        identity_verifier::identity_registry_storage(e)
    }

    #[only_admin]
    pub fn set_identity_registry_storage(
        e: &Env,
        identity_registry_storage: Address,
        _operator: Address,
    ) {
        identity_verifier::set_identity_registry_storage(e, &identity_registry_storage);
    }
}

#[contractimpl(contracttrait)]
impl IdentityVerifier for IdentityVerifierContract {
    fn verify_identity(e: &Env, account: &Address) {
        identity_verifier::verify_identity(e, account);
    }

    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
        identity_verifier::recovery_target(e, old_account)
    }

    #[only_admin]
    fn set_claim_topics_and_issuers(
        e: &Env,
        claim_topics_and_issuers: Address,
        _operator: Address,
    ) {
        identity_verifier::set_claim_topics_and_issuers(e, &claim_topics_and_issuers);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for IdentityVerifierContract {}
