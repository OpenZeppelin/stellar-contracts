mod storage;
mod test;

use soroban_sdk::{contracterror, Address, Bytes, BytesN, Env, String, Symbol, Vec};

// TODO: export one by one
pub use storage::*;

/// Core trait for managing on-chain identity claims, based on ERC-XXXX OnChainIdentity.
///
/// This trait provides functionality for adding, retrieving, and managing claims
/// associated with an identity. Claims are attestations made by issuers about
/// specific topics related to the identity.
pub trait IdentityClaims {
    /// Adds a new claim to the identity or updates an existing one.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `topic` - The claim topic (u32 identifier).
    /// * `scheme` - The signature scheme used.
    /// * `issuer` - The address of the claim issuer.
    /// * `signature` - The cryptographic signature of the claim.
    /// * `data` - The claim data.
    /// * `uri` - Optional URI for additional claim information.
    ///
    /// # Returns
    ///
    /// Returns the unique claim ID (BytesN<32>).
    ///
    /// # Events
    ///
    /// * topics - `["claim_added", claim_id: BytesN<32>, topic: u32]`
    /// * data - `[]`
    ///
    /// OR (for updates):
    ///
    /// * topics - `["claim_changed", claim_id: BytesN<32>, topic: u32]`
    /// * data - `[]`
    fn add_claim(
        e: &Env,
        topic: u32,
        scheme: u32,
        issuer: Address,
        signature: Bytes,
        data: Bytes,
        uri: String,
    ) -> BytesN<32>;

    /// Retrieves a claim by its ID.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `claim_id` - The unique claim identifier.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing (topic, scheme, issuer, signature, data, uri).
    ///
    /// # Errors
    ///
    /// * [`ClaimsError::ClaimNotFound`] - If the claim ID does not exist.
    fn get_claim(e: &Env, claim_id: BytesN<32>) -> Claim;

    /// Retrieves all claim IDs for a specific topic.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `topic` - The claim topic to filter by.
    ///
    /// # Returns
    ///
    /// Returns a vector of claim IDs associated with the topic.
    fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>>;
}

/// Trait for validating claims issued by this identity to other identities.
pub trait ClaimIssuer {
    /// Validates whether a claim is valid for a given identity.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim to validate.
    /// * `signature` - The signature to validate.
    /// * `data` - The claim data to validate.
    ///
    /// # Returns
    ///
    /// Returns true if the claim is valid, false otherwise.
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        signature: Bytes,
        data: Bytes,
    ) -> bool;
}

// TODO: correct enumeration and move up to higher level
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimsError {
    ClaimNotFound = 1,
    InvalidSignature = 2,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const CLAIMS_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const CLAIMS_TTL_THRESHOLD: u32 = CLAIMS_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

pub enum ClaimEvent {
    Added,
    Removed,
    Changed,
}

/// Emits an event for a claim operation (add, remove, change).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim` - The claim on which was operated.
///
/// # Events
///
/// * topics - `["event_name": Symbol, claim: Claim]`
/// * data - `[]`
///
/// Where `event_name` is one of:
/// - `"claim_added"` for [`ClaimEvent::Added`]
/// - `"claim_removed"` for [`ClaimEvent::Removed`]
/// - `"claim_changed"` for [`ClaimEvent::Changed`]
pub fn emit_claim_event(e: &Env, event_type: ClaimEvent, claim: &Claim) {
    let name = match event_type {
        ClaimEvent::Added => Symbol::new(e, "claim_added"),
        ClaimEvent::Removed => Symbol::new(e, "claim_removed"),
        ClaimEvent::Changed => Symbol::new(e, "claim_changed"),
    };
    let event_topics = (name, claim.clone());
    e.events().publish(event_topics, ());
}
