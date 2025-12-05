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
use soroban_sdk::{contracttype, xdr::ToXdr, Bytes, BytesN, Env};
use stellar_accounts::verifiers::{
    utils::base64_url_encode,
    webauthn::{
        WebAuthnSigData, AUTH_DATA_FLAGS_BE, AUTH_DATA_FLAGS_BS, AUTH_DATA_FLAGS_UP,
        AUTH_DATA_FLAGS_UV,
    },
};

use crate::contract::{WebauthnVerifierContract, WebauthnVerifierContractClient};

fn sign(e: &Env, digest: &BytesN<32>) -> (BytesN<65>, BytesN<64>) {
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

#[test]
fn verify_success() {
    let e = Env::default();
    let contract_id = e.register(WebauthnVerifierContract, ());
    let client = WebauthnVerifierContractClient::new(&e, &contract_id);

    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");
    let signature_payload = Bytes::from_array(&e, &payload);

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
    let (pub_key, signature) = sign(&e, &digest.into());

    let sig_data = WebAuthnSigData { client_data, authenticator_data, signature };
    let mut key_data = Bytes::from_array(&e, &pub_key.to_array());
    key_data.extend_from_array(&[1u8; 32]); // append credential id

    assert!(client.verify(&signature_payload, &key_data, &sig_data.to_xdr(&e)));
}

#[test]
#[should_panic(expected = "Error(Object, UnexpectedSize)")]
fn verify_sig_data_invalid_fails() {
    let e = Env::default();
    let contract_id = e.register(WebauthnVerifierContract, ());
    let client = WebauthnVerifierContractClient::new(&e, &contract_id);

    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");
    let signature_payload = Bytes::from_array(&e, &payload);

    let client_data =
        encode_client_data(&e, std::str::from_utf8(&[0u8; 43]).unwrap(), "webauthn.get");
    let authenticator_data = encode_authenticator_data(
        &e,
        AUTH_DATA_FLAGS_UP | AUTH_DATA_FLAGS_UV | AUTH_DATA_FLAGS_BE | AUTH_DATA_FLAGS_BS,
    );

    let mut msg = authenticator_data.clone();
    msg.extend_from_array(&e.crypto().sha256(&client_data).to_array());
    let digest = e.crypto().sha256(&msg);
    let (pub_key, _) = sign(&e, &digest.into());

    #[contracttype]
    struct InvalidWebAuthnSigData {
        // no signature
        pub authenticator_data: Bytes,
        pub client_data: Bytes,
    }

    let sig_data = InvalidWebAuthnSigData { client_data, authenticator_data };
    let key_data = Bytes::from_slice(&e, &pub_key.to_array());

    assert!(client.verify(&signature_payload, &key_data, &sig_data.to_xdr(&e)));
}

#[test]
#[should_panic(expected = "65-byte public key to be extracted")]
fn verify_key_data_invalid_fails() {
    let e = Env::default();
    let contract_id = e.register(WebauthnVerifierContract, ());
    let client = WebauthnVerifierContractClient::new(&e, &contract_id);

    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");
    let signature_payload = Bytes::from_array(&e, &payload);

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
    let (pub_key, signature) = sign(&e, &digest.into());

    let sig_data = WebAuthnSigData { client_data, authenticator_data, signature };
    let invalid_key_data = Bytes::from_slice(&e, &pub_key.to_array()[1..]);

    assert!(client.verify(&signature_payload, &invalid_key_data, &sig_data.to_xdr(&e)));
}

#[test]
#[should_panic(expected = "Error(Crypto, InvalidInput)")]
fn verify_invalid_signature() {
    let e = Env::default();
    let contract_id = e.register(WebauthnVerifierContract, ());
    let client = WebauthnVerifierContractClient::new(&e, &contract_id);

    let payload: [u8; 32] =
        hex!("4bb7a8b99609b0b8b1d534694bb1f31f129138a2f2a11f8e8702eedbb792922e");
    let signature_payload = Bytes::from_array(&e, &payload);

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
    let (pub_key, mut signature) = sign(&e, &digest.into());
    signature.set(0, 123);

    let sig_data = WebAuthnSigData { client_data, authenticator_data, signature };
    let key_data = Bytes::from_array(&e, &pub_key.to_array());

    client.verify(&signature_payload, &key_data, &sig_data.to_xdr(&e));
}
