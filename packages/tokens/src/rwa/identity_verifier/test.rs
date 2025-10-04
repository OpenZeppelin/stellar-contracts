extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, map, symbol_short, testutils::Address as _, vec, Address,
    Bytes, BytesN, Env, Map, String, Vec,
};

use crate::rwa::{
    claim_issuer::ClaimIssuer,
    identity_claims::{generate_claim_id, Claim, IdentityClaims},
    identity_verifier::storage::{
        claim_topics_and_issuers, identity_registry_storage, set_claim_topics_and_issuers,
        set_identity_registry_storage, validate_claim, verify_identity,
    },
};

#[contract]
struct MockContract;

// Mock contracts for identity verification
#[contract]
pub struct MockIdentityRegistryStorage;

#[contractimpl]
impl MockIdentityRegistryStorage {
    pub fn stored_identity(e: &Env, _account: Address) -> Address {
        e.storage().persistent().get(&symbol_short!("stored_id")).unwrap()
    }
}

#[contract]
pub struct MockClaimTopicsAndIssuers;

#[contractimpl]
impl MockClaimTopicsAndIssuers {
    pub fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
        let issuers = e.storage().persistent().get(&symbol_short!("issuers")).unwrap();
        map![e, (1u32, issuers)]
    }
}

#[contract]
pub struct MockIdentityClaims;

#[contracttype]
pub enum IdentityClaimsMockStorageKey {
    Claim(BytesN<32>),
}

#[contractimpl]
impl IdentityClaims for MockIdentityClaims {
    fn get_claim(e: &Env, claim_id: soroban_sdk::BytesN<32>) -> Claim {
        e.storage().persistent().get(&IdentityClaimsMockStorageKey::Claim(claim_id)).unwrap()
    }

    fn add_claim(
        _e: &Env,
        _topic: u32,
        _scheme: u32,
        _issuer: Address,
        _signature: Bytes,
        _data: Bytes,
        _uri: String,
    ) -> BytesN<32> {
        unimplemented!()
    }

    fn get_claim_ids_by_topic(e: &Env, _topic: u32) -> Vec<BytesN<32>> {
        e.storage().persistent().get(&symbol_short!("claim_ids")).unwrap()
    }
}

#[contract]
pub struct MockClaimIssuer;

#[contractimpl]
impl ClaimIssuer for MockClaimIssuer {
    fn is_claim_valid(
        e: &Env,
        _identity: Address,
        _claim_topic: u32,
        _sig_data: Bytes,
        _claim_data: Bytes,
    ) -> bool {
        e.storage().persistent().get(&symbol_short!("claim_ok")).unwrap_or(false)
    }
}

// Helper functions
fn construct_claim(e: &Env, issuer: &Address, topic: u32) -> Claim {
    Claim {
        topic,
        scheme: 1u32,
        issuer: issuer.clone(),
        signature: Bytes::from_array(e, &[1, 2, 3, 4]),
        data: Bytes::from_array(e, &[5, 6, 7, 8]),
        uri: soroban_sdk::String::from_str(e, "https://example.com"),
    }
}

fn setup_verification_contracts(e: &Env) -> (Address, Address, Address, Address) {
    let identity_claims = e.register(MockIdentityClaims, ());
    let issuer = e.register(MockClaimIssuer, ());
    let irs = e.register(MockIdentityRegistryStorage, ());
    let cti = e.register(MockClaimTopicsAndIssuers, ());

    e.as_contract(&irs, || {
        e.storage().persistent().set(&symbol_short!("stored_id"), &identity_claims);
    });
    e.as_contract(&identity_claims, || {
        let claim = construct_claim(e, &issuer, 1);
        let claim_id = generate_claim_id(e, &issuer, 1);
        e.storage()
            .persistent()
            .set(&IdentityClaimsMockStorageKey::Claim(claim_id.clone()), &claim);
        e.storage().persistent().set(&symbol_short!("claim_ids"), &Vec::from_array(e, [claim_id]));
    });
    e.as_contract(&cti, || {
        e.storage().persistent().set(&symbol_short!("issuers"), &vec![&e, issuer.clone()]);
    });

    (identity_claims, issuer, irs, cti)
}

