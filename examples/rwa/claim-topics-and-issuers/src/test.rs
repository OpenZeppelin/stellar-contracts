extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

use crate::contract::{ClaimTopicsAndIssuersContract, ClaimTopicsAndIssuersContractClient};

fn create_client(e: &Env) -> (Address, ClaimTopicsAndIssuersContractClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(ClaimTopicsAndIssuersContract, (&admin,));
    let client = ClaimTopicsAndIssuersContractClient::new(e, &contract_id);
    (admin, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);

    // Initially no claim topics should exist
    assert_eq!(client.get_claim_topics().len(), 0);
    assert_eq!(client.get_trusted_issuers().len(), 0);
}

#[test]
fn test_claim_topic_management() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);

    // Add claim topic
    client.add_claim_topic(&1u32, &admin);
    let topics = client.get_claim_topics();
    assert_eq!(topics.len(), 1);
    assert_eq!(topics.get(0).unwrap(), 1u32);

    // Remove claim topic
    client.remove_claim_topic(&1u32, &admin);
    assert_eq!(client.get_claim_topics().len(), 0);
}

#[test]
fn test_trusted_issuer_management() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);
    let issuer = Address::generate(&e);
    let claim_topics = Vec::from_array(&e, [1u32, 2u32]);

    // Add trusted issuer
    client.add_trusted_issuer(&issuer, &claim_topics, &admin);

    assert!(client.is_trusted_issuer(&issuer));
    let issuers = client.get_trusted_issuers();
    assert_eq!(issuers.len(), 1);
    assert_eq!(issuers.get(0).unwrap(), issuer);

    // Check issuer claim topics
    let issuer_topics = client.get_trusted_issuer_claim_topics(&issuer);
    assert_eq!(issuer_topics.len(), 2);
    assert!(client.has_claim_topic(&issuer, &1u32));
    assert!(client.has_claim_topic(&issuer, &2u32));

    // Remove trusted issuer
    client.remove_trusted_issuer(&issuer, &admin);
    assert!(!client.is_trusted_issuer(&issuer));
}

#[test]
fn test_setup_default_topics() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);

    // Setup default topics
    client.setup_default_topics(&admin);

    let topics = client.get_claim_topics();
    assert_eq!(topics.len(), 4);
    assert!(topics.contains(1u32)); // KYC
    assert!(topics.contains(2u32)); // AML
    assert!(topics.contains(3u32)); // Accredited Investor
    assert!(topics.contains(4u32)); // Country Verification
}
