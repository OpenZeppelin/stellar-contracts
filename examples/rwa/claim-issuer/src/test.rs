extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ClaimIssuerContract, ClaimIssuerContractClient};

fn create_client(e: &Env) -> (Address, ClaimIssuerContractClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(ClaimIssuerContract, (&admin,));
    let client = ClaimIssuerContractClient::new(e, &contract_id);
    (admin, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);

    // Test that the contract was initialized properly
    assert_eq!(client.name(), String::from_str(&e, "Example Claim Issuer"));
}

#[test]
fn test_key_management() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);

    let key = soroban_sdk::Bytes::from_array(&e, &[1, 2, 3, 4]);
    let claim_topic = 1u32;

    // Initially key should not be allowed
    assert!(!client.is_key_allowed(&key, &claim_topic));

    // Allow the key
    client.allow_key(&key, &claim_topic, &admin);
    assert!(client.is_key_allowed(&key, &claim_topic));

    // Remove the key
    client.remove_key(&key, &claim_topic, &admin);
    assert!(!client.is_key_allowed(&key, &claim_topic));
}

#[test]
fn test_claim_revocation() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = create_client(&e);

    let claim_digest = soroban_sdk::BytesN::from_array(&e, &[0u8; 32]);

    // Initially claim should not be revoked
    assert!(!client.is_claim_revoked(&claim_digest));

    // Revoke the claim
    client.revoke_claim(&claim_digest, &admin);
    assert!(client.is_claim_revoked(&claim_digest));
}
