extern crate std;
use ed25519_dalek::Signer as Ed25519Signer;
use k256::{
    ecdsa::SigningKey as Secp256k1SigningKey, elliptic_curve::sec1::ToEncodedPoint,
    SecretKey as Secp256k1SecretKey,
};
use p256::{
    ecdsa::{
        signature::hazmat::PrehashSigner, Signature as Secp256r1Signature,
        SigningKey as Secp256r1SigningKey,
    },
    SecretKey as Secp256r1SecretKey,
};
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Bytes, BytesN, Env, Map, Vec,
};

use crate::rwa::{
    claim_issuer::{
        storage::{
            allow_key, get_keys_for_topic, get_registries, is_claim_revoked,
            is_key_allowed_for_topic, remove_key, set_claim_revoked, ClaimIssuerStorageKey,
            Ed25519SignatureData, Ed25519Verifier, Secp256k1SignatureData, Secp256k1Verifier,
            Secp256r1SignatureData, Secp256r1Verifier, SigningKey,
        },
        SignatureVerifier, MAX_KEYS_PER_TOPIC, MAX_REGISTRIES_PER_KEY,
    },
    claim_topics_and_issuers::{
        storage as ct_storage, ClaimTopicsAndIssuers, ClaimTopicsAndIssuersClient,
    },
};

#[contract]
struct MockContract;

#[contract]
struct MockClaimTopicsAndIssuersContract;

#[contractimpl]
impl ClaimTopicsAndIssuers for MockClaimTopicsAndIssuersContract {
    fn add_claim_topic(e: &Env, claim_topic: u32, _operator: Address) {
        ct_storage::add_claim_topic(e, claim_topic);
    }

    fn remove_claim_topic(e: &Env, claim_topic: u32, _operator: Address) {
        ct_storage::remove_claim_topic(e, claim_topic);
    }

    fn get_claim_topics(e: &Env) -> Vec<u32> {
        ct_storage::get_claim_topics(e)
    }

    fn add_trusted_issuer(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        _operator: Address,
    ) {
        ct_storage::add_trusted_issuer(e, &trusted_issuer, &claim_topics);
    }

    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, _operator: Address) {
        ct_storage::remove_trusted_issuer(e, &trusted_issuer);
    }

    fn get_trusted_issuers(e: &Env) -> Vec<Address> {
        ct_storage::get_trusted_issuers(e)
    }

    fn get_claim_topic_issuers(e: &Env, claim_topic: u32) -> Vec<Address> {
        ct_storage::get_claim_topic_issuers(e, claim_topic)
    }

    fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
        ct_storage::get_claim_topics_and_issuers(e)
    }

    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        _operator: Address,
    ) {
        ct_storage::update_issuer_claim_topics(e, &trusted_issuer, &claim_topics);
    }

    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool {
        ct_storage::is_trusted_issuer(e, &issuer)
    }

    fn get_trusted_issuer_claim_topics(e: &Env, trusted_issuer: Address) -> Vec<u32> {
        ct_storage::get_trusted_issuer_claim_topics(e, &trusted_issuer)
    }

    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool {
        ct_storage::has_claim_topic(e, &issuer, claim_topic)
    }
}

/// Helper to setup a mock registry contract
fn setup_mock_registry(e: &Env, issuer: &Address, topics: &[u32]) -> Address {
    let registry_id = e.register(MockClaimTopicsAndIssuersContract, ());
    let registry_client = ClaimTopicsAndIssuersClient::new(e, &registry_id);
    let operator = Address::generate(e);

    for topic in topics {
        registry_client.add_claim_topic(topic, &operator);
    }

    registry_client.add_trusted_issuer(issuer, &Vec::from_slice(e, topics), &operator);

    registry_id
}

/// Helper function to create test signature data for Ed25519
fn create_ed25519_test_data(e: &Env) -> (Bytes, Ed25519SignatureData) {
    let public_key = BytesN::<32>::from_array(e, &[1u8; 32]);
    let signature = BytesN::<64>::from_array(e, &[2u8; 64]);

    let mut sig_data = Bytes::new(e);
    sig_data.append(&public_key.clone().into());
    sig_data.append(&signature.clone().into());

    let expected_data = Ed25519SignatureData { public_key, signature };

    (sig_data, expected_data)
}

