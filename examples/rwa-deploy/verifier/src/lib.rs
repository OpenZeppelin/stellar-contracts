#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env, Symbol, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::{
    emit_claim_topics_and_issuers_set, identity_registry_storage::IdentityRegistryStorageClient,
    identity_verifier::IdentityVerifier, RWAError,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Irs,
    ClaimTopicsAndIssuers,
}

#[contract]
pub struct SimpleIdentityVerifier;

fn identity_registry_storage(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&DataKey::Irs)
        .unwrap_or_else(|| panic_with_error!(e, RWAError::IdentityRegistryStorageNotSet))
}

#[contractimpl]
impl SimpleIdentityVerifier {
    pub fn __constructor(e: &Env, admin: Address, irs: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &symbol_short!("admin"), &admin);
        e.storage().instance().set(&DataKey::Irs, &irs);
    }
}

#[contractimpl]
impl IdentityVerifier for SimpleIdentityVerifier {
    fn verify_identity(e: &Env, account: &Address) {
        let irs = identity_registry_storage(e);
        let client = IdentityRegistryStorageClient::new(e, &irs);
        if client.try_stored_identity(account).is_err() {
            panic_with_error!(e, RWAError::IdentityVerificationFailed);
        }
    }

    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
        let irs = identity_registry_storage(e);
        let client = IdentityRegistryStorageClient::new(e, &irs);
        client.get_recovered_to(old_account)
    }

    #[only_role(operator, "admin")]
    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address) {
        e.storage().instance().set(&DataKey::ClaimTopicsAndIssuers, &claim_topics_and_issuers);
        emit_claim_topics_and_issuers_set(e, &claim_topics_and_issuers);
    }

    fn claim_topics_and_issuers(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::ClaimTopicsAndIssuers)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::ClaimTopicsAndIssuersNotSet))
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for SimpleIdentityVerifier {}
