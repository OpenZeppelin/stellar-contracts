use clap::{Parser, ValueEnum};
use ed25519_dalek::Signer;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
const MAINNET_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";

#[derive(Clone, ValueEnum)]
enum Network {
    Testnet,
    Mainnet,
}

#[derive(Parser)]
#[command(about = "Compute Ed25519 claim signatures for RWA identity claims")]
struct Args {
    /// Ed25519 secret key as a 64-char hex string
    #[arg(long)]
    secret_key: String,

    /// Deployed claim issuer contract address (C...)
    #[arg(long)]
    claim_issuer: String,

    /// Identity contract or account address (C... or G...)
    #[arg(long)]
    identity: String,

    /// Claim topic
    #[arg(long, default_value_t = 1)]
    claim_topic: u32,

    /// Nonce (query get_current_nonce_for on the claim issuer if greater than 0)
    #[arg(long, default_value_t = 0)]
    nonce: u32,

    /// Number of days the claim is valid for
    #[arg(long, default_value_t = 365)]
    valid_for_days: u32,

    /// Network to sign for
    #[arg(long, value_enum, default_value_t = Network::Testnet)]
    network: Network,
}

fn hex_decode(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "hex string must have even length");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("invalid hex character"))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// XDR-encodes a Stellar address the same way `Address::to_xdr(e)` does inside
/// Soroban.
///
/// Soroban's `to_xdr` serializes as `ScVal::Address(ScAddress(...))`, so the
/// encoding always starts with the `SCV_ADDRESS` type discriminant (18 =
/// `0x00000012`), followed by the `ScAddress` payload:
///
/// - Contract (`C…`): `[0x00,0x00,0x00,0x12]` (SCV_ADDRESS)
///                  || `[0x00,0x00,0x00,0x01]` (SC_ADDRESS_TYPE_CONTRACT)
///                  || 32-byte contract hash
/// - Account  (`G…`): `[0x00,0x00,0x00,0x12]` (SCV_ADDRESS)
///                  || `[0x00,0x00,0x00,0x00]` (SC_ADDRESS_TYPE_ACCOUNT)
///                  || `[0x00,0x00,0x00,0x00]` (PUBLIC_KEY_TYPE_ED25519)
///                  || 32-byte public key
fn address_to_xdr(address: &str) -> Vec<u8> {
    // SCV_ADDRESS = 18 (ScValType discriminant)
    const SCV_ADDRESS: [u8; 4] = [0x00, 0x00, 0x00, 0x12];

    if address.starts_with('C') {
        let contract =
            stellar_strkey::Contract::from_string(address).expect("invalid contract address");
        let mut xdr = Vec::with_capacity(40);
        xdr.extend_from_slice(&SCV_ADDRESS);
        xdr.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // SC_ADDRESS_TYPE_CONTRACT = 1
        xdr.extend_from_slice(&contract.0);
        xdr
    } else if address.starts_with('G') {
        let key = stellar_strkey::ed25519::PublicKey::from_string(address)
            .expect("invalid account address");
        let mut xdr = Vec::with_capacity(44);
        xdr.extend_from_slice(&SCV_ADDRESS);
        xdr.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // SC_ADDRESS_TYPE_ACCOUNT = 0
        xdr.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PUBLIC_KEY_TYPE_ED25519 = 0
        xdr.extend_from_slice(&key.0);
        xdr
    } else {
        panic!("unsupported address type: {address}");
    }
}

