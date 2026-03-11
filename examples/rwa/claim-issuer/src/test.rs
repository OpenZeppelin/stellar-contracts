extern crate std;

use ed25519_dalek::Signer as Ed25519Signer;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    Address, Bytes, BytesN, Env, Map, Vec,
};
use stellar_tokens::rwa::{
    claim_issuer::{encode_claim_data_expiration, Ed25519Verifier, SignatureVerifier},
    claim_topics_and_issuers::{
        storage as cti, ClaimTopicsAndIssuers, ClaimTopicsAndIssuersClient,
    },
};

use crate::contract::{ClaimIssuerContract, ClaimIssuerContractClient, ED25519_SCHEME};

// ============ Mock registry ============

#[contract]
struct MockRegistry;

#[contractimpl]
impl ClaimTopicsAndIssuers for MockRegistry {
    fn add_claim_topic(e: &Env, claim_topic: u32, _operator: Address) {
        cti::add_claim_topic(e, claim_topic);
    }

    fn remove_claim_topic(e: &Env, claim_topic: u32, _operator: Address) {
        cti::remove_claim_topic(e, claim_topic);
    }

    fn get_claim_topics(e: &Env) -> Vec<u32> {
        cti::get_claim_topics(e)
    }

    fn add_trusted_issuer(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        _operator: Address,
    ) {
        cti::add_trusted_issuer(e, &trusted_issuer, &claim_topics);
    }

    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, _operator: Address) {
        cti::remove_trusted_issuer(e, &trusted_issuer);
    }

    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        _operator: Address,
    ) {
        cti::update_issuer_claim_topics(e, &trusted_issuer, &claim_topics);
    }

    fn get_trusted_issuers(e: &Env) -> Vec<Address> {
        cti::get_trusted_issuers(e)
    }

    fn get_claim_topic_issuers(e: &Env, claim_topic: u32) -> Vec<Address> {
        cti::get_claim_topic_issuers(e, claim_topic)
    }

    fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
        cti::get_claim_topics_and_issuers(e)
    }

    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool {
        cti::is_trusted_issuer(e, &issuer)
    }

    fn get_trusted_issuer_claim_topics(e: &Env, trusted_issuer: Address) -> Vec<u32> {
        cti::get_trusted_issuer_claim_topics(e, &trusted_issuer)
    }

    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool {
        cti::has_claim_topic(e, &issuer, claim_topic)
    }
}

// ============ Helpers ============

struct Ed25519KeyPair {
    signing_key: ed25519_dalek::SigningKey,
    public_key_bytes: [u8; 32],
}

impl Ed25519KeyPair {
    fn generate(secret: [u8; 32]) -> Self {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
        let public_key_bytes = signing_key.verifying_key().to_bytes();
        Self { signing_key, public_key_bytes }
    }

    fn public_key(&self, e: &Env) -> Bytes {
        Bytes::from_array(e, &self.public_key_bytes)
    }

    fn sign_message(&self, e: &Env, message: &Bytes) -> BytesN<64> {
        let buf = message.to_buffer::<256>();
        let slice = &buf.as_slice()[..message.len() as usize];
        let sig = self.signing_key.sign(slice).to_bytes();
        BytesN::from_array(e, &sig)
    }
}

fn make_sig_data(e: &Env, public_key: &Bytes, signature: &BytesN<64>) -> Bytes {
    let mut sig_data = Bytes::new(e);
    sig_data.append(public_key);
    sig_data.append(&signature.clone().into());
    sig_data
}

/// Registers a `ClaimIssuerContract` and a `MockRegistry`. Adds `claim_topic`
/// to the registry and registers the issuer contract as a trusted issuer for
/// that topic. Returns `(client, registry_address_address)`.
fn setup<'a>(e: &Env, claim_topic: u32) -> (ClaimIssuerContractClient<'a>, Address) {
    let owner = Address::generate(e);
    let issuer_addr = e.register(ClaimIssuerContract, (&owner,));
    let client = ClaimIssuerContractClient::new(e, &issuer_addr);

    let registry = e.register(MockRegistry, ());
    let operator = Address::generate(e);
    let reg_client = ClaimTopicsAndIssuersClient::new(e, &registry);
    reg_client.add_claim_topic(&claim_topic, &operator);
    reg_client.add_trusted_issuer(&issuer_addr, &soroban_sdk::vec![e, claim_topic], &operator);

    (client, registry)
}

fn default_claim_data(e: &Env) -> Bytes {
    let raw = Bytes::from_array(e, &[1u8, 2, 3, 4, 5]);
    let created_at = e.ledger().timestamp();
    let valid_until = created_at + 1000;
    encode_claim_data_expiration(e, created_at, valid_until, &raw)
}

// ============ Key management ============

#[test]
fn allow_key_and_remove_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([1u8; 32]);

    client.allow_key(&kp.public_key(&e), &registry, &claim_topic);
    client.remove_key(&kp.public_key(&e), &registry, &claim_topic);
}

#[test]
#[should_panic(expected = "Error(Contract, #352)")]
fn allow_key_duplicate_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([1u8; 32]);

    client.allow_key(&kp.public_key(&e), &registry, &claim_topic);
    client.allow_key(&kp.public_key(&e), &registry, &claim_topic);
}

