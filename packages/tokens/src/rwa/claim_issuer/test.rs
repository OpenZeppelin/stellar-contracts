extern crate std;
use ed25519_dalek::{Signer as Ed25519Signer, SigningKey};
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
use soroban_sdk::{contract, testutils::Address as _, Address, Bytes, BytesN, Env};

use crate::rwa::claim_issuer::{
    storage::{
        allow_key, is_claim_revoked, is_key_allowed, remove_key, set_claim_revoked,
        Ed25519SignatureData, Ed25519Verifier, Secp256k1SignatureData, Secp256k1Verifier,
        Secp256r1SignatureData, Secp256r1Verifier,
    },
    SignatureVerifier,
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
fn topic_specific_key_management() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let public_key = Bytes::from_array(&e, &[2u8; 32]);
    let topic = 42u32;

    e.as_contract(&contract_id, || {
        assert!(!is_key_allowed(&e, &public_key, 1, topic));

        allow_key(&e, &public_key, 1, topic);
        assert!(is_key_allowed(&e, &public_key, 1, topic));

        // check for different topic
        assert!(!is_key_allowed(&e, &public_key, 1, topic + 1));
        // check for different scheme
        assert!(!is_key_allowed(&e, &public_key, 2, topic));

        remove_key(&e, &public_key, 1, topic);
        assert!(!is_key_allowed(&e, &public_key, 1, topic));
    });
}

#[test]
fn ed25519_verify_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    let secret_key: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    let signing_key = SigningKey::from_bytes(&secret_key);
    let verifying_key = signing_key.verifying_key();

    let public_key: BytesN<32> = BytesN::from_array(&e, verifying_key.as_bytes());

    e.as_contract(&contract_id, || {
        // Build message using the verifier
        let message = Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data);

        // Convert message to buffer for signing
        let message_buf = message.to_buffer::<256>();
        let message_slice = &message_buf.as_slice()[..message.len() as usize];
        let sig = signing_key.sign(message_slice).to_bytes();
        let signature = BytesN::<64>::from_array(&e, &sig);

        let signature_data = Ed25519SignatureData { public_key, signature };

        assert!(Ed25519Verifier::verify(&e, &message, &signature_data))
    });
}

#[test]
fn secp256k1_verify_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

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

    e.as_contract(&contract_id, || {
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
    });
}

#[test]
fn secp256r1_verify_success() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

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

    e.as_contract(&contract_id, || {
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
    });
}

#[test]
fn signature_verifier_different_inputs_different_messages() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    let identity1 = Address::generate(&e);
    let identity2 = Address::generate(&e);
    let claim_data = Bytes::from_array(&e, &[1, 2, 3]);

    // Different identities should produce different messages
    e.as_contract(&contract_id, || {
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
    });
}

#[test]
fn signature_verifier_build_claim_digest_different_issuers() {
    let e = Env::default();
    let contract_id1 = e.register(MockContract, ());
    let contract_id2 = e.register(MockContract, ());

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    // Use same data on different claim issuers
    let ed25519_digest_1 = e.as_contract(&contract_id1, || {
        Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data)
    });
    let ed25519_digest_2 = e.as_contract(&contract_id2, || {
        Ed25519Verifier::build_message(&e, &identity, claim_topic, &claim_data)
    });
    assert_ne!(ed25519_digest_1, ed25519_digest_2);
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
#[should_panic(expected = "Error(Contract, #352)")]
fn allow_key_already_allowed() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let public_key = Bytes::from_array(&e, &[7u8; 32]);
    let topic = 999u32;

    e.as_contract(&contract_id, || {
        // Add key first time
        allow_key(&e, &public_key, 1, topic);

        // Try to add same key again - should panic with KeyAlreadyAllowed
        allow_key(&e, &public_key, 1, topic);
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
        // Try to remove non-existent key - should panic with KeyNotFound
        remove_key(&e, &public_key, 1, topic);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #351)")]
fn empty_key_panics() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let empty_key = Bytes::new(&e);
    let topic = 123u32;

    e.as_contract(&contract_id, || {
        // Test with empty key
        allow_key(&e, &empty_key, 1, topic);
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
