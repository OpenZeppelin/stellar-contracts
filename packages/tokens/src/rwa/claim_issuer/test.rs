#![cfg(test)]
extern crate std;
use ed25519_dalek::{Signer, SigningKey, SECRET_KEY_LENGTH};
use soroban_sdk::{contract, testutils::Address as _, xdr::ToXdr, Address, Bytes, BytesN, Env};

use crate::rwa::claim_issuer::storage::{
    allow_key, allow_key_for_claim_topic, is_key_allowed, is_key_allowed_for_topic,
    is_key_universally_allowed, remove_key, remove_key_for_claim_topic, Ed25519SignatureData,
    Ed25519Verifier, Secp256k1SignatureData, Secp256k1Verifier, Secp256r1SignatureData,
    Secp256r1Verifier, SignatureVerifier,
};

#[contract]
struct MockContract;

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
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let (sig_data, expected_data) = create_ed25519_test_data(&e);

        let extracted_data = Ed25519Verifier::extract_signature_data(&e, &sig_data);

        assert_eq!(extracted_data.public_key, expected_data.public_key);
        assert_eq!(extracted_data.signature, expected_data.signature);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn ed25519_extract_signature_data_invalid_length() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let invalid_sig_data = Bytes::from_array(&e, &[1u8; 50]); // Wrong length

        Ed25519Verifier::extract_signature_data(&e, &invalid_sig_data);
    });
}

#[test]
fn ed25519_expected_sig_data_len() {
    assert_eq!(Ed25519Verifier::expected_sig_data_len(), 96); // 32 + 64
}

#[test]
fn secp256r1_extract_signature_data_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let (sig_data, expected_data) = create_secp256r1_test_data(&e);

        let extracted_data = Secp256r1Verifier::extract_signature_data(&e, &sig_data);

        assert_eq!(extracted_data.public_key, expected_data.public_key);
        assert_eq!(extracted_data.signature, expected_data.signature);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn secp256r1_extract_signature_data_invalid_length() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let invalid_sig_data = Bytes::from_array(&e, &[1u8; 100]); // Wrong length

        Secp256r1Verifier::extract_signature_data(&e, &invalid_sig_data);
    });
}

#[test]
fn secp256r1_expected_sig_data_len() {
    assert_eq!(Secp256r1Verifier::expected_sig_data_len(), 129); // 65 + 64
}

#[test]
fn secp256k1_extract_signature_data_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let (sig_data, expected_data) = create_secp256k1_test_data(&e);

        let extracted_data = Secp256k1Verifier::extract_signature_data(&e, &sig_data);

        assert_eq!(extracted_data.public_key, expected_data.public_key);
        assert_eq!(extracted_data.signature, expected_data.signature);
        assert_eq!(extracted_data.recovery_id, expected_data.recovery_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn secp256k1_extract_signature_data_invalid_length() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let invalid_sig_data = Bytes::from_array(&e, &[1u8; 120]); // Wrong length

        Secp256k1Verifier::extract_signature_data(&e, &invalid_sig_data);
    });
}

#[test]
fn secp256k1_expected_sig_data_len() {
    assert_eq!(Secp256k1Verifier::expected_sig_data_len(), 133); // 65 + 64 + 4
}

#[test]
fn secp256k1_recovery_id_extraction() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let public_key = BytesN::<65>::from_array(&e, &[5u8; 65]);
        let signature = BytesN::<64>::from_array(&e, &[6u8; 64]);
        let recovery_id = 0x12345678u32; // Test specific recovery ID

        let mut sig_data = Bytes::new(&e);
        sig_data.append(&public_key.into());
        sig_data.append(&signature.into());
        sig_data.extend_from_array(&recovery_id.to_be_bytes());

        let extracted_data = Secp256k1Verifier::extract_signature_data(&e, &sig_data);

        assert_eq!(extracted_data.recovery_id, recovery_id);
    });
}

#[test]
fn universal_key_management() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let public_key = Bytes::from_array(&e, &[1u8; 32]);

        // Initially not allowed
        assert!(!is_key_universally_allowed(&e, &public_key));

        // Allow key
        allow_key(&e, &public_key);
        assert!(is_key_universally_allowed(&e, &public_key));

        // Remove key
        remove_key(&e, &public_key);
        assert!(!is_key_universally_allowed(&e, &public_key));
    });
}

#[test]
fn topic_specific_key_management() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let public_key = Bytes::from_array(&e, &[2u8; 32]);
        let topic = 42u32;

        // Initially not allowed
        assert!(!is_key_allowed_for_topic(&e, &public_key, topic));

        // Allow key for topic
        allow_key_for_claim_topic(&e, &public_key, topic);
        assert!(is_key_allowed_for_topic(&e, &public_key, topic));

        // Should not be allowed for different topic
        assert!(!is_key_allowed_for_topic(&e, &public_key, topic + 1));

        // Remove key for topic
        remove_key_for_claim_topic(&e, &public_key, topic);
        assert!(!is_key_allowed_for_topic(&e, &public_key, topic));
    });
}