fn main() {
    let args = Args::parse();

    // Decode the secret key and construct the signing key.
    let secret_bytes = hex_decode(&args.secret_key);
    let secret_array: [u8; 32] =
        secret_bytes.try_into().expect("secret key must be exactly 32 bytes (64 hex chars)");
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_array);
    let public_key = signing_key.verifying_key().to_bytes();

    // XDR-encode both addresses.
    let issuer_xdr = address_to_xdr(&args.claim_issuer);
    let identity_xdr = address_to_xdr(&args.identity);

    // Compute network_id = SHA-256(network passphrase).
    let passphrase = match args.network {
        Network::Testnet => TESTNET_PASSPHRASE,
        Network::Mainnet => MAINNET_PASSPHRASE,
    };
    let network_id = Sha256::digest(passphrase.as_bytes());

    // Build claim_data = created_at (u64 BE, 8 B) || valid_until (u64 BE, 8 B).
    // Mirrors encode_claim_data_expiration in packages/tokens/src/rwa/claim_issuer/storage.rs.
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before UNIX epoch")
        .as_secs();
    let valid_until = created_at + u64::from(args.valid_for_days) * 86400;
    let mut claim_data = Vec::with_capacity(16);
    claim_data.extend_from_slice(&created_at.to_be_bytes());
    claim_data.extend_from_slice(&valid_until.to_be_bytes());

    // Build message = 0x01 || network_id || issuer_xdr || identity_xdr
    //               || claim_topic (u32 BE) || nonce (u32 BE) || claim_data.
    // Mirrors build_claim_message in packages/tokens/src/rwa/claim_issuer/storage.rs.
    let mut message = Vec::new();
    message.push(0x01u8);
    message.extend_from_slice(&network_id);
    message.extend_from_slice(&issuer_xdr);
    message.extend_from_slice(&identity_xdr);
    message.extend_from_slice(&args.claim_topic.to_be_bytes());
    message.extend_from_slice(&args.nonce.to_be_bytes());
    message.extend_from_slice(&claim_data);

    // Sign and build sig_data = public_key (32 B) || signature (64 B).
    let signature = signing_key.sign(&message).to_bytes();
    let mut sig_data = Vec::with_capacity(96);
    sig_data.extend_from_slice(&public_key);
    sig_data.extend_from_slice(&signature);

    println!("--data      {}", hex_encode(&claim_data));
    println!("--signature {}", hex_encode(&sig_data));
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known test addresses and their expected 32-byte payloads.
    const CONTRACT_ADDR: &str = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM";
    // GA5WUJ54Z23KILLCUOUNAKTPBVZWKMQVO4O6EQ5GHLAERIMLLHNCSKYH encodes the public key
    // 3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29 (well-known test key).
    const ACCOUNT_ADDR: &str = "GA5WUJ54Z23KILLCUOUNAKTPBVZWKMQVO4O6EQ5GHLAERIMLLHNCSKYH";

    #[test]
    fn address_to_xdr_contract_structure() {
        let xdr = address_to_xdr(CONTRACT_ADDR);
        let contract = stellar_strkey::Contract::from_string(CONTRACT_ADDR).unwrap();

        // Total length: 4 (SCV_ADDRESS) + 4 (type) + 32 (hash) = 40 bytes
        assert_eq!(xdr.len(), 40);
        // SCV_ADDRESS type discriminant = 18 = 0x12
        assert_eq!(&xdr[0..4], &[0x00, 0x00, 0x00, 0x12]);
        // SC_ADDRESS_TYPE_CONTRACT = 1
        assert_eq!(&xdr[4..8], &[0x00, 0x00, 0x00, 0x01]);
        // 32-byte payload matches strkey decode
        assert_eq!(&xdr[8..40], &contract.0);
    }

    #[test]
    fn address_to_xdr_account_structure() {
        let xdr = address_to_xdr(ACCOUNT_ADDR);
        let key = stellar_strkey::ed25519::PublicKey::from_string(ACCOUNT_ADDR).unwrap();

        // Total length: 4 (SCV_ADDRESS) + 4 (type) + 4 (key type) + 32 (key) = 44 bytes
        assert_eq!(xdr.len(), 44);
        // SCV_ADDRESS type discriminant = 18 = 0x12
        assert_eq!(&xdr[0..4], &[0x00, 0x00, 0x00, 0x12]);
        // SC_ADDRESS_TYPE_ACCOUNT = 0
        assert_eq!(&xdr[4..8], &[0x00, 0x00, 0x00, 0x00]);
        // PUBLIC_KEY_TYPE_ED25519 = 0
        assert_eq!(&xdr[8..12], &[0x00, 0x00, 0x00, 0x00]);
        // 32-byte payload matches strkey decode
        assert_eq!(&xdr[12..44], &key.0);
    }
}
