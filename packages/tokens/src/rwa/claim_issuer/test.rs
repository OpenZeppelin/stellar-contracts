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
use soroban_sdk::{contract, testutils::Address as _, xdr::ToXdr, Address, Bytes, BytesN, Env};

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
        assert!(!is_key_allowed(&e, &public_key, topic));

        allow_key(&e, &public_key, topic);
        assert!(is_key_allowed(&e, &public_key, topic));

        assert!(!is_key_allowed(&e, &public_key, topic + 1));

        remove_key(&e, &public_key, topic);
        assert!(!is_key_allowed(&e, &public_key, topic));
    });
}

#[test]
fn multiple_topics_same_key() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let public_key = Bytes::from_array(&e, &[6u8; 32]);
    let topic1 = 100u32;
    let topic2 = 200u32;
    let topic3 = 300u32;

    e.as_contract(&contract_id, || {
        allow_key(&e, &public_key, topic1);
        allow_key(&e, &public_key, topic2);

        assert!(is_key_allowed(&e, &public_key, topic1));
        assert!(is_key_allowed(&e, &public_key, topic2));
        assert!(!is_key_allowed(&e, &public_key, topic3));

        remove_key(&e, &public_key, topic1);

        assert!(!is_key_allowed(&e, &public_key, topic1));
        assert!(is_key_allowed(&e, &public_key, topic2));
    });
}

#[test]
fn signature_data_structures() {
    let e = Env::default();
    // Test Ed25519 structure
    let ed25519_key = BytesN::<32>::from_array(&e, &[1u8; 32]);
    let ed25519_sig = BytesN::<64>::from_array(&e, &[2u8; 64]);
    let ed25519_data =
        Ed25519SignatureData { public_key: ed25519_key.clone(), signature: ed25519_sig.clone() };
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
}

#[test]
fn ed25519_verify_claim_message_building() {
    let e = Env::default();

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

    let mut data = identity.clone().to_xdr(&e);
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(&claim_data);
    let digest = e.crypto().keccak256(&data);

    let sig = signing_key.sign(&digest.to_array()).to_bytes();
    let signature = BytesN::<64>::from_array(&e, &sig);

    let signature_data = Ed25519SignatureData { public_key, signature };

    assert!(Ed25519Verifier::verify_claim_digest(&e, &digest, &signature_data,))
}

#[test]
fn secp256k1_verify_claim_digest_success() {
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

    let mut data = identity.clone().to_xdr(&e);
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(&claim_data);
    let digest = e.crypto().keccak256(&data);

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

    assert!(Secp256k1Verifier::verify_claim_digest(&e, &digest, &signature_data));
}

#[test]
fn secp256r1_verify_claim_digest_success() {
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

    let mut data = identity.clone().to_xdr(&e);
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(&claim_data);
    let digest = e.crypto().keccak256(&data);

    let signature: Secp256r1Signature = signing_key.sign_prehash(&digest.to_array()).unwrap();

    let sig_slice = signature.normalize_s().unwrap_or(signature).to_bytes();
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&sig_slice);

    let signature_data =
        Secp256r1SignatureData { public_key, signature: BytesN::from_array(&e, &sig) };

    assert!(Secp256r1Verifier::verify_claim_digest(&e, &digest, &signature_data));
}

#[test]
fn signature_verifier_build_claim_digest_consistency() {
    let e = Env::default();

    let identity = Address::generate(&e);
    let claim_topic = 42u32;
    let claim_data = Bytes::from_array(&e, &[1, 2, 3, 4, 5]);

    // All verifiers should produce the same digest for the same inputs
    let ed25519_digest =
        Ed25519Verifier::build_claim_digest(&e, &identity, claim_topic, &claim_data).to_bytes();
    let secp256r1_digest =
        Secp256r1Verifier::build_claim_digest(&e, &identity, claim_topic, &claim_data).to_bytes();
    let secp256k1_digest =
        Secp256k1Verifier::build_claim_digest(&e, &identity, claim_topic, &claim_data).to_bytes();

    assert_eq!(ed25519_digest, secp256r1_digest);
    assert_eq!(secp256r1_digest, secp256k1_digest);
}

#[test]
fn signature_verifier_different_inputs_different_digests() {
    let e = Env::default();

    let identity1 = Address::generate(&e);
    let identity2 = Address::generate(&e);
    let claim_data = Bytes::from_array(&e, &[1, 2, 3]);

    // Different identities should produce different digests
    let digest1 = Ed25519Verifier::build_claim_digest(&e, &identity1, 1u32, &claim_data).to_bytes();
    let digest2 = Ed25519Verifier::build_claim_digest(&e, &identity2, 1u32, &claim_data).to_bytes();
    assert_ne!(digest1, digest2);

    // Different topics should produce different digests
    let digest3 = Ed25519Verifier::build_claim_digest(&e, &identity1, 2u32, &claim_data).to_bytes();
    assert_ne!(digest1, digest3);

    // Different data should produce different digests
    let different_data = Bytes::from_array(&e, &[4, 5, 6]);
    let digest4 =
        Ed25519Verifier::build_claim_digest(&e, &identity1, 1u32, &different_data).to_bytes();
    assert_ne!(digest1, digest4);
}

#[test]
fn set_and_check_claim_revocation() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let test_digest = e.crypto().keccak256(&Bytes::from_array(&e, &[1, 2, 3, 4]));

    e.as_contract(&contract_id, || {
        assert!(!is_claim_revoked(&e, &test_digest));

        set_claim_revoked(&e, &test_digest, true);

        assert!(is_claim_revoked(&e, &test_digest));
    });
}

#[test]
fn unrevoke_claim() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let test_digest = e.crypto().keccak256(&Bytes::from_array(&e, &[5, 6, 7, 8]));

    e.as_contract(&contract_id, || {
        set_claim_revoked(&e, &test_digest, true);
        assert!(is_claim_revoked(&e, &test_digest));

        set_claim_revoked(&e, &test_digest, false);
        assert!(!is_claim_revoked(&e, &test_digest));
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
        allow_key(&e, &public_key, topic);

        // Try to add same key again - should panic with KeyAlreadyAllowed
        allow_key(&e, &public_key, topic);
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
        remove_key(&e, &public_key, topic);
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
        allow_key(&e, &empty_key, topic);
    });
}

#[test]
fn revocation_edge_cases() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        // Test with different digest types
        let digest1 = e.crypto().keccak256(&Bytes::new(&e));
        let digest2 = e.crypto().keccak256(&Bytes::from_array(&e, &[0u8; 1000]));

        // Test revoking non-existent claim
        assert!(!is_claim_revoked(&e, &digest1));

        // Test setting revocation multiple times
        set_claim_revoked(&e, &digest1, true);
        set_claim_revoked(&e, &digest1, true); // Should not error
        assert!(is_claim_revoked(&e, &digest1));

        // Test unrevoking non-revoked claim
        set_claim_revoked(&e, &digest2, false);
        assert!(!is_claim_revoked(&e, &digest2));

        // Test multiple digests independently
        set_claim_revoked(&e, &digest2, true);
        assert!(is_claim_revoked(&e, &digest1));
        assert!(is_claim_revoked(&e, &digest2));

        set_claim_revoked(&e, &digest1, false);
        assert!(!is_claim_revoked(&e, &digest1));
        assert!(is_claim_revoked(&e, &digest2));
    });
}