/// Helper function to create test signature data for Secp256r1
fn create_secp256r1_test_data(e: &Env) -> (Bytes, Secp256r1SignatureData) {
    let public_key = BytesN::<65>::from_array(e, &[3u8; 65]);
    let signature = BytesN::<64>::from_array(e, &[4u8; 64]);

    let mut sig_data = Bytes::new(e);
    sig_data.append(&public_key.clone().into());
    sig_data.append(&signature.clone().into());

    let expected_data = Secp256r1SignatureData { public_key, signature };

    (sig_data, expected_data)
}

/// Helper function to create test signature data for Secp256k1
fn create_secp256k1_test_data(e: &Env) -> (Bytes, Secp256k1SignatureData) {
    let public_key = BytesN::<65>::from_array(e, &[5u8; 65]);
    let signature = BytesN::<64>::from_array(e, &[6u8; 64]);
    let recovery_id = 1u32;

    let mut sig_data = Bytes::new(e);
    sig_data.append(&public_key.clone().into());
    sig_data.append(&signature.clone().into());
    sig_data.extend_from_array(&recovery_id.to_be_bytes());

    let expected_data = Secp256k1SignatureData { public_key, signature, recovery_id };

    (sig_data, expected_data)
}

#[test]
fn ed25519_extract_signature_data_success() {
    let e = Env::default();

    let (sig_data, expected_data) = create_ed25519_test_data(&e);

    let extracted_data = Ed25519Verifier::extract_signature_data(&e, &sig_data);

    assert_eq!(extracted_data.public_key, expected_data.public_key);
    assert_eq!(extracted_data.signature, expected_data.signature);
}

#[test]
#[should_panic(expected = "Error(Contract, #350)")]
fn ed25519_extract_signature_data_invalid_length() {
    let e = Env::default();

    let invalid_sig_data = Bytes::from_array(&e, &[1u8; 50]); // Wrong length

    Ed25519Verifier::extract_signature_data(&e, &invalid_sig_data);
}

#[test]
fn ed25519_expected_sig_data_len() {
    assert_eq!(Ed25519Verifier::expected_sig_data_len(), 96); // 32 + 64
}

#[test]
fn secp256r1_extract_signature_data_success() {
    let e = Env::default();

    let (sig_data, expected_data) = create_secp256r1_test_data(&e);

    let extracted_data = Secp256r1Verifier::extract_signature_data(&e, &sig_data);

    assert_eq!(extracted_data.public_key, expected_data.public_key);
    assert_eq!(extracted_data.signature, expected_data.signature);
}

#[test]
#[should_panic(expected = "Error(Contract, #350)")]
fn secp256r1_extract_signature_data_invalid_length() {
    let e = Env::default();

    let invalid_sig_data = Bytes::from_array(&e, &[1u8; 100]); // Wrong length

    Secp256r1Verifier::extract_signature_data(&e, &invalid_sig_data);
}

#[test]
fn secp256r1_expected_sig_data_len() {
    assert_eq!(Secp256r1Verifier::expected_sig_data_len(), 129); // 65 + 64
}

