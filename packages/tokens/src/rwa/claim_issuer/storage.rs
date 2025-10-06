//! ## Signing Keys Management
//!
//! This module maintains two correlated storage branches:
//! - Registry branch: `Registries(SigningKey) -> Vec<Address>` Tracks at which
//!   `claim_topics_and_issuers` registries a given signing key (public key +
//!   scheme) is assigned.
//! - Topic branch: `Topics(u32) -> Vec<SigningKey>` Tracks which signing keys
//!   are assigned to a specific claim topic.
//!
//! ```text
//!                          ┌─────────────────────────┐
//!                          │ SigningKey              │
//!                          │ (public_key + scheme)   │
//!                          └─────────────────────────┘
//!                                      │
//!                    ┌─────────────────┴─────────────────┐
//!                    │                                   │
//!                    ▼                                   ▼
//!        ┌────────────────────────┐          ┌──────────────────────┐
//!        │ Registries(SigningKey) │          │ Topics(claim_topic)  │
//!        │ -> Vec<Address>        │          │ -> Vec<SigningKey>   │
//!        └────────────────────────┘          └──────────────────────┘
//!                    │                                   │
//!                    ▼                                   ▼
//!     [registry_1, registry_2, ...]         [key_1, key_2, ...]
//! ```
//!
//! ## Key Properties
//!
//! 1. **Atomic updates**: Although the branches are separate, they are updated
//!    atomically by `allow_key()`/`remove_key()`. A signing key cannot be
//!    assigned to a topic without also being assigned to at least one registry.
//!    When the last registry assignment for a key is removed, the key is
//!    automatically removed from the topic branch as well.
//!
//! 2. **Rationale for separation**: During claim verification, the claim
//!    issuer's internal flow typically needs to check only the topic branch
//!    using `is_key_allowed_for_topic()` to confirm that a given signing key is
//!    assigned to the claim topic in this contract. This design avoids
//!    redundant cross-contract calls. When the identity verifier calls the
//!    claim issuer, it is assumed to pass a valid topic and only invoke the
//!    claim issuer if it is registered as a trusted issuer in the
//!    `claim_topics_and_issuers` contract. Therefore, the claim issuer only
//!    needs to verify internally that the signing key is still allowed for the
//!    topic.
//!
//! 3. **Synchronization note**: After initial key assignment, the
//!    `claim_topics_and_issuers` contract may invalidate a topic or remove the
//!    `is_key_authorized()` to verify both the topic validity and issuer
//!    registration status.
//!
//! ## Claim Revocation and Signature Invalidation
//!
//! This module provides two independent mechanisms for invalidating claims:
//!
//! 1. **Per-claim revocation** (`set_claim_revoked`): Revokes a specific claim
//!    by storing its revocation status under the claim's digest. This allows
//!    fine-grained control over individual claims.
//!
//! 2. **Signature invalidation** (`invalidate_claim_signatures`): Invalidates
//!    all existing claim signatures by incrementing the nonce. This is
//!    efficient for invalidating multiple signatures at once without storing
//!    individual revocation entries.
//!
//! A nonce is included by default in every claim message (see
//! `build_claim_message`) to enable signature invalidation. The message format
//! is: network_id || claim_issuer || identity || claim_topic || nonce ||
//! claim_data
use core::ops::RangeBounds;

use soroban_sdk::{contracttype, panic_with_error, xdr::ToXdr, Address, Bytes, BytesN, Env, Vec};

use crate::rwa::{
    claim_issuer::{
        emit_key_allowed, emit_key_removed, emit_revocation_event, emit_signatures_invalidated,
        ClaimIssuerError, SignatureVerifier, CLAIMS_EXTEND_AMOUNT, CLAIMS_TTL_THRESHOLD,
        KEYS_EXTEND_AMOUNT, KEYS_TTL_THRESHOLD, MAX_KEYS_PER_TOPIC, MAX_REGISTRIES_PER_KEY,
    },
    claim_topics_and_issuers::ClaimTopicsAndIssuersClient,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SigningKey {
    pub public_key: Bytes,
    pub scheme: u32,
}

/// Storage keys for claim issuer key management.
#[contracttype]
#[derive(Clone)]
pub enum ClaimIssuerStorageKey {
    /// topic -> Vec<SigningKey>
    Topics(u32),
    /// SigningKey -> Vec<registry>
    Registries(SigningKey),
    /// Tracks explicitly revoked claims by claim digest
    RevokedClaim(BytesN<32>),
    /// Tracks current nonce for a specific identity and claim topics
    ClaimNonce(Address, u32),
}

/// Signature data for Ed25519 scheme.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ed25519SignatureData {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

/// Signature data for Secp256r1 scheme.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Secp256r1SignatureData {
    pub public_key: BytesN<65>,
    pub signature: BytesN<64>,
}

