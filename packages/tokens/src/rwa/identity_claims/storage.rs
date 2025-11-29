/// # Identity Claims Storage Implementation
///
/// This module provides storage functionality for on-chain identity claims,
/// implementing the ERC-XXXX OnChainIdentity standard adapted for Soroban.
///
/// ## Claim Structure
///
/// Claims are attestations made by issuers about specific topics related to an
/// identity. Each claim contains:
/// - **Topic**: A numeric identifier for the claim type
/// - **Scheme**: The signature scheme used for verification
/// - **Issuer**: The address that issued the claim
/// - **Signature**: Cryptographic proof of the claim
/// - **Data**: Can be a clear text string, the hash of some content, or empty.
/// - **URI**: Optional reference to additional information
///
/// ## Claim ID Generation
///
/// Claim IDs are generated using `keccak256(issuer || topic)` to ensure
/// uniqueness per issuer-topic pair, following the ERC standard.
///
/// ## Storage Layout
///
/// - Claims are stored by their unique ID
/// - Topic-based indexing allows efficient retrieval by claim type
use soroban_sdk::{
    contracttype, panic_with_error, vec, xdr::ToXdr, Address, Bytes, BytesN, Env, String, Vec,
};

use super::{
    ClaimEvent, ClaimsError, CLAIMS_EXTEND_AMOUNT, CLAIMS_TTL_THRESHOLD,
};

#[cfg(not(feature = "certora"))]
use super::{
    emit_claim_event,
};

use crate::rwa::claim_issuer::ClaimIssuerClient;

/// Represents a claim stored on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claim {
    /// The claim topic (numeric identifier)
    pub topic: u32,
    /// The signature scheme used
    pub scheme: u32,
    /// The address of the claim issuer
    pub issuer: Address,
    /// The cryptographic signature
    pub signature: Bytes,
    /// The claim data
    pub data: Bytes,
    /// Optional URI for additional information
    pub uri: String,
}

/// Storage keys for the data associated with Identity Claims.
#[contracttype]
pub enum ClaimsStorageKey {
    /// Maps claim ID to claim data
    Claim(BytesN<32>),
    /// Maps topic to vector of claim IDs
    ClaimsByTopic(u32),
}

/// Stores a new claim or updates an existing one.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `topic` - The claim topic.
/// * `scheme` - The signature scheme used.
/// * `issuer` - The address of the claim issuer.
/// * `signature` - The cryptographic signature of the claim.
/// * `data` - The claim data.
/// * `uri` - Optional URI for additional claim information.
///
/// # Events
///
/// * topics - `["claim_added", claim: Claim]` for new claims
/// * data - `[]`
///
/// OR:
///
/// * topics - `["claim_changed", claim: Claim]` for updates
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function should include authorization checks in
/// production. The current implementation allows any caller to add claims. In a
/// real deployment, you should verify that the caller is authorized to add
/// claims for this identity.
pub fn add_claim(
    e: &Env,
    topic: u32,
    scheme: u32,
    issuer: &Address,
    signature: &Bytes,
    data: &Bytes,
    uri: &String,
) -> BytesN<32> {
    let claim_issuer_client = ClaimIssuerClient::new(e, issuer);
    let identity = e.current_contract_address();

    claim_issuer_client.is_claim_valid(&identity, &topic, &scheme, signature, data);

    let claim_id = generate_claim_id(e, issuer, topic);

    let claim_key = ClaimsStorageKey::Claim(claim_id.clone());
    let is_new_claim = !e.storage().persistent().has(&claim_key);

    let claim = Claim {
        topic,
        scheme,
        issuer: issuer.clone(),
        signature: signature.clone(),
        data: data.clone(),
        uri: uri.clone(),
    };

    e.storage().persistent().set(&claim_key, &claim);

    // Emit appropriate event
    if is_new_claim {
        add_claim_to_topic_index(e, topic, &claim_id);
        #[cfg(not(feature = "certora"))]
        emit_claim_event(e, ClaimEvent::Added, claim);
    } else {
        #[cfg(not(feature = "certora"))]
        emit_claim_event(e, ClaimEvent::Changed, claim);
    }

    claim_id
}