#[test]
fn secp256k1_extract_signature_data_success() {
    let e = Env::default();
    let (sig_data, expected_data) = create_secp256k1_test_data(&e);

    let extracted_data = Secp256k1Verifier::extract_signature_data(&e, &sig_data);

    assert_eq!(extracted_data.public_key, expected_data.public_key);
    assert_eq!(extracted_data.signature, expected_data.signature);
    assert_eq!(extracted_data.recovery_id, expected_data.recovery_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #350)")]
fn secp256k1_extract_signature_data_invalid_length() {
    let e = Env::default();
    let invalid_sig_data = Bytes::from_array(&e, &[1u8; 120]); // Wrong length

    Secp256k1Verifier::extract_signature_data(&e, &invalid_sig_data);
}

#[test]
fn secp256k1_expected_sig_data_len() {
    assert_eq!(Secp256k1Verifier::expected_sig_data_len(), 133); // 65 + 64 + 4
}

#[test]
fn secp256k1_recovery_id_extraction() {
    let e = Env::default();
    let public_key = BytesN::<65>::from_array(&e, &[5u8; 65]);
    let signature = BytesN::<64>::from_array(&e, &[6u8; 64]);
    let recovery_id = 0x12345678u32; // Test specific recovery ID

    let mut sig_data = Bytes::new(&e);
    sig_data.append(&public_key.into());
    sig_data.append(&signature.into());
    sig_data.extend_from_array(&recovery_id.to_be_bytes());

    let extracted_data = Secp256k1Verifier::extract_signature_data(&e, &sig_data);

    assert_eq!(extracted_data.recovery_id, recovery_id);
}

#[test]
fn ed25519_verify_success() {
    let e = Env::default();

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    let secret_key: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_key);
    let verifying_key = signing_key.verifying_key();

    let public_key: BytesN<32> = BytesN::from_array(&e, verifying_key.as_bytes());

    // Build message using the verifier
    let message = Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data);

    // Convert message to buffer for signing
    let message_buf = message.to_buffer::<256>();
    let message_slice = &message_buf.as_slice()[..message.len() as usize];
    let sig = signing_key.sign(message_slice).to_bytes();
    let signature = BytesN::<64>::from_array(&e, &sig);

    let signature_data = Ed25519SignatureData { public_key, signature };

    assert!(Ed25519Verifier::verify(&e, &message, &signature_data))
}

#[test]
fn secp256k1_verify_success() {
    let e = Env::default();

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    let secret_key_bytes: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];
    let secret_key = Secp256k1SecretKey::from_slice(&secret_key_bytes).unwrap();
    let signing_key = Secp256k1SigningKey::from(&secret_key);

    let pubkey = secret_key.public_key().to_encoded_point(false).to_bytes().to_vec();

    let mut pubkey_slice = [0u8; 65];
    pubkey_slice.copy_from_slice(&pubkey);
    let public_key: BytesN<65> = BytesN::from_array(&e, &pubkey_slice);

    // Build message using the verifier
    let message = Secp256k1Verifier::build_message(&e, &identity, claim_topic, &claim_data);
    let digest = e.crypto().keccak256(&message);

    let (signature, recovery_id) =
        signing_key.sign_prehash_recoverable(&digest.to_array()).unwrap();

    let sig_slice = signature.to_bytes();
    let mut sig = [0u8; 64];
    sig.copy_from_slice(sig_slice.as_slice());

    let signature_data = Secp256k1SignatureData {
        public_key,
        signature: BytesN::from_array(&e, &sig),
        recovery_id: recovery_id.to_byte() as u32,
    };

    assert!(Secp256k1Verifier::verify(&e, &message, &signature_data));
}

#[test]
fn secp256r1_verify_success() {
    let e = Env::default();

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    let secret_key_bytes: [u8; 32] = [
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63, 64,
    ];
    let secret_key = Secp256r1SecretKey::from_slice(&secret_key_bytes).unwrap();
    let signing_key = Secp256r1SigningKey::from(&secret_key);

    let pubkey = secret_key.public_key().to_encoded_point(false).to_bytes().to_vec();

    let mut pubkey_slice = [0u8; 65];
    pubkey_slice.copy_from_slice(&pubkey);
    let public_key: BytesN<65> = BytesN::from_array(&e, &pubkey_slice);

    // Build message using the verifier
    let message = Secp256r1Verifier::build_message(&e, &identity, claim_topic, &claim_data);
    // For Secp256r1, use SHA256 hash
    let digest = e.crypto().sha256(&message);

    let signature: Secp256r1Signature = signing_key.sign_prehash(&digest.to_array()).unwrap();

    let sig_slice = signature.normalize_s().unwrap_or(signature).to_bytes();
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&sig_slice);

    let signature_data =
        Secp256r1SignatureData { public_key, signature: BytesN::from_array(&e, &sig) };

    assert!(Secp256r1Verifier::verify(&e, &message, &signature_data));
}

#[test]
fn signature_verifier_different_inputs_different_messages() {
    let e = Env::default();

    let identity1 = Address::generate(&e);
    let identity2 = Address::generate(&e);
    let claim_data = Bytes::from_array(&e, &[1, 2, 3]);

    // Different identities should produce different messages
    let message1 = Ed25519Verifier::build_message(&e, &identity1, 1u32, &claim_data);
    let message2 = Ed25519Verifier::build_message(&e, &identity2, 1u32, &claim_data);
    assert_ne!(message1, message2);

    // Different topics should produce different messages
    let message3 = Ed25519Verifier::build_message(&e, &identity1, 2u32, &claim_data);
    assert_ne!(message1, message3);

    // Different data should produce different messages
    let different_data = Bytes::from_array(&e, &[4, 5, 6]);
    let message4 = Ed25519Verifier::build_message(&e, &identity1, 1u32, &different_data);
    assert_ne!(message1, message4);
}