/// Signature data for Secp256k1 scheme.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Secp256k1SignatureData {
    pub public_key: BytesN<65>,
    pub signature: BytesN<64>,
    pub recovery_id: u32,
}

// ====================== SIGNATURE VERIFICATION =====================

/// Ed25519 signature verifier.
///
/// Expected signature data format: public_key (32 bytes) || signature (64
/// bytes)
pub struct Ed25519Verifier;

impl SignatureVerifier<32> for Ed25519Verifier {
    type SignatureData = Ed25519SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<32> = extract_from_bytes(e, sig_data, 0..32);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 32..96);

        Ed25519SignatureData { public_key, signature }
    }

    fn build_message(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> Bytes {
        build_claim_message(e, identity, claim_topic, claim_data)
    }

    fn verify(e: &Env, message: &Bytes, signature_data: &Self::SignatureData) -> bool {
        e.crypto().ed25519_verify(&signature_data.public_key, message, &signature_data.signature);
        true
    }

    fn expected_sig_data_len() -> u32 {
        96 // 32 bytes public key + 64 bytes signature
    }
}

/// Secp256r1 signature verifier.
///
/// Expected signature data format: public_key (65 bytes) || signature (64
/// bytes)
pub struct Secp256r1Verifier;

impl SignatureVerifier<32> for Secp256r1Verifier {
    type SignatureData = Secp256r1SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);

        Secp256r1SignatureData { public_key, signature }
    }

    fn build_message(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> Bytes {
        build_claim_message(e, identity, claim_topic, claim_data)
    }

    fn verify(e: &Env, message: &Bytes, signature_data: &Self::SignatureData) -> bool {
        // For Secp256r1, use the claim digest directly
        let claim_digest = e.crypto().sha256(message);
        e.crypto().secp256r1_verify(
            &signature_data.public_key,
            &claim_digest,
            &signature_data.signature,
        );
        true
    }

    fn expected_sig_data_len() -> u32 {
        129 // 65 bytes public key + 64 bytes signature
    }
}

/// Secp256k1 signature verifier.
///
/// Expected signature data format: public_key (65 bytes) || signature (64
/// bytes) || recovery_id (4 bytes)
pub struct Secp256k1Verifier;

impl SignatureVerifier<32> for Secp256k1Verifier {
    type SignatureData = Secp256k1SignatureData;

    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData {
        if sig_data.len() != Self::expected_sig_data_len() {
            panic_with_error!(e, ClaimIssuerError::SigDataMismatch)
        }

        let public_key: BytesN<65> = extract_from_bytes(e, sig_data, 0..65);
        let signature: BytesN<64> = extract_from_bytes(e, sig_data, 65..129);

        // Extract recovery_id from the last 4 bytes
        let recovery_id_bytes = sig_data.slice(129..133);
        let recovery_id = u32::from_be_bytes([
            recovery_id_bytes.get(0).unwrap_or(0),
            recovery_id_bytes.get(1).unwrap_or(0),
            recovery_id_bytes.get(2).unwrap_or(0),
            recovery_id_bytes.get(3).unwrap_or(0),
        ]);

        Secp256k1SignatureData { public_key, signature, recovery_id }
    }