#[test]
fn set_and_get_claim_topics_and_issuers() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let claim_topics_and_issuers_contract = Address::generate(&e);
        set_claim_topics_and_issuers(&e, &claim_topics_and_issuers_contract);
        assert_eq!(claim_topics_and_issuers(&e), claim_topics_and_issuers_contract);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #310)")]
fn get_unset_claim_topics_and_issuers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        claim_topics_and_issuers(&e);
    });
}

#[test]
fn set_and_get_identity_registry_storage() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let identity_registry_contract = Address::generate(&e);
        set_identity_registry_storage(&e, &identity_registry_contract);
        assert_eq!(identity_registry_storage(&e), identity_registry_contract);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #311)")]
fn get_unset_identity_registry_storage_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        identity_registry_storage(&e);
    });
}

// Tests for validate_claim function
#[test]
fn validate_claim_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let issuer = e.register(MockClaimIssuer, ());
    let investor_onchain_id = Address::generate(&e);

    e.as_contract(&address, || {
        let claim = construct_claim(&e, &issuer, 1);

        // Mock claim issuer to return valid claim
        e.as_contract(&issuer, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &true);
        });

        let result = validate_claim(&e, &claim, 1u32, &issuer, &investor_onchain_id);
        assert!(result);
    });
}

#[test]
fn validate_claim_wrong_topic() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let issuer = e.register(MockClaimIssuer, ());
    let investor_onchain_id = Address::generate(&e);

    e.as_contract(&address, || {
        let claim = construct_claim(&e, &issuer, 1);

        // Different topic should return false
        let result = validate_claim(&e, &claim, 2u32, &issuer, &investor_onchain_id);
        assert!(!result);
    });
}

#[test]
fn validate_claim_wrong_issuer() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let issuer = e.register(MockClaimIssuer, ());
    let wrong_issuer = e.register(MockClaimIssuer, ());
    let investor_onchain_id = Address::generate(&e);

    e.as_contract(&address, || {
        let claim = construct_claim(&e, &issuer, 1);

        // Different issuer should return false
        let result = validate_claim(&e, &claim, 1u32, &wrong_issuer, &investor_onchain_id);
        assert!(!result);
    });
}

#[test]
fn validate_claim_invalid_signature() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let issuer = e.register(MockClaimIssuer, ());
    let investor_onchain_id = Address::generate(&e);

    e.as_contract(&address, || {
        let claim = construct_claim(&e, &issuer, 1);

        // Mock claim issuer to return invalid claim
        e.as_contract(&issuer, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &false);
        });

        let result = validate_claim(&e, &claim, 1u32, &issuer, &investor_onchain_id);
        assert!(!result);
    });
}

// Tests for verify_identity function
#[test]
fn verify_identity_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let user_address = Address::generate(&e);

    e.as_contract(&address, || {
        let (_identity_claims, issuer, irs, cti) = setup_verification_contracts(&e);

        // Set up the storage references
        set_identity_registry_storage(&e, &irs);
        set_claim_topics_and_issuers(&e, &cti);

        // Mock claim issuer to return valid claim
        e.as_contract(&issuer, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &true);
        });

        // Should not panic
        verify_identity(&e, &user_address);
    });
}

