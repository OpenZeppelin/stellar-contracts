extern crate std;

use hex_literal::hex;
use p256::{
    ecdsa::{
        signature::hazmat::PrehashSigner, Signature as Secp256r1Signature,
        SigningKey as Secp256r1SigningKey,
    },
    elliptic_curve::sec1::ToEncodedPoint,
    SecretKey as Secp256r1SecretKey,
};
use soroban_sdk::{contract, crypto::Hash, Bytes, BytesN, Env};

use crate::verifiers::{
    utils::base64_url_encode,
    webauthn::{
        validate_backup_eligibility_and_state, validate_challenge, validate_expected_type,
        validate_user_present_bit_set, validate_user_verified_bit_set, verify, ClientDataJson,
        WebAuthnSigData, AUTH_DATA_FLAGS_BE, AUTH_DATA_FLAGS_BS, AUTH_DATA_FLAGS_UP,
        AUTH_DATA_FLAGS_UV, CLIENT_DATA_MAX_LEN,
    },
};

fn sign(e: &Env, digest: Hash<32>) -> (BytesN<65>, BytesN<64>) {
    let secret_key_bytes: [u8; 32] = [
        33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63, 64,
    ];
    let secret_key = Secp256r1SecretKey::from_slice(&secret_key_bytes).unwrap();
    let signing_key = Secp256r1SigningKey::from(&secret_key);

    let pubkey = secret_key.public_key().to_encoded_point(false).to_bytes().to_vec();

    let mut pubkey_slice = [0u8; 65];
    pubkey_slice.copy_from_slice(&pubkey);
    let public_key = BytesN::<65>::from_array(e, &pubkey_slice);

    let signature: Secp256r1Signature = signing_key.sign_prehash(&digest.to_array()).unwrap();

    let sig_slice = signature.normalize_s().unwrap_or(signature).to_bytes();
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&sig_slice);
    let signature = BytesN::<64>::from_array(e, &sig);

    (public_key, signature)
}

fn encode_authenticator_data(e: &Env, flags: u8) -> Bytes {
    let mut data = [0u8; 37];
    data[32] = flags;
    Bytes::from_array(e, &data)
}

fn encode_client_data(e: &Env, challenge: &str, type_field: &str) -> Bytes {
    let json_str = std::format!(
        r#"{{
            "type": "{type_field}",
            "challenge": "{challenge}",
            "origin": "https://example.com",
            "crossOrigin": false
        }}"#
    );

    Bytes::from_slice(e, json_str.as_bytes())
}

#[contract]
struct MockContract;

#[test]
fn validate_expected_type_valid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let client_data_json =
            ClientDataJson { challenge: "test_challenge", type_field: "webauthn.get" };

        // Should not panic
        validate_expected_type(&e, &client_data_json);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3113)")]
fn validate_expected_type_invalid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let client_data_json = ClientDataJson {
            challenge: "test_challenge",
            type_field: "webauthn.create", // Wrong type
        };

        validate_expected_type(&e, &client_data_json);
    });
}

#[test]
fn validate_challenge_valid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let payload: [u8; 32] = [1; 32];
        let signature_payload = Bytes::from_array(&e, &payload);

        let mut encoded = [0u8; 43];
        base64_url_encode(&mut encoded, &payload);
        let challenge_str = std::str::from_utf8(&encoded).unwrap();

        let client_data_json =
            ClientDataJson { challenge: challenge_str, type_field: "webauthn.get" };

        // Should not panic
        validate_challenge(&e, &client_data_json, &signature_payload);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3114)")]
fn validate_challenge_invalid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let payload: [u8; 32] = [1; 32];
        let signature_payload = Bytes::from_array(&e, &payload);

        let client_data_json =
            ClientDataJson { challenge: "wrong_challenge", type_field: "webauthn.get" };

        validate_challenge(&e, &client_data_json, &signature_payload);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3110)")]
fn validate_challenge_invalid_payload_size() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let payload: [u8; 16] = [1; 16]; // Too small
        let signature_payload = Bytes::from_array(&e, &payload);

        let client_data_json =
            ClientDataJson { challenge: "test_challenge", type_field: "webauthn.get" };

        validate_challenge(&e, &client_data_json, &signature_payload);
    });
}

#[test]
fn validate_user_present_bit_set_valid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = AUTH_DATA_FLAGS_UP; // UP bit set

        // Should not panic
        validate_user_present_bit_set(&e, flags);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3116)")]
fn validate_user_present_bit_set_invalid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = 0u8; // UP bit not set

        validate_user_present_bit_set(&e, flags);
    });
}

// Tests for validate_user_verified_bit_set function
#[test]
fn validate_user_verified_bit_set_valid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = AUTH_DATA_FLAGS_UV; // UV bit set

        // Should not panic
        validate_user_verified_bit_set(&e, flags);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3117)")]
fn validate_user_verified_bit_set_invalid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = 0u8; // UV bit not set

        validate_user_verified_bit_set(&e, flags);
    });
}

// Tests for validate_backup_eligibility_and_state function
#[test]
fn validate_backup_eligibility_and_state_valid_be1_bs0() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = AUTH_DATA_FLAGS_BE; // BE=1, BS=0

        // Should not panic
        validate_backup_eligibility_and_state(&e, flags);
    });
}

#[test]
fn validate_backup_eligibility_and_state_valid_be1_bs1() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = AUTH_DATA_FLAGS_BE | AUTH_DATA_FLAGS_BS; // BE=1, BS=1

        // Should not panic
        validate_backup_eligibility_and_state(&e, flags);
    });
}