    fn build_message(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> Bytes {
        build_claim_message(e, identity, claim_topic, claim_data)
    }

    fn verify(e: &Env, message: &Bytes, signature_data: &Self::SignatureData) -> bool {
        // For Secp256k1, recover public key and compare
        let claim_digest = e.crypto().keccak256(message);
        let recovered_key = e.crypto().secp256k1_recover(
            &claim_digest,
            &signature_data.signature,
            signature_data.recovery_id,
        );
        signature_data.public_key == recovered_key
    }

    fn expected_sig_data_len() -> u32 {
        // 65 bytes public key + 64 bytes signature + 4 bytes recovery_id;
        //
        // `recovery_id` usually fits in a single byte, but the argument in
        // `secp256k1_recover` is u32, that's why expecting here 4 bytes
        133
    }
}

// ====================== KEY MANAGEMENT =====================

/// Returns all signing keys assigned to a specific claim topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_topic` - The claim topic to get signing keys for.
///
/// # Errors
///
/// * [`ClaimIssuerError::NoKeysForTopic`] - If no signing keys are found for
///   the specified claim topic.
pub fn get_keys_for_topic(e: &Env, claim_topic: u32) -> Vec<SigningKey> {
    let topics_storage_key = ClaimIssuerStorageKey::Topics(claim_topic);

    e.storage()
        .persistent()
        .get::<_, Vec<SigningKey>>(&topics_storage_key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &topics_storage_key,
                KEYS_TTL_THRESHOLD,
                KEYS_EXTEND_AMOUNT,
            );
        })
        .unwrap_or_else(|| panic_with_error!(e, ClaimIssuerError::NoKeysForTopic))
}

/// Returns all registries associated with a specific signing key.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `signing_key` - The signing key to get registries for.
///
/// # Errors
///
/// * [`ClaimIssuerError::KeyNotFound`] - If the key is not found for this
///   topic.
pub fn get_registries(e: &Env, signing_key: &SigningKey) -> Vec<Address> {
    let registries_storage_key = ClaimIssuerStorageKey::Registries(signing_key.clone());

    e.storage()
        .persistent()
        .get::<_, Vec<Address>>(&registries_storage_key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &registries_storage_key,
                KEYS_TTL_THRESHOLD,
                KEYS_EXTEND_AMOUNT,
            );
        })
        .unwrap_or_else(|| panic_with_error!(e, ClaimIssuerError::KeyNotFound))
}

/// Checks if a public key and its scheme are allowed to sign claims for a
/// specific topic.
///
/// This function is a helper meant to be used within the `is_claim_valid` flow
/// (`identity_verifier` -> `claim_issuer`). It only checks whether the given
/// signing key (public key + scheme) is authorized for the provided
/// `claim_topic` in this contract's storage.
///
/// It does not:
/// - validate that `claim_topic` is registered in the
///   `claim_topics_and_issuers` contract, or
/// - verify that this contract is a trusted issuer.
///
/// These validations are expected to be performed by the calling identity
/// verifier prior to invoking this helper.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to check.
/// * `scheme` - The signature scheme used.
/// * `claim_topic` - The claim topic to check authorization for.
pub fn is_key_allowed_for_topic(
    e: &Env,
    public_key: &Bytes,
    scheme: u32,
    claim_topic: u32,
) -> bool {
    let topics_storage_key = ClaimIssuerStorageKey::Topics(claim_topic);

    if let Some(topic_keys) =
        e.storage().persistent().get::<_, Vec<SigningKey>>(&topics_storage_key)
    {
        e.storage().persistent().extend_ttl(
            &topics_storage_key,
            KEYS_TTL_THRESHOLD,
            KEYS_EXTEND_AMOUNT,
        );
        return topic_keys.iter().any(|key| key.public_key == *public_key && key.scheme == scheme);
    }

    false
}

/// Checks if a public key and its scheme are assigned to a given
/// `claim_topics_and_issuers` registry (regardless of topic).
///
/// It does not verify that this contract is a trusted issuer, registered in the
/// `claim_topics_and_issuers` contract.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to check.
/// * `scheme` - The signature scheme used.
/// * `registry` - The registry address to check assignment for.
pub fn is_key_allowed_for_registry(
    e: &Env,
    public_key: &Bytes,
    scheme: u32,
    registry: &Address,
) -> bool {
    let signing_key = SigningKey { public_key: public_key.clone(), scheme };
    let registries_key = ClaimIssuerStorageKey::Registries(signing_key);

    if let Some(registries) = e.storage().persistent().get::<_, Vec<Address>>(&registries_key) {
        e.storage().persistent().extend_ttl(
            &registries_key,
            KEYS_TTL_THRESHOLD,
            KEYS_EXTEND_AMOUNT,
        );
        return registries.iter().any(|addr| addr == *registry);
    }

    false
}