#[test]
fn combined_key_authorization() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let universal_key = Bytes::from_array(&e, &[3u8; 32]);
        let topic_key = Bytes::from_array(&e, &[4u8; 32]);
        let topic = 123u32;

        allow_key(&e, &universal_key);

        allow_key_for_claim_topic(&e, &topic_key, topic);

        // Universal key should be allowed for any topic
        assert!(is_key_allowed(&e, &universal_key, topic));
        assert!(is_key_allowed(&e, &universal_key, topic + 1));

        // Topic-specific key should only be allowed for its topic
        assert!(is_key_allowed(&e, &topic_key, topic));
        assert!(!is_key_allowed(&e, &topic_key, topic + 1));

        // Unknown key should not be allowed
        let unknown_key = Bytes::from_array(&e, &[5u8; 32]);
        assert!(!is_key_allowed(&e, &unknown_key, topic));
    });
}

#[test]
fn multiple_topics_same_key() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let public_key = Bytes::from_array(&e, &[6u8; 32]);
        let topic1 = 100u32;
        let topic2 = 200u32;
        let topic3 = 300u32;

        allow_key_for_claim_topic(&e, &public_key, topic1);
        allow_key_for_claim_topic(&e, &public_key, topic2);

        // Should be allowed for both topics
        assert!(is_key_allowed_for_topic(&e, &public_key, topic1));
        assert!(is_key_allowed_for_topic(&e, &public_key, topic2));
        assert!(!is_key_allowed_for_topic(&e, &public_key, topic3));

        // Remove one topic
        remove_key_for_claim_topic(&e, &public_key, topic1);

        // Should still be allowed for topic2 but not topic1
        assert!(!is_key_allowed_for_topic(&e, &public_key, topic1));
        assert!(is_key_allowed_for_topic(&e, &public_key, topic2));
    });
}

#[test]
fn universal_key_overrides_topic_specific() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let public_key = Bytes::from_array(&e, &[7u8; 32]);
        let topic = 500u32;

        // First allow for specific topic
        allow_key_for_claim_topic(&e, &public_key, topic);
        assert!(is_key_allowed(&e, &public_key, topic));
        assert!(!is_key_allowed(&e, &public_key, topic + 1));

        // Then allow universally
        allow_key(&e, &public_key);
        assert!(is_key_allowed(&e, &public_key, topic));
        assert!(is_key_allowed(&e, &public_key, topic + 1)); // Now allowed for any topic

        // Remove universal authorization
        remove_key(&e, &public_key);
        assert!(is_key_allowed(&e, &public_key, topic)); // Still allowed via topic-specific
        assert!(!is_key_allowed(&e, &public_key, topic + 1)); // No longer universal
    });
}

#[test]
fn signature_data_structures() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        // Test Ed25519 structure
        let ed25519_key = BytesN::<32>::from_array(&e, &[1u8; 32]);
        let ed25519_sig = BytesN::<64>::from_array(&e, &[2u8; 64]);
        let ed25519_data = Ed25519SignatureData {
            public_key: ed25519_key.clone(),
            signature: ed25519_sig.clone(),
        };
        assert_eq!(ed25519_data.public_key, ed25519_key);
        assert_eq!(ed25519_data.signature, ed25519_sig);

        // Test Secp256r1 structure
        let secp256r1_key = BytesN::<65>::from_array(&e, &[3u8; 65]);
        let secp256r1_sig = BytesN::<64>::from_array(&e, &[4u8; 64]);
        let secp256r1_data = Secp256r1SignatureData {
            public_key: secp256r1_key.clone(),
            signature: secp256r1_sig.clone(),
        };
        assert_eq!(secp256r1_data.public_key, secp256r1_key);
        assert_eq!(secp256r1_data.signature, secp256r1_sig);

        // Test Secp256k1 structure
        let secp256k1_key = BytesN::<65>::from_array(&e, &[5u8; 65]);
        let secp256k1_sig = BytesN::<64>::from_array(&e, &[6u8; 64]);
        let recovery_id = 42u32;
        let secp256k1_data = Secp256k1SignatureData {
            public_key: secp256k1_key.clone(),
            signature: secp256k1_sig.clone(),
            recovery_id,
        };
        assert_eq!(secp256k1_data.public_key, secp256k1_key);
        assert_eq!(secp256k1_data.signature, secp256k1_sig);
        assert_eq!(secp256k1_data.recovery_id, recovery_id);
    });
}

#[test]
fn ed25519_verify_claim_with_data_message_building() {
    let e = Env::default();

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    let secret_key: [u8; SECRET_KEY_LENGTH] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    let signing_key = SigningKey::from_bytes(&secret_key);
    let verifying_key = signing_key.verifying_key();

    let public_key: BytesN<32> = BytesN::from_array(&e, verifying_key.as_bytes());

    let mut data = identity.clone().to_xdr(&e);
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(&claim_data);
    let digest = e.crypto().keccak256(&data).to_array();

    let sig = signing_key.sign(&digest).to_bytes();
    let signature = BytesN::<64>::from_array(&e, &sig);

    let signature_data = Ed25519SignatureData { public_key, signature };

    assert!(Ed25519Verifier::verify_claim_with_data(
        &e,
        &identity,
        claim_topic,
        &claim_data,
        &signature_data,
    ))
}