#[test]
fn validate_backup_eligibility_and_state_valid_be0_bs0() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = 0u8; // BE=0, BS=0

        // Should not panic
        validate_backup_eligibility_and_state(&e, flags);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3118)")]
fn validate_backup_eligibility_and_state_invalid_be0_bs1() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let flags = AUTH_DATA_FLAGS_BS; // BE=0, BS=1 (invalid state)

        validate_backup_eligibility_and_state(&e, flags);
    });
}

#[test]
fn webauthn_verify_success() {
    let e = Env::default();

    //"S7eouZYJsLix1TRpS7HzHxKROKLyoR-OhwLu27eSki4",
    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");

    let mut encoded = [0u8; 43];
    base64_url_encode(&mut encoded, &payload);

    let client_data =
        encode_client_data(&e, std::str::from_utf8(&encoded).unwrap(), "webauthn.get");
    let authenticator_data = encode_authenticator_data(
        &e,
        AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV | AUTH_DATA_FLAGS_BE | AUTH_DATA_FLAGS_BS,
    );

    let mut msg = authenticator_data.clone();
    msg.extend_from_array(&e.crypto().sha256(&client_data).to_array());
    let digest = e.crypto().sha256(&msg);
    let (key_data, signature) = sign(&e, digest);

    let sig_data = WebAuthnSigData { client_data, authenticator_data, signature };

    let signature_payload = Bytes::from_array(&e, &payload);

    let address = e.register(MockContract, ());
    e.as_contract(&address, || assert!(verify(&e, &signature_payload, &key_data, &sig_data)));
}

#[test]
#[should_panic(expected = "Error(Crypto, InvalidInput)")]
fn webauthn_verify_fake_signature_fails() {
    let e = Env::default();

    //"S7eouZYJsLix1TRpS7HzHxKROKLyoR-OhwLu27eSki4",
    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");

    let mut encoded = [0u8; 43];
    base64_url_encode(&mut encoded, &payload);

    let client_data =
        encode_client_data(&e, std::str::from_utf8(&encoded).unwrap(), "webauthn.get");
    let authenticator_data = encode_authenticator_data(
        &e,
        AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV | AUTH_DATA_FLAGS_BE | AUTH_DATA_FLAGS_BS,
    );

    let mut msg = authenticator_data.clone();
    msg.extend_from_array(&e.crypto().sha256(&client_data).to_array());
    let digest = e.crypto().sha256(&msg);
    let (key_data, mut signature) = sign(&e, digest);

    // modify signature
    signature.set(0, 123);

    let sig_data = WebAuthnSigData { client_data, authenticator_data, signature };

    let signature_payload = Bytes::from_array(&e, &payload);

    let address = e.register(MockContract, ());
    e.as_contract(&address, || assert!(verify(&e, &signature_payload, &key_data, &sig_data)));
}

#[test]
#[should_panic(expected = "Error(Contract, #3111)")]
fn verify_client_data_too_long() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let payload: [u8; 32] = [1; 32];
        let signature_payload = Bytes::from_array(&e, &payload);
        let key_data = BytesN::<65>::from_array(&e, &[1u8; 65]);

        // Create client data that exceeds CLIENT_DATA_MAX_LEN (1024 bytes)
        let large_origin = "x".repeat(CLIENT_DATA_MAX_LEN + 100);
        let json_str = std::format!(
            r#"{{"type": "webauthn.get", "challenge": "test", "origin": "{large_origin}", "crossOrigin": false}}"#
        );
        let large_client_data = Bytes::from_slice(&e, json_str.as_bytes());

        let authenticator_data = encode_authenticator_data(&e, AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV);
        let signature = BytesN::<64>::from_array(&e, &[2u8; 64]);

        let sig_data = WebAuthnSigData {
            client_data: large_client_data,
            authenticator_data,
            signature,
        };

        verify(&e, &signature_payload, &key_data, &sig_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3115)")]
fn verify_authenticator_data_too_short() {
    let e = Env::default();
    let key_data = BytesN::<65>::from_array(&e, &[1u8; 65]);
    let payload: [u8; 32] = [1; 32];
    let signature_payload = Bytes::from_array(&e, &payload);

    let mut encoded = [0u8; 43];
    base64_url_encode(&mut encoded, &payload);

    let client_data =
        encode_client_data(&e, std::str::from_utf8(&encoded).unwrap(), "webauthn.get");
    // Slice authenticator_data
    let authenticator_data =
        encode_authenticator_data(&e, AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV).slice(0..35);
    let signature = BytesN::<64>::from_array(&e, &[2u8; 64]);

    let sig_data = WebAuthnSigData { client_data, authenticator_data, signature };

    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        verify(&e, &signature_payload, &key_data, &sig_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3112)")]
fn verify_json_parse_error() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let payload: [u8; 32] = [1; 32];
        let signature_payload = Bytes::from_array(&e, &payload);
        let key_data = BytesN::<65>::from_array(&e, &[1u8; 65]);

        // Invalid JSON - missing closing brace
        let invalid_json = r#"{"type": "webauthn.get", "challenge": "test""#;
        let invalid_client_data = Bytes::from_slice(&e, invalid_json.as_bytes());

        let authenticator_data =
            encode_authenticator_data(&e, AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV);
        let signature = BytesN::<64>::from_array(&e, &[2u8; 64]);

        let sig_data =
            WebAuthnSigData { client_data: invalid_client_data, authenticator_data, signature };

        verify(&e, &signature_payload, &key_data, &sig_data);
    });
}
