extern crate std;

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Bytes, BytesN, Env, String, Vec};

use crate::rwa::{
    claim_issuer::ClaimIssuer,
    identity_claims::storage::{
        add_claim, get_claim, get_claim_ids_by_topic, remove_claim, remove_claim_from_topic_index,
        ClaimsStorageKey,
    },
};

pub mod mock_claim_issuer {
    use soroban_sdk::symbol_short;

    use super::*;

    #[contract]
    pub struct Contract;

    #[contractimpl]
    impl ClaimIssuer for Contract {
        fn is_claim_valid(
            e: &Env,
            _identity: Address,
            _claim_topic: u32,
            _sig_data: Bytes,
            _claim_data: Bytes,
        ) -> bool {
            e.storage().persistent().get(&symbol_short!("valid")).unwrap_or(true)
        }
    }
}

#[contract]
struct MockContract;

// Helper function to create common test data
fn setup_test_data(e: &Env) -> (Address, u32, u32, Bytes, Bytes, String) {
    let issuer = e.register(mock_claim_issuer::Contract, ());
    let topic = 1u32;
    let scheme = 1u32;
    let signature = Bytes::from_array(e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(e, &[5, 6, 7, 8]);
    let uri = String::from_str(e, "https://example.com");

    (issuer, topic, scheme, signature, data, uri)
}

#[test]
fn add_claim_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer, topic, scheme, signature, data, uri) = setup_test_data(&e);

    e.as_contract(&contract_id, || {
        let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);

        // Verify claim was stored
        let claim = get_claim(&e, &claim_id);

        assert_eq!(claim.topic, topic);
        assert_eq!(claim.scheme, scheme);
        assert_eq!(claim.issuer, issuer);
        assert_eq!(claim.signature, signature);
        assert_eq!(claim.data, data);
        assert_eq!(claim.uri, uri);

        // Verify claim is indexed by topic
        let claim_ids = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids.len(), 1);
        assert_eq!(claim_ids.get(0).unwrap(), claim_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #341)")]
fn add_claim_fails() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer, topic, scheme, signature, data, uri) = setup_test_data(&e);

    e.as_contract(&issuer, || {
        // Set mock claim issuer to return false
        e.storage().persistent().set(&symbol_short!("valid"), &false);
    });

    e.as_contract(&contract_id, || {
        add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    });
}

#[test]
fn update_existing_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer, topic, scheme, signature1, data1, _) = setup_test_data(&e);
    let uri1 = String::from_str(&e, "https://example1.com");

    e.as_contract(&contract_id, || {
        // Add initial claim
        let claim_id1 = add_claim(&e, topic, scheme, &issuer, &signature1, &data1, &uri1);

        // Update the same claim (same issuer + topic)
        let signature2 = Bytes::from_array(&e, &[9, 10, 11, 12]);
        let data2 = Bytes::from_array(&e, &[13, 14, 15, 16]);
        let uri2 = String::from_str(&e, "https://example2.com");

        let claim_id2 = add_claim(&e, topic, scheme, &issuer, &signature2, &data2, &uri2);

        // Should be the same claim ID
        assert_eq!(claim_id1, claim_id2);

        // Verify updated data
        let claim = get_claim(&e, &claim_id1);
        assert_eq!(claim.signature, signature2);
        assert_eq!(claim.data, data2);
        assert_eq!(claim.uri, uri2);

        // Should still only have one claim for this topic
        let claim_ids = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids.len(), 1);
    });
}

#[test]
fn multiple_claims_different_topics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer, _, scheme, signature, data, uri) = setup_test_data(&e);

    e.as_contract(&contract_id, || {
        // Add claims for different topics
        let claim_id1 = add_claim(&e, 1u32, scheme, &issuer, &signature, &data, &uri);
        let claim_id2 = add_claim(&e, 2u32, scheme, &issuer, &signature, &data, &uri);
        let claim_id3 = add_claim(&e, 1u32, scheme, &issuer, &signature, &data, &uri);

        // claim_id1 and claim_id3 should be the same (same issuer + topic)
        assert_eq!(claim_id1, claim_id3);
        assert_ne!(claim_id1, claim_id2);

        // Topic 1 should have 1 claim
        let topic1_claims = get_claim_ids_by_topic(&e, 1u32);
        assert_eq!(topic1_claims.len(), 1);

        // Topic 2 should have 1 claim
        let topic2_claims = get_claim_ids_by_topic(&e, 2u32);
        assert_eq!(topic2_claims.len(), 1);

        // Topic 3 should have no claims
        let topic3_claims = get_claim_ids_by_topic(&e, 3u32);
        assert_eq!(topic3_claims.len(), 0);
    });
}

