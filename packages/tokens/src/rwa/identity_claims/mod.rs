mod storage;
#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{
    contractclient, contracterror, Address, Bytes, BytesN, Env, String, Vec,
};
pub use storage::{
    add_claim, generate_claim_id, get_claim, get_claim_ids_by_topic, remove_claim, Claim,
};

/// Core trait for managing on-chain identity claims, based on ERC-XXXX
/// OnChainIdentity.
///
/// This trait provides functionality for adding, retrieving, and managing
/// claims associated with an identity. Claims are attestations made by issuers
/// about specific topics related to the identity.
#[contractclient(name = "IdentityClaimsClient")]
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
    fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>>;
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimsError {
    /// Claim  ID does not exist.
    ClaimNotFound = 340,
    /// Claim Issuer cannot validate the claim (revocation, signature mismatch,
    /// unauthorized signing key, etc.)
    ClaimNotValid = 341,
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

/// Event emitted when a claim is added.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimAdded {
    #[topic]
    pub claim: Claim,
}

/// Event emitted when a claim is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRemoved {
    #[topic]
    pub claim: Claim,
}

/// Event emitted when a claim is changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimChanged {
    #[topic]
    pub claim: Claim,
}

/// Emits an event for a claim operation.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `event_type` - The type of claim event (Added, Removed, or Changed).
/// * `claim` - The claim data.
#[cfg(not(feature = "certora"))]
pub fn emit_claim_event(e: &Env, event_type: ClaimEvent, claim: Claim) {
    match event_type {
        ClaimEvent::Added => ClaimAdded { claim }.publish(e),
        ClaimEvent::Removed => ClaimRemoved { claim }.publish(e),
        ClaimEvent::Changed => ClaimChanged { claim }.publish(e),
    }
}