/// Checks whether the current contract (claim issuer) is authorized at a given
/// `claim_topics_and_issuers` registry for a specific claim topic.
///
/// This helper verifies both conditions:
/// - the current contract is a trusted issuer in `registry`, and
/// - the current contract is allowed to sign claims for `claim_topic` in
///   `registry`.
///
/// It does not does not check signing key assignment.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `registry` - The registry address to check against.
/// * `claim_topic` - The claim topic to check authorization for.
pub fn is_key_authorized(e: &Env, registry: &Address, claim_topic: u32) -> bool {
    let registry_client = ClaimTopicsAndIssuersClient::new(e, registry);

    registry_client.is_trusted_issuer(&e.current_contract_address())
        && registry_client.has_claim_topic(&e.current_contract_address(), &claim_topic)
}

/// Allows a public key to sign claims for specific topic and
/// `claim_topics_and_issuers` registry.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to authorize.
/// * `registry` - The address of the `claim_topics_and_issuers` registry.
/// * `scheme` - The signature scheme used.
/// * `claim_topic` - The specific claim topic to authorize for.
///
/// # Errors
///
/// * [`ClaimIssuerError::KeyIsEmpty`] - If attempting to allow an empty key.
/// * [`ClaimIssuerError::IssuerNotRegistered`] - If this claim issuer is not
///   registered at the `claim_topics_and_issuers` registry.
/// * [`ClaimIssuerError::ClaimTopicNotAllowed`] - If this claim issuer is not
///   allowed to sign claims about the `claim_topic`.
/// * [`ClaimIssuerError::KeyAlreadyAllowed`] - If the key is already allowed
///   for this topic.
///
/// # Events
///
/// * topics - `["key_allowed", public_key: Bytes]`
/// * data - `[registry: Address, scheme: u32, claim_topic: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn allow_key(e: &Env, public_key: &Bytes, registry: &Address, scheme: u32, claim_topic: u32) {
    if public_key.is_empty() {
        panic_with_error!(e, ClaimIssuerError::KeyIsEmpty)
    }

    let registry_client = ClaimTopicsAndIssuersClient::new(e, registry);

    // Check claim issuer is registered at claim_topics_and_issuers registry
    if !registry_client.is_trusted_issuer(&e.current_contract_address()) {
        panic_with_error!(e, ClaimIssuerError::IssuerNotRegistered)
    }

    // Check claim issuer can sign claim about a specific topic
    if !registry_client.has_claim_topic(&e.current_contract_address(), &claim_topic) {
        panic_with_error!(e, ClaimIssuerError::ClaimTopicNotAllowed)
    }

    let signing_key = SigningKey { public_key: public_key.clone(), scheme };

    // Check if key already exists for this topic
    if !is_key_allowed_for_topic(e, &signing_key.public_key, scheme, claim_topic) {
        let key = ClaimIssuerStorageKey::Topics(claim_topic);
        let mut topic_keys: Vec<SigningKey> =
            e.storage().persistent().get(&key).unwrap_or(Vec::new(e));

        if topic_keys.len() >= MAX_KEYS_PER_TOPIC {
            panic_with_error!(e, ClaimIssuerError::MaxKeysPerTopicExceeded)
        }

        topic_keys.push_back(signing_key.clone());
        e.storage().persistent().set(&key, &topic_keys);
    }

    // Update Registries mapping: SigningKey -> Vec<Address>
    let registries_key = ClaimIssuerStorageKey::Registries(signing_key);
    let mut registries: Vec<Address> =
        e.storage().persistent().get(&registries_key).unwrap_or(Vec::new(e));

    // Check if this registry is already added for this key
    for existing_registry in registries.iter() {
        if existing_registry == *registry {
            panic_with_error!(e, ClaimIssuerError::KeyAlreadyAllowed)
        }
    }

    registries.push_back(registry.clone());

    if registries.len() >= MAX_REGISTRIES_PER_KEY {
        panic_with_error!(e, ClaimIssuerError::MaxRegistriesPerKeyExceeded)
    }

    e.storage().persistent().set(&registries_key, &registries);

    emit_key_allowed(e, public_key, registry, scheme, claim_topic);
}