#[test]
fn multiple_issuers_same_topic() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer1, topic, scheme, signature, data, uri) = setup_test_data(&e);
    let issuer2 = e.register(mock_claim_issuer::Contract, ());

    e.as_contract(&contract_id, || {
        // Add claims from different issuers for the same topic
        let claim_id1 = add_claim(&e, topic, scheme, &issuer1, &signature, &data, &uri);
        let claim_id2 = add_claim(&e, topic, scheme, &issuer2, &signature, &data, &uri);

        // Should be different claim IDs
        assert_ne!(claim_id1, claim_id2);

        // Topic should have 2 claims
        let topic_claims = get_claim_ids_by_topic(&e, topic);
        assert_eq!(topic_claims.len(), 2);
        assert!(topic_claims.contains(&claim_id1));
        assert!(topic_claims.contains(&claim_id2));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #340)")]
fn get_nonexistent_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let fake_claim_id = BytesN::from_array(&e, &[0u8; 32]);
    e.as_contract(&contract_id, || {
        get_claim(&e, &fake_claim_id);
    });
}

#[test]
fn claim_removal() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer, topic, scheme, signature, data, uri) = setup_test_data(&e);
    e.as_contract(&contract_id, || {
        // Add a claim
        let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);

        // Verify claim exists
        let claim = get_claim(&e, &claim_id);
        assert_eq!(claim.topic, topic);

        // Verify claim is in topic index
        let claim_ids = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids.len(), 1);
        assert_eq!(claim_ids.get(0).unwrap(), claim_id);

        // Remove the claim
        remove_claim(&e, &claim_id);

        // Verify claim is removed from topic index
        let claim_ids_after = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids_after.len(), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #340)")]
fn remove_nonexistent_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let fake_claim_id = BytesN::from_array(&e, &[0u8; 32]);

    e.as_contract(&contract_id, || {
        remove_claim(&e, &fake_claim_id);
    });
}

#[test]
fn remove_claim_from_topic_index_when_empty() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer, topic, scheme, signature, data, uri) = setup_test_data(&e);

    e.as_contract(&contract_id, || {
        // Add a single claim
        let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);

        // Verify claim is in topic index
        let claim_ids = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids.len(), 1);

        // Remove the claim from topic index
        remove_claim_from_topic_index(&e, topic, &claim_id);

        // Verify storage key should be removed
        assert!(e
            .storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&ClaimsStorageKey::ClaimsByTopic(topic))
            .is_none())
    });
}

#[test]
fn remove_claim_from_topic_index_when_not_empty() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let (issuer1, topic, scheme, signature, data, uri) = setup_test_data(&e);
    let issuer2 = e.register(mock_claim_issuer::Contract, ());

    e.as_contract(&contract_id, || {
        // Add two claims from different issuers for the same topic
        let claim_id1 = add_claim(&e, topic, scheme, &issuer1, &signature, &data, &uri);
        let claim_id2 = add_claim(&e, topic, scheme, &issuer2, &signature, &data, &uri);

        // Verify both claims are in topic index
        let claim_ids = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids.len(), 2);
        assert!(claim_ids.contains(&claim_id1));
        assert!(claim_ids.contains(&claim_id2));

        // Remove one claim from topic index
        remove_claim_from_topic_index(&e, topic, &claim_id1);

        // Verify only one claim remains in topic index
        let claim_ids_after = get_claim_ids_by_topic(&e, topic);
        assert_eq!(claim_ids_after.len(), 1);
        assert!(!claim_ids_after.contains(&claim_id1));
        assert!(claim_ids_after.contains(&claim_id2));
    });
}