#[test]
fn verify_identity_success_with_multiple_issuers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let user_address = Address::generate(&e);

    e.as_contract(&address, || {
        let (identity_claims, issuer1, irs, cti) = setup_verification_contracts(&e);
        let issuer2 = e.register(MockClaimIssuer, ());

        // First issuer returns invalid claim
        e.as_contract(&issuer1, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &false);
        });

        // Second issuer returns valid claim
        e.as_contract(&issuer2, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &true);
        });

        // Update claim topics and issuers to include both
        e.as_contract(&cti, || {
            e.storage()
                .persistent()
                .set(&symbol_short!("issuers"), &vec![&e, issuer1.clone(), issuer2.clone()]);
        });

        e.as_contract(&identity_claims, || {
            let claim1 = construct_claim(&e, &issuer2, 1);
            let claim2 = construct_claim(&e, &issuer2, 1);
            let id1 = generate_claim_id(&e, &issuer1, 1);
            let id2 = generate_claim_id(&e, &issuer2, 1);
            e.storage()
                .persistent()
                .set(&IdentityClaimsMockStorageKey::Claim(id1.clone()), &claim1);
            e.storage()
                .persistent()
                .set(&IdentityClaimsMockStorageKey::Claim(id2.clone()), &claim2);
            e.storage()
                .persistent()
                .set(&symbol_short!("claim_ids"), &Vec::from_array(&e, [id1, id2]));
        });

        set_identity_registry_storage(&e, &irs);
        set_claim_topics_and_issuers(&e, &cti);

        // Should succeed with second issuer
        verify_identity(&e, &user_address);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn verify_identity_fails_all_issuers_invalid() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let user_address = Address::generate(&e);

    e.as_contract(&address, || {
        let (identity_claims, issuer1, irs, cti) = setup_verification_contracts(&e);
        let issuer2 = e.register(MockClaimIssuer, ());

        // Both issuers return invalid claims
        e.as_contract(&issuer1, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &false);
        });
        e.as_contract(&issuer2, || {
            e.storage().persistent().set(&symbol_short!("claim_ok"), &false);
        });

        // Update claim topics and issuers to include both
        e.as_contract(&cti, || {
            e.storage()
                .persistent()
                .set(&symbol_short!("issuers"), &vec![&e, issuer1.clone(), issuer2.clone()]);
        });

        e.as_contract(&identity_claims, || {
            let claim1 = construct_claim(&e, &issuer2, 1);
            let claim2 = construct_claim(&e, &issuer2, 1);
            let id1 = generate_claim_id(&e, &issuer1, 1);
            let id2 = generate_claim_id(&e, &issuer2, 1);
            e.storage()
                .persistent()
                .set(&IdentityClaimsMockStorageKey::Claim(id1.clone()), &claim1);
            e.storage()
                .persistent()
                .set(&IdentityClaimsMockStorageKey::Claim(id2.clone()), &claim2);
            e.storage()
                .persistent()
                .set(&symbol_short!("claim_ids"), &Vec::from_array(&e, [id1, id2]));
        });

        set_identity_registry_storage(&e, &irs);
        set_claim_topics_and_issuers(&e, &cti);

        verify_identity(&e, &user_address);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn verify_identity_fails_no_matched_ids() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let user_address = Address::generate(&e);

    e.as_contract(&address, || {
        let (identity_claims, issuer1, irs, cti) = setup_verification_contracts(&e);
        let issuer2 = e.register(MockClaimIssuer, ());
        let issuer3 = e.register(MockClaimIssuer, ());

        // Both issuers return invalid claims
        e.as_contract(&identity_claims, || {
            // set claim id from another issuer
            let id3 = generate_claim_id(&e, &issuer3, 1);

            e.storage().persistent().set(&symbol_short!("claim_ids"), &Vec::from_array(&e, [id3]));
        });

        // Update claim topics and issuers to include issuer1 and issuer2, but not
        // issuer3
        e.as_contract(&cti, || {
            e.storage()
                .persistent()
                .set(&symbol_short!("issuers"), &vec![&e, issuer1.clone(), issuer2.clone()]);
        });

        set_identity_registry_storage(&e, &irs);
        set_claim_topics_and_issuers(&e, &cti);

        verify_identity(&e, &user_address);
    });
}