/// Retrieves a claim by its ID.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_id` - The unique claim identifier.
///
/// # Errors
///
/// * [`ClaimsError::ClaimNotFound`] - If the claim ID does not exist.
pub fn get_claim(e: &Env, claim_id: &BytesN<32>) -> Claim {
    let key = ClaimsStorageKey::Claim(claim_id.clone());

    let claim: Claim = e
        .storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(&key, CLAIMS_TTL_THRESHOLD, CLAIMS_EXTEND_AMOUNT)
        })
        .unwrap_or_else(|| panic_with_error!(e, ClaimsError::ClaimNotFound));

    claim
}

/// Retrieves all claim IDs for a specific topic.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `topic` - The claim topic to filter by.
pub fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>> {
    let key = ClaimsStorageKey::ClaimsByTopic(topic);

    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(&key, CLAIMS_TTL_THRESHOLD, CLAIMS_EXTEND_AMOUNT)
        })
        .unwrap_or_else(|| vec![e])
}

/// Removes a claim by its ID. Although the interface does not specify a removal
/// method, it might be useful to dispose one in cases where it is
/// needed/allowed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_id` - The unique claim identifier to remove.
///
/// # Errors
///
/// * [`ClaimsError::ClaimNotFound`] - If the claim ID does not exist.
///
/// # Events
///
/// * topics - `["claim_removed", claim_id: BytesN<32>, topic: u32]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function should include proper authorization checks.
/// Only the claim issuer or identity owner should be able to remove claims.
pub fn remove_claim(e: &Env, claim_id: &BytesN<32>) {
    let claim_key = ClaimsStorageKey::Claim(claim_id.clone());

    // Get the claim to retrieve its topic before removal
    let claim: Claim = e
        .storage()
        .persistent()
        .get(&claim_key)
        .unwrap_or_else(|| panic_with_error!(e, ClaimsError::ClaimNotFound));

    e.storage().persistent().remove(&claim_key);

    remove_claim_from_topic_index(e, claim.topic, claim_id);
    #[cfg(not(feature = "certora"))]
    emit_claim_event(e, ClaimEvent::Removed, claim);
}

/// Low-level function to remove a claim ID from the topic index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `topic` - The claim topic.
/// * `claim_id` - The claim ID to remove.
///
/// # Security Warning
///
/// **IMPORTANT**: This function should include proper authorization checks.
/// Only the claim issuer or identity owner should be able to remove claims.
pub fn remove_claim_from_topic_index(e: &Env, topic: u32, claim_id: &BytesN<32>) {
    let key = ClaimsStorageKey::ClaimsByTopic(topic);
    let mut claim_ids: Vec<BytesN<32>> =
        e.storage().persistent().get(&key).unwrap_or_else(|| vec![e]);

    if let Some(index) = claim_ids.iter().position(|id| id == *claim_id) {
        claim_ids.remove(index as u32);

        if claim_ids.is_empty() {
            e.storage().persistent().remove(&key);
        } else {
            e.storage().persistent().set(&key, &claim_ids);
        }
    }
}

// ==================== HELPER FUNCTIONS ====================

/// Generates a unique claim ID using keccak256(issuer || topic).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `issuer` - The claim issuer address.
/// * `topic` - The claim topic.
pub fn generate_claim_id(e: &Env, issuer: &Address, topic: u32) -> BytesN<32> {
    // Create a bytes representation of issuer + topic for hashing

    let mut data = issuer.to_xdr(e);
    data.extend_from_array(&topic.to_be_bytes());

    e.crypto().keccak256(&data).to_bytes()
}

/// Adds a claim ID to the topic index.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `topic` - The claim topic.
/// * `claim_id` - The claim ID to add.
fn add_claim_to_topic_index(e: &Env, topic: u32, claim_id: &BytesN<32>) {
    let key = ClaimsStorageKey::ClaimsByTopic(topic);
    let mut claim_ids: Vec<BytesN<32>> =
        e.storage().persistent().get(&key).unwrap_or_else(|| vec![e]);

    claim_ids.push_back(claim_id.clone());

    e.storage().persistent().set(&key, &claim_ids);
}
