use soroban_sdk::{contracterror, panic_with_error, xdr::FromXdr, Bytes, BytesN, Env};

use crate::verifiers::utils::extract_from_bytes;

// TODO: proper enumeration
#[contracterror]
#[repr(u32)]
pub enum Ed25519Error {
    KeyDataInvalid = 0,
    SignatureFormatInvalid = 1,
}

pub type Ed25519SigData = BytesN<64>;

pub fn verify(e: &Env, signature_payload: Bytes, key_data: Bytes, sig_data: Bytes) -> bool {
    let public_key: BytesN<32> = extract_from_bytes(e, &key_data, 0..32)
        .unwrap_or_else(|| panic_with_error!(e, Ed25519Error::KeyDataInvalid));

    let signature: BytesN<64> = Ed25519SigData::from_xdr(e, &sig_data)
        .unwrap_or_else(|_| panic_with_error!(e, Ed25519Error::SignatureFormatInvalid));

    e.crypto().ed25519_verify(&public_key, &signature_payload, &signature);

    true
}
