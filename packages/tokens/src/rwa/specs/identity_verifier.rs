use soroban_sdk::{contract, contractimpl, Address, Env};

use crate::rwa::identity_verifier::{storage, IdentityVerifier};

pub struct IdentityVerifierContract;

impl IdentityVerifier for IdentityVerifierContract {
    fn verify_identity(e: &Env, account: &Address) {
        storage::verify_identity(e, account);
    }

    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
        storage::recovery_target(e, old_account)
    }

    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address) {
        operator.require_auth(); // check if this is needed? Based on docs, it might be.
        storage::set_claim_topics_and_issuers(e, &claim_topics_and_issuers);
    }

    fn claim_topics_and_issuers(e: &Env) -> Address {
        storage::claim_topics_and_issuers(e)
    }

}