/// Removes a public key's authorization for a specific claim topic and
/// registry.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `public_key` - The public key to remove authorization for.
/// * `registry` - The registry address to remove authorization for.
/// * `scheme` - The signature scheme used.
/// * `claim_topic` - The claim topic to remove authorization for.
///
/// # Errors
///
/// * [`ClaimIssuerError::KeyNotFound`] - If the key is not found for this topic
///   or registry.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn remove_key(e: &Env, public_key: &Bytes, registry: &Address, scheme: u32, claim_topic: u32) {
    let signing_key = SigningKey { public_key: public_key.clone(), scheme };

    // Remove registry from Registries mapping
    let registries_key = ClaimIssuerStorageKey::Registries(signing_key.clone());
    let mut registries: Vec<Address> = e
        .storage()
        .persistent()
        .get(&registries_key)
        .unwrap_or_else(|| panic_with_error!(e, ClaimIssuerError::KeyNotFound));

    // Find and remove the specific registry
    match registries.first_index_of(registry) {
        Some(pos) => registries.remove_unchecked(pos),
        None => panic_with_error!(e, ClaimIssuerError::KeyNotFound),
    }

    // Update or remove Registries mapping
    if registries.is_empty() {
        e.storage().persistent().remove(&registries_key);

        // If no more registries, also remove the key from Topics mapping
        let topics_storage_key = ClaimIssuerStorageKey::Topics(claim_topic);
        let mut topic_keys: Vec<SigningKey> = e
            .storage()
            .persistent()
            .get(&topics_storage_key)
            .expect("signing keys for claim topic must be present"); // user can't remove this
                                                                     // storage entry alone

        let pos = topic_keys.first_index_of(&signing_key).expect("key must be in topic keys");
        topic_keys.remove_unchecked(pos);

        if topic_keys.is_empty() {
            e.storage().persistent().remove(&topics_storage_key);
        } else {
            e.storage().persistent().set(&topics_storage_key, &topic_keys);
        }
    } else {
        e.storage().persistent().set(&registries_key, &registries);
    }

    emit_key_removed(e, public_key, registry, scheme, claim_topic);
}

// =========== CLAIM REVOCATION & SIGNATURE INVALIDATION ===========

/// Returns the current nonce for a specific identity and claim topic.
///
/// The nonce is included in every claim message built by
/// `build_claim_message()`. When the nonce is incremented via
/// `invalidate_claim_signatures()`, all previously signed claims become
/// invalid.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the nonce is for.
/// * `claim_topic` - The claim topic the nonce is for.
pub fn get_current_nonce_for(e: &Env, identity: &Address, claim_topic: u32) -> u32 {
    let nonce_key = ClaimIssuerStorageKey::ClaimNonce(identity.clone(), claim_topic);
    e.storage()
        .persistent()
        .get(&nonce_key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &nonce_key,
                CLAIMS_TTL_THRESHOLD,
                CLAIMS_EXTEND_AMOUNT,
            );
        })
        .unwrap_or(0)
}

/// Invalidates all claim signatures for an identity and topic by incrementing
/// the nonce.
///
/// This provides an efficient way to invalidate all existing claim signatures
/// without storing individual revocation entries. After calling this function,
/// the nonce is incremented, causing all previously signed claims to have
/// invalid signatures since they were computed with the old nonce.
///
/// New claims must be signed with the new nonce (obtained via
/// `get_current_nonce_for()` or by directly computing the message via
/// `build_claim_message()`) to be valid.
///
/// **Note**: This does NOT affect per-claim revocation status set via
/// `set_claim_revoked()`. Those revocations persist independently.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address to invalidate signatures for.
/// * `claim_topic` - The claim topic to invalidate signatures for.
///
/// # Events
///
/// * topics - `["signatures_invalidated", identity: Address, claim_topic: u32]`
/// * data - `[nonce: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn invalidate_claim_signatures(e: &Env, identity: &Address, claim_topic: u32) {
    let nonce_key = ClaimIssuerStorageKey::ClaimNonce(identity.clone(), claim_topic);
    let mut nonce: u32 = e.storage().persistent().get(&nonce_key).unwrap_or(0);

    emit_signatures_invalidated(e, identity, claim_topic, nonce);

    nonce += 1;
    e.storage().persistent().set(&nonce_key, &nonce);
}

