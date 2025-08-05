#![cfg(test)]
extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Bytes, BytesN, Env, String};

use super::storage::{add_claim, get_claim, get_claim_ids_by_topic, remove_claim};

#[contract]
struct MockContract;

#[test]
fn add_claim_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let issuer = Address::generate(&e);
    let topic = 1u32;
    let scheme = 1u32;
    let signature = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com");

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
fn update_existing_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let issuer = Address::generate(&e);
    let topic = 1u32;
    let scheme = 1u32;
    let signature1 = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data1 = Bytes::from_array(&e, &[5, 6, 7, 8]);
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

    let issuer = Address::generate(&e);
    let signature = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com");

    e.as_contract(&contract_id, || {
        // Add claims for different topics
        let claim_id1 = add_claim(&e, 1u32, 1u32, &issuer, &signature, &data, &uri);
        let claim_id2 = add_claim(&e, 2u32, 1u32, &issuer, &signature, &data, &uri);
        let claim_id3 = add_claim(&e, 1u32, 1u32, &issuer, &signature, &data, &uri);

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

    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);
    let topic = 1u32;
    let signature = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com");

    e.as_contract(&contract_id, || {
        // Add claims from different issuers for the same topic
        let claim_id1 = add_claim(&e, topic, 1u32, &issuer1, &signature, &data, &uri);
        let claim_id2 = add_claim(&e, topic, 1u32, &issuer2, &signature, &data, &uri);

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
#[should_panic(expected = "Error(Contract, #1)")]
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

    let issuer = Address::generate(&e);
    let topic = 1u32;
    let scheme = 1u32;
    let signature = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com");
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
#[should_panic(expected = "Error(Contract, #1)")]
fn remove_nonexistent_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let fake_claim_id = BytesN::from_array(&e, &[0u8; 32]);

    e.as_contract(&contract_id, || {
        remove_claim(&e, &fake_claim_id);
    });
}