#[test]
fn set_and_check_claim_revocation() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3]);

    e.as_contract(&contract_id, || {
        assert!(!is_claim_revoked(&e, &identity, claim_topic, &claim_data));

        set_claim_revoked(&e, &identity, claim_topic, &claim_data, true);

        assert!(is_claim_revoked(&e, &identity, claim_topic, &claim_data));
    });
}

#[test]
fn unrevoke_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3]);

    e.as_contract(&contract_id, || {
        set_claim_revoked(&e, &identity, claim_topic, &claim_data, true);
        assert!(is_claim_revoked(&e, &identity, claim_topic, &claim_data));

        set_claim_revoked(&e, &identity, claim_topic, &claim_data, false);
        assert!(!is_claim_revoked(&e, &identity, claim_topic, &claim_data));
    });
}

#[test]
fn revocation_edge_cases() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let identity1 = Address::generate(&e);
        let identity2 = Address::generate(&e);
        let claim_topic = 42u32;
        let claim_data1 = Bytes::from_array(&e, &[1, 2, 3]);
        let claim_data2 = Bytes::from_array(&e, &[4, 5, 6]);

        // Test revoking non-existent claim
        assert!(!is_claim_revoked(&e, &identity1, claim_topic, &claim_data1));

        // Test setting revocation multiple times
        set_claim_revoked(&e, &identity1, claim_topic, &claim_data1, true);
        set_claim_revoked(&e, &identity1, claim_topic, &claim_data1, true); // Should not error
        assert!(is_claim_revoked(&e, &identity1, claim_topic, &claim_data1));

        // Test unrevoking non-revoked claim
        set_claim_revoked(&e, &identity2, claim_topic, &claim_data2, false);
        assert!(!is_claim_revoked(&e, &identity2, claim_topic, &claim_data2));

        // Test multiple claims independently
        set_claim_revoked(&e, &identity2, claim_topic, &claim_data2, true);
        assert!(is_claim_revoked(&e, &identity1, claim_topic, &claim_data1));
        assert!(is_claim_revoked(&e, &identity2, claim_topic, &claim_data2));

        set_claim_revoked(&e, &identity1, claim_topic, &claim_data1, false);
        assert!(!is_claim_revoked(&e, &identity1, claim_topic, &claim_data1));
        assert!(is_claim_revoked(&e, &identity2, claim_topic, &claim_data2));
    });
}

// ====================== KEY MANAGEMENT =====================

#[test]
fn topic_specific_key_management() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[42, 43]);
    let public_key = Bytes::from_array(&e, &[2u8; 32]);
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        assert!(!is_key_allowed_for_topic(&e, &public_key, 1, topic));

        allow_key(&e, &public_key, &registry, 1, topic);
        assert!(is_key_allowed_for_topic(&e, &public_key, 1, topic));

        // check for different topic
        assert!(!is_key_allowed_for_topic(&e, &public_key, 1, topic + 1));
        // check for different scheme
        assert!(!is_key_allowed_for_topic(&e, &public_key, 2, topic));

        remove_key(&e, &public_key, &registry, 1, topic);
        assert!(!is_key_allowed_for_topic(&e, &public_key, 1, topic));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #352)")]