/// Sets the revocation status for a single claim.
///
/// The claim is identified by hashing a nonce-independent identifier consisting
/// of: network_id || claim_issuer || identity || claim_topic || claim_data.
/// This ensures that revocation status persists even when the nonce changes.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim.
/// * `claim_data` - The claim data.
/// * `revoked` - Whether the claim should be marked as revoked.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should only
/// be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant security
/// risks as it could allow unauthorized modifications.
pub fn set_claim_revoked(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
    revoked: bool,
) {
    // Build a nonce-independent claim identifier for revocation tracking
    let claim_digest = e
        .crypto()
        .keccak256(&build_claim_identifier(e, identity, claim_topic, claim_data))
        .to_bytes();

    e.storage().persistent().set(&ClaimIssuerStorageKey::RevokedClaim(claim_digest), &revoked);

    emit_revocation_event(e, identity, claim_topic, claim_data, revoked);
}

/// Checks if a claim has been revoked.
///
/// The claim is identified by hashing a nonce-independent identifier consisting
/// of: network_id || claim_issuer || identity || claim_topic || claim_data.
/// This ensures that revocation status persists even when the nonce changes.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim.
/// * `claim_data` - The claim data.
pub fn is_claim_revoked(e: &Env, identity: &Address, claim_topic: u32, claim_data: &Bytes) -> bool {
    // Use the nonce-independent identifier for checking revocation
    let claim_digest = e
        .crypto()
        .keccak256(&build_claim_identifier(e, identity, claim_topic, claim_data))
        .to_bytes();

    let key = ClaimIssuerStorageKey::RevokedClaim(claim_digest);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(&key, CLAIMS_TTL_THRESHOLD, CLAIMS_EXTEND_AMOUNT)
        })
        .unwrap_or_default()
}

// ====================== HELPERS =====================

/// Builds and returns the message to verify for claim signature validation.
///
/// The message format is: network_id || claim_issuer || identity ||
/// claim_topic || nonce || claim_data
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim to validate.
/// * `claim_data` - The claim data to validate.
pub fn build_claim_message(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
) -> Bytes {
    let nonce = get_current_nonce_for(e, identity, claim_topic);

    let mut data = Bytes::from_array(e, &e.ledger().network_id().to_array());
    data.append(&e.current_contract_address().to_xdr(e));
    data.append(&identity.to_xdr(e));
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.extend_from_array(&nonce.to_be_bytes());
    data.append(claim_data);
    data
}

/// Builds a nonce-independent claim identifier for revocation tracking.
///
/// The identifier format is: network_id || claim_issuer || identity ||
/// claim_topic || claim_data (WITHOUT nonce)
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `identity` - The identity address the claim is about.
/// * `claim_topic` - The topic of the claim.
/// * `claim_data` - The claim data.
pub fn build_claim_identifier(
    e: &Env,
    identity: &Address,
    claim_topic: u32,
    claim_data: &Bytes,
) -> Bytes {
    let mut data = Bytes::from_array(e, &e.ledger().network_id().to_array());
    data.append(&e.current_contract_address().to_xdr(e));
    data.append(&identity.to_xdr(e));
    data.extend_from_array(&claim_topic.to_be_bytes());
    data.append(claim_data);
    data
}

/// Extracts and returns a fixed-size array as BytesN<N> from a Bytes object.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `data` - The Bytes object to extract from.
/// * `r` - The range of bytes to extract.
pub fn extract_from_bytes<const N: usize>(
    e: &Env,
    data: &Bytes,
    r: impl RangeBounds<u32>,
) -> BytesN<N> {
    let buf = data.slice(r).to_buffer::<N>();
    let src = buf.as_slice();
    let mut items = [0u8; N];
    items.copy_from_slice(src);
    BytesN::<N>::from_array(e, &items)
}
