extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::contract::{ClaimTopicsAndIssuersContract, ClaimTopicsAndIssuersContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> ClaimTopicsAndIssuersContractClient<'a> {
    let address = e.register(ClaimTopicsAndIssuersContract, (admin, manager));
    ClaimTopicsAndIssuersContractClient::new(e, &address)
}

// ################## CLAIM TOPICS ##################

#[test]
fn add_and_get_claim_topics_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert!(client.get_claim_topics().is_empty());

    client.add_claim_topic(&1u32, &manager);
    client.add_claim_topic(&2u32, &manager);

    let topics = client.get_claim_topics();
    assert_eq!(topics.len(), 2);
    assert!(topics.contains(1u32));
    assert!(topics.contains(2u32));
}

#[test]
fn remove_claim_topic_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.add_claim_topic(&1u32, &manager);
    client.add_claim_topic(&2u32, &manager);

    client.remove_claim_topic(&1u32, &manager);

    let topics = client.get_claim_topics();
    assert_eq!(topics.len(), 1);
    assert!(!topics.contains(1u32));
    assert!(topics.contains(2u32));
}

#[test]
#[should_panic(expected = "Error(Contract, #372)")]
fn add_duplicate_claim_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.add_claim_topic(&1u32, &manager);
    client.add_claim_topic(&1u32, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #370)")]
fn remove_nonexistent_claim_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.remove_claim_topic(&99u32, &manager);
}

// ################## TRUSTED ISSUERS ##################

#[test]
fn add_and_get_trusted_issuer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);

    assert!(client.get_trusted_issuers().is_empty());
    assert!(!client.is_trusted_issuer(&issuer));

    client.add_trusted_issuer(&issuer, &vec![&e, 1u32], &manager);

    assert_eq!(client.get_trusted_issuers().len(), 1);
    assert!(client.is_trusted_issuer(&issuer));
    assert_eq!(client.get_trusted_issuer_claim_topics(&issuer), vec![&e, 1u32]);
    assert!(client.has_claim_topic(&issuer, &1u32));
}

#[test]
fn remove_trusted_issuer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);
    client.add_trusted_issuer(&issuer, &vec![&e, 1u32], &manager);

    client.remove_trusted_issuer(&issuer, &manager);

    assert!(client.get_trusted_issuers().is_empty());
    assert!(!client.is_trusted_issuer(&issuer));
}

#[test]
fn update_issuer_claim_topics_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);
    client.add_claim_topic(&2u32, &manager);
    client.add_trusted_issuer(&issuer, &vec![&e, 1u32], &manager);

    assert!(client.has_claim_topic(&issuer, &1u32));
    assert!(!client.has_claim_topic(&issuer, &2u32));

    client.update_issuer_claim_topics(&issuer, &vec![&e, 2u32], &manager);

    assert!(!client.has_claim_topic(&issuer, &1u32));
    assert!(client.has_claim_topic(&issuer, &2u32));
}

#[test]
#[should_panic(expected = "Error(Contract, #373)")]
fn add_duplicate_issuer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);
    client.add_trusted_issuer(&issuer, &vec![&e, 1u32], &manager);
    client.add_trusted_issuer(&issuer, &vec![&e, 1u32], &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #371)")]
fn remove_nonexistent_issuer_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.remove_trusted_issuer(&issuer, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #370)")]
fn add_issuer_for_nonexistent_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    // Topic 99 was never added
    client.add_trusted_issuer(&issuer, &vec![&e, 99u32], &manager);
}

// ################## REVERSE MAPPINGS ##################

#[test]
fn get_claim_topics_and_issuers_map_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer1 = Address::generate(&e);
    let issuer2 = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);
    client.add_claim_topic(&2u32, &manager);
    client.add_trusted_issuer(&issuer1, &vec![&e, 1u32], &manager);
    client.add_trusted_issuer(&issuer2, &vec![&e, 1u32, 2u32], &manager);

    let map = client.get_claim_topics_and_issuers();

    let topic1_issuers = map.get(1u32).unwrap();
    assert_eq!(topic1_issuers.len(), 2);
    assert!(topic1_issuers.contains(&issuer1));
    assert!(topic1_issuers.contains(&issuer2));

    let topic2_issuers = map.get(2u32).unwrap();
    assert_eq!(topic2_issuers.len(), 1);
    assert!(topic2_issuers.contains(&issuer2));
}

#[test]
fn get_claim_topic_issuers_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);
    client.add_trusted_issuer(&issuer, &vec![&e, 1u32], &manager);

    let issuers = client.get_claim_topic_issuers(&1u32);
    assert_eq!(issuers.len(), 1);
    assert!(issuers.contains(&issuer));
}

#[test]
fn remove_claim_topic_cleans_up_issuer_mappings() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    let issuer = Address::generate(&e);

    client.add_claim_topic(&1u32, &manager);
    client.add_claim_topic(&2u32, &manager);
    client.add_trusted_issuer(&issuer, &vec![&e, 1u32, 2u32], &manager);

    // Remove topic 1
    client.remove_claim_topic(&1u32, &manager);

    // The issuer should now only have topic 2
    let issuer_topics = client.get_trusted_issuer_claim_topics(&issuer);
    assert_eq!(issuer_topics.len(), 1);
    assert!(issuer_topics.contains(2u32));
    assert!(!issuer_topics.contains(1u32));
}
