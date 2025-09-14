extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, String};

use crate::contract::{IdentityClaimsContract, IdentityClaimsContractClient};

fn create_client(e: &Env) -> (Address, IdentityClaimsContractClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(IdentityClaimsContract, (&admin,));
    let client = IdentityClaimsContractClient::new(e, &contract_id);
    (admin, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, _client) = create_client(&e);

    // Contract should initialize successfully
}

#[test]
fn test_add_and_get_claim() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let issuer = Address::generate(&e);

    let topic = 1u32; // KYC
    let scheme = 1u32; // ECDSA
    let signature = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com/claim");

    // Add claim
    let claim_id = client.add_claim(&topic, &scheme, &issuer, &signature, &data, &uri);

    // Get claim
    let claim = client.get_claim(&claim_id);
    assert_eq!(claim.topic, topic);
    assert_eq!(claim.scheme, scheme);
    assert_eq!(claim.issuer, issuer);
    assert_eq!(claim.signature, signature);
    assert_eq!(claim.data, data);
    assert_eq!(claim.uri, uri);
}

#[test]
fn test_get_claim_ids_by_topic() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let issuer = Address::generate(&e);

    let topic = 1u32; // KYC
    let scheme = 1u32; // ECDSA
    let signature = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(&e, &[5, 6, 7, 8]);
    let uri = String::from_str(&e, "https://example.com/claim");

    // Initially no claims for topic
    let claim_ids = client.get_claim_ids_by_topic(&topic);
    assert_eq!(claim_ids.len(), 0);

    // Add claim
    let claim_id = client.add_claim(&topic, &scheme, &issuer, &signature, &data, &uri);

    // Should now have one claim for topic
    let claim_ids = client.get_claim_ids_by_topic(&topic);
    assert_eq!(claim_ids.len(), 1);
    assert_eq!(claim_ids.get(0).unwrap(), claim_id);
}