fn allow_key_already_allowed() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[999]);
    let public_key = Bytes::from_array(&e, &[7u8; 32]);
    let topic = 999u32;

    e.as_contract(&contract_id, || {
        // Add key first time
        allow_key(&e, &public_key, &registry, 1, topic);

        // Try to add same key again - should panic with KeyAlreadyAllowed
        allow_key(&e, &public_key, &registry, 1, topic);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #353)")]
fn remove_key_not_found() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let public_key = Bytes::from_array(&e, &[8u8; 32]);
    let topic = 888u32;

    e.as_contract(&contract_id, || {
        let registry = Address::generate(&e);
        // Try to remove non-existent key - should panic with KeyNotFound
        remove_key(&e, &public_key, &registry, 1, topic);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #351)")]
fn empty_key_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[123]);
    let empty_key = Bytes::new(&e);
    let topic = 123u32;

    e.as_contract(&contract_id, || {
        // Test with empty key
        allow_key(&e, &empty_key, &registry, 1, topic);
    });
}

#[test]
fn bidirectional_mapping_allow_key() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[42, 43]);

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &public_key, &registry, scheme, topic);

        // Verify key is allowed
        assert!(is_key_allowed_for_topic(&e, &public_key, scheme, topic));

        // Verify Topics mapping
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 1);
        assert_eq!(topic_keys.get(0).unwrap().public_key, public_key);
        assert_eq!(topic_keys.get(0).unwrap().scheme, scheme);

        // Verify Registries mapping
        let signing_key = SigningKey { public_key: public_key.clone(), scheme };
        let registries = get_registries(&e, &signing_key);
        assert_eq!(registries.len(), 1);
        assert_eq!(registries.get(0).unwrap(), registry);
    });
}

#[test]
fn bidirectional_mapping_remove_key() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[42]);

    let public_key = Bytes::from_array(&e, &[2u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &public_key, &registry, scheme, topic);
        remove_key(&e, &public_key, &registry, scheme, topic);

        // Verify key is no longer allowed
        assert!(!is_key_allowed_for_topic(&e, &public_key, scheme, topic));

        // Verify Topics mapping cleaned up
        let topics_key = ClaimIssuerStorageKey::Topics(topic);
        assert!(!e.storage().persistent().has(&topics_key));

        // Verify Registries mapping cleaned up
        let signing_key = SigningKey { public_key: public_key.clone(), scheme };
        let registries_key = ClaimIssuerStorageKey::Registries(signing_key);
        assert!(!e.storage().persistent().has(&registries_key));
    });
}

#[test]
fn bidirectional_mapping_multiple_keys_same_topic() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[42]);

    let key1 = Bytes::from_array(&e, &[1u8; 32]);
    let key2 = Bytes::from_array(&e, &[2u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &key1, &registry, scheme, topic);
        allow_key(&e, &key2, &registry, scheme, topic);

        // Verify both keys in Topics mapping
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 2);

        // Remove one key
        remove_key(&e, &key1, &registry, scheme, topic);

        // Verify Topics mapping still has one key
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 1);
        assert_eq!(topic_keys.get(0).unwrap().public_key, key2);

        // Remove second key
        remove_key(&e, &key2, &registry, scheme, topic);

        // Verify Topics mapping cleaned up
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #352)")]
fn bidirectional_mapping_same_key_same_registry_fails() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[42]);

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &public_key, &registry, scheme, topic);

        // Try to add same key again for same topic and same registry - should fail
        allow_key(&e, &public_key, &registry, scheme, topic);
    });
}

#[test]
fn bidirectional_mapping_same_key_different_registries() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry1 = setup_mock_registry(&e, &contract_id, &[42]);
    let registry2 = setup_mock_registry(&e, &contract_id, &[42]);

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &public_key, &registry1, scheme, topic);

        // Add same key for same topic but different registry - should succeed
        allow_key(&e, &public_key, &registry2, scheme, topic);

        // Verify key is allowed
        assert!(is_key_allowed_for_topic(&e, &public_key, scheme, topic));

        // Verify Topics mapping still has only one entry (key not duplicated)
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 1);

        // Verify Registries mapping has 2 registries
        let signing_key = SigningKey { public_key: public_key.clone(), scheme };
        let registries = get_registries(&e, &signing_key);
        assert_eq!(registries.len(), 2);
        assert_eq!(registries.get(0).unwrap(), registry1);
        assert_eq!(registries.get(1).unwrap(), registry2);
    });
}