#[test]
#[should_panic(expected = "Error(Contract, #353)")]
fn remove_nonexistent_key_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([1u8; 32]);

    client.remove_key(&kp.public_key(&e), &registry, &claim_topic);
}

#[test]
#[should_panic(expected = "Error(Contract, #354)")]
fn allow_key_for_unregistered_topic_panics() {
    let e = Env::default();
    e.mock_all_auths();
    // Registry only knows about topic 1; trying to allow key for topic 99 fails.
    let (client, registry) = setup(&e, 1u32);

    let kp = Ed25519KeyPair::generate([1u8; 32]);

    client.allow_key(&kp.public_key(&e), &registry, &99u32);
}

// ============ is_claim_valid ============

#[test]
fn is_claim_valid_works() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([42u8; 32]);
    let identity = Address::generate(&e);

    client.allow_key(&kp.public_key(&e), &registry, &claim_topic);

    let claim_data = default_claim_data(&e);

    let issuer_addr = client.address.clone();
    let message = e.as_contract(&issuer_addr, || {
        Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data)
    });
    let signature = kp.sign_message(&e, &message);
    let sig_data = make_sig_data(&e, &kp.public_key(&e), &signature);

    client.is_claim_valid(&identity, &claim_topic, &ED25519_SCHEME, &sig_data, &claim_data);
}

#[test]
#[should_panic(expected = "Error(Contract, #350)")]
fn is_claim_valid_wrong_scheme_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, _registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([42u8; 32]);
    let identity = Address::generate(&e);
    let claim_data = default_claim_data(&e);

    let dummy_signature = BytesN::from_array(&e, &[0u8; 64]);
    let sig_data = make_sig_data(&e, &kp.public_key(&e), &dummy_signature);

    client.is_claim_valid(&identity, &claim_topic, &999u32, &sig_data, &claim_data);
}

#[test]
#[should_panic(expected = "Error(Contract, #354)")]
fn is_claim_valid_unauthorized_key_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, _registry) = setup(&e, claim_topic);

    // Key is never authorized via allow_key.
    let kp = Ed25519KeyPair::generate([42u8; 32]);
    let identity = Address::generate(&e);
    let claim_data = default_claim_data(&e);

    let issuer_addr = client.address.clone();
    let message = e.as_contract(&issuer_addr, || {
        Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data)
    });
    let signature = kp.sign_message(&e, &message);
    let sig_data = make_sig_data(&e, &kp.public_key(&e), &signature);

    client.is_claim_valid(&identity, &claim_topic, &ED25519_SCHEME, &sig_data, &claim_data);
}

#[test]
#[should_panic(expected = "Error(Contract, #354)")]
fn is_claim_valid_expired_claim_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([42u8; 32]);
    let identity = Address::generate(&e);

    client.allow_key(&kp.public_key(&e), &registry, &claim_topic);

    // Claim expires at timestamp 500.
    let raw = Bytes::from_array(&e, &[1u8, 2, 3]);
    let claim_data = encode_claim_data_expiration(&e, 100, 500, &raw);

    // Build message and sign while ledger is still at the default (0) timestamp.
    let issuer_addr = client.address.clone();
    let message = e.as_contract(&issuer_addr, || {
        Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data)
    });
    let signature = kp.sign_message(&e, &message);
    let sig_data = make_sig_data(&e, &kp.public_key(&e), &signature);

    // Advance ledger past expiry.
    e.ledger().set_timestamp(600);

    client.is_claim_valid(&identity, &claim_topic, &ED25519_SCHEME, &sig_data, &claim_data);
}

#[test]
#[should_panic(expected = "Error(Contract, #354)")]
fn is_claim_valid_after_remove_key_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, registry) = setup(&e, claim_topic);

    let kp = Ed25519KeyPair::generate([42u8; 32]);
    let identity = Address::generate(&e);

    client.allow_key(&kp.public_key(&e), &registry, &claim_topic);

    let claim_data = default_claim_data(&e);

    let issuer_addr = client.address.clone();
    let message = e.as_contract(&issuer_addr, || {
        Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data)
    });
    let signature = kp.sign_message(&e, &message);
    let sig_data = make_sig_data(&e, &kp.public_key(&e), &signature);

    // Revoke the key.
    client.remove_key(&kp.public_key(&e), &registry, &claim_topic);

    client.is_claim_valid(&identity, &claim_topic, &ED25519_SCHEME, &sig_data, &claim_data);
}

#[test]
#[should_panic(expected = "Error(Contract, #350)")]
fn is_claim_valid_sig_data_too_short_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let claim_topic = 1u32;
    let (client, _registry) = setup(&e, claim_topic);

    let identity = Address::generate(&e);
    let claim_data = default_claim_data(&e);

    // 32 bytes — too short for the 96-byte Ed25519 sig_data.
    let short_sig_data = Bytes::from_array(&e, &[0u8; 32]);

    client.is_claim_valid(&identity, &claim_topic, &ED25519_SCHEME, &short_sig_data, &claim_data);
}
