use ed25519_dalek::{Signer as Ed25519Signer, SigningKey};
use soroban_sdk::{contract, xdr::ToXdr, Bytes, BytesN, Env};

use crate::verifiers::ed25519::verify;

#[contract]
struct MockContract;

#[test]
fn ed25519_verify_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let secret_key: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    let signing_key = SigningKey::from_bytes(&secret_key);
    let verifying_key = signing_key.verifying_key();

    let key_data: Bytes = Bytes::from_array(&e, verifying_key.as_bytes());

    let data = Bytes::from_array(&e, &[1u8; 64]);
    let signature_payload = e.crypto().keccak256(&data);

    let signature = signing_key.sign(&signature_payload.to_array()).to_bytes();
    let sig_data = Bytes::from_array(&e, &signature);

    e.as_contract(&address, || {
        assert!(verify(
            &e,
            Bytes::from_array(&e, &signature_payload.to_array()),
            key_data,
            sig_data.to_xdr(&e)
        ))
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn ed25519_verify_key_data_invalid_too_small() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signature_payload = Bytes::from_array(&e, &[1u8; 32]);

        // Invalid key data - too small (should be 32 bytes)
        let key_data = Bytes::from_array(&e, &[1u8; 16]);

        let signature = BytesN::<64>::from_array(&e, &[2u8; 64]);
        let sig_data = signature.to_xdr(&e);

        verify(&e, signature_payload, key_data, sig_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn ed25519_verify_key_data_invalid_empty() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signature_payload = Bytes::from_array(&e, &[1u8; 32]);

        // Empty key data
        let key_data = Bytes::from_array(&e, &[]);

        let signature = BytesN::<64>::from_array(&e, &[2u8; 64]);
        let sig_data = signature.to_xdr(&e);

        verify(&e, signature_payload, key_data, sig_data);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn ed25519_verify_signature_format_invalid() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signature_payload = Bytes::from_array(&e, &[1u8; 32]);
        let key_data = Bytes::from_array(&e, &[1u8; 32]);

        // Invalid XDR data
        let invalid_sig_data = Bytes::from_array(&e, &[0xFF, 0xFE, 0xFD]);

        verify(&e, signature_payload, key_data, invalid_sig_data.to_xdr(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Crypto, InvalidInput)")]
fn ed25519_verify_invalid_signature() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let secret_key: [u8; 32] = [
            157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
            105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
        ];

        let signing_key = SigningKey::from_bytes(&secret_key);
        let verifying_key = signing_key.verifying_key();
        let key_data = Bytes::from_array(&e, verifying_key.as_bytes());

        let data = Bytes::from_array(&e, &[1u8; 64]);
        let signature_payload = e.crypto().keccak256(&data);

        // Create a valid signature but modify it to make it invalid
        let mut signature = signing_key.sign(&signature_payload.to_array()).to_bytes();
        signature[0] = signature[0].wrapping_add(1); // Corrupt the signature

        let sig_data = BytesN::<64>::from_array(&e, &signature).to_xdr(&e);

        verify(&e, Bytes::from_array(&e, &signature_payload.to_array()), key_data, sig_data);
    });
}

#[test]
#[should_panic(expected = "Error(Crypto, InvalidInput)")]
fn ed25519_verify_wrong_key() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let secret_key1: [u8; 32] = [
            157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
            105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
        ];
        let secret_key2: [u8; 32] = [
            200, 100, 150, 200, 240, 250, 95, 100, 190, 140, 80, 250, 150, 240, 50, 200, 70, 80,
            200, 110, 130, 55, 110, 30, 115, 65, 175, 10, 35, 180, 130, 100,
        ];

        let signing_key1 = SigningKey::from_bytes(&secret_key1);
        let signing_key2 = SigningKey::from_bytes(&secret_key2);

        // Use key1's public key but key2's signature
        let verifying_key1 = signing_key1.verifying_key();
        let key_data = Bytes::from_array(&e, verifying_key1.as_bytes());

        let data = Bytes::from_array(&e, &[1u8; 64]);
        let signature_payload = e.crypto().keccak256(&data);

        // Sign with key2 but verify with key1
        let signature = signing_key2.sign(&signature_payload.to_array()).to_bytes();
        let sig_data = BytesN::<64>::from_array(&e, &signature).to_xdr(&e);

        verify(&e, Bytes::from_array(&e, &signature_payload.to_array()), key_data, sig_data);
    });
}