#[test]
fn remove_key_granular_per_registry() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry1 = setup_mock_registry(&e, &contract_id, &[42]);
    let registry2 = setup_mock_registry(&e, &contract_id, &[42]);

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        // Add key for same topic with two different registries
        allow_key(&e, &public_key, &registry1, scheme, topic);
        allow_key(&e, &public_key, &registry2, scheme, topic);

        // Verify both registries are stored
        let signing_key = SigningKey { public_key: public_key.clone(), scheme };
        let registries = get_registries(&e, &signing_key);
        assert_eq!(registries.len(), 2);

        // Remove key from only registry1
        remove_key(&e, &public_key, &registry1, scheme, topic);

        // Key should still be allowed (because registry2 still has it)
        assert!(is_key_allowed_for_topic(&e, &public_key, scheme, topic));

        // Verify only registry2 remains
        let registries = get_registries(&e, &signing_key);
        assert_eq!(registries.len(), 1);
        assert_eq!(registries.get(0).unwrap(), registry2);

        // Verify Topics mapping still has the key
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 1);

        // Remove key from registry2
        remove_key(&e, &public_key, &registry2, scheme, topic);

        // Now key should not be allowed
        assert!(!is_key_allowed_for_topic(&e, &public_key, scheme, topic));

        // Verify Topics mapping is cleaned up
        let topic_keys = get_keys_for_topic(&e, topic);
        assert_eq!(topic_keys.len(), 0);
    });
}

#[test]
fn bidirectional_mapping_same_key_different_topics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry1 = setup_mock_registry(&e, &contract_id, &[42]);
    let registry2 = setup_mock_registry(&e, &contract_id, &[43]);

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &public_key, &registry1, scheme, 42);
        allow_key(&e, &public_key, &registry2, scheme, 43);

        // Verify both topics have the key
        let topic_keys_42 = get_keys_for_topic(&e, 42);
        assert_eq!(topic_keys_42.len(), 1);

        let topic_keys_43 = get_keys_for_topic(&e, 43);
        assert_eq!(topic_keys_43.len(), 1);

        // Verify Registries mapping has 2 entries (one per topic/registry combination)
        let signing_key = SigningKey { public_key: public_key.clone(), scheme };
        let registries = get_registries(&e, &signing_key);
        assert_eq!(registries.len(), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #356)")]
fn max_keys_per_topic_exceeded() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry = setup_mock_registry(&e, &contract_id, &[42]);

    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        // Add MAX_KEYS_PER_TOPIC keys
        for i in 0..MAX_KEYS_PER_TOPIC {
            let key = Bytes::from_array(&e, &[i as u8; 32]);
            allow_key(&e, &key, &registry, scheme, topic);
        }

        // Try to add one more - should panic
        let extra_key = Bytes::from_array(&e, &[255u8; 32]);
        allow_key(&e, &extra_key, &registry, scheme, topic);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #357)")]
fn max_registries_per_key_exceeded() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;

    e.as_contract(&contract_id, || {
        // Add MAX_REGISTRIES_PER_KEY registries for different topics
        for i in 0..MAX_REGISTRIES_PER_KEY {
            let registry = setup_mock_registry(&e, &contract_id, &[i]);
            allow_key(&e, &public_key, &registry, scheme, i);
        }

        // Try to add one more - should panic
        let extra_registry = setup_mock_registry(&e, &contract_id, &[MAX_REGISTRIES_PER_KEY]);
        allow_key(&e, &public_key, &extra_registry, scheme, MAX_REGISTRIES_PER_KEY);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #354)")]
fn allow_key_issuer_not_registered() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry_id = e.register(MockClaimTopicsAndIssuersContract, ());

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        // Try to allow key without being registered - should panic
        allow_key(&e, &public_key, &registry_id, scheme, topic);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #355)")]
fn allow_key_topic_not_allowed() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let registry_id = e.register(MockClaimTopicsAndIssuersContract, ());
    let registry_client = ClaimTopicsAndIssuersClient::new(&e, &registry_id);
    let operator = Address::generate(&e);

    // Add a different topic (99) to the registry
    registry_client.add_claim_topic(&99u32, &operator);

    // Register issuer with topic 99, but we'll try to use topic 42
    let mut topics = Vec::new(&e);
    topics.push_back(99u32);
    registry_client.add_trusted_issuer(&contract_id, &topics, &operator);

    let public_key = Bytes::from_array(&e, &[1u8; 32]);
    let scheme = 1u32;
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        // Try to allow key for topic 42 which is not in the issuer's allowed topics -
        // should panic
        allow_key(&e, &public_key, &registry_id, scheme, topic);
    });
}
