#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use stellar_tokens::rwa::identity_verifier::IdentityVerifier;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    IRS,
    ClaimTopicsAndIssuers,
}

#[soroban_sdk::contractclient(name = "IRSClient")]
#[allow(dead_code)]
trait IRSView {
    fn stored_identity(e: &Env, account: Address) -> Address;
    fn get_recovered_to(e: &Env, old: Address) -> Option<Address>;
}

#[contract]
pub struct SimpleIdentityVerifier;

#[contractimpl]
impl SimpleIdentityVerifier {
    pub fn __constructor(e: &Env, irs: Address) {
        e.storage().instance().set(&DataKey::IRS, &irs);
    }
}

#[contractimpl]
impl IdentityVerifier for SimpleIdentityVerifier {
    fn verify_identity(e: &Env, account: &Address) {
        let irs: Address = e.storage().instance().get(&DataKey::IRS).expect("IRS not set");
        let client = IRSClient::new(e, &irs);
        client.stored_identity(account);
    }

    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
        let irs: Address = e.storage().instance().get(&DataKey::IRS).expect("IRS not set");
        let client = IRSClient::new(e, &irs);
        client.get_recovered_to(old_account)
    }

    fn set_claim_topics_and_issuers(
        e: &Env,
        claim_topics_and_issuers: Address,
        _operator: Address,
    ) {
        e.storage().instance().set(&DataKey::ClaimTopicsAndIssuers, &claim_topics_and_issuers);
    }

    fn claim_topics_and_issuers(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::ClaimTopicsAndIssuers)
            .expect("ClaimTopicsAndIssuers not set")
    }
}
