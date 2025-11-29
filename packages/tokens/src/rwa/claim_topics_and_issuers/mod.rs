pub mod storage;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{contractclient, contracterror, Address, Env, Map, Vec};

/// Trait for managing claim topics and trusted issuers for RWA tokens.
///
/// [`ClaimTopicsAndIssuers`] trait is not expected to be an extension to a RWA
/// smart contract, but it is a separate contract on its own. This design allows
/// it to be shared across many RWA tokens. Note that, there is no `RWA` bound
/// on the [`ClaimTopicsAndIssuers`] trait:
///
/// ```rust, ignore
/// pub trait ClaimTopicsAndIssuers       // ✅
/// pub trait ClaimTopicsAndIssuers: RWA  // ❌
/// ```
#[contractclient(name = "ClaimTopicsAndIssuersClient")]
pub trait ClaimTopicsAndIssuers {
    // ################## CLAIM TOPICS ##################

    /// Adds a claim topic (for example: KYC=1, AML=2).
    ///
    /// Only an operator with sufficient permissions should be able to call this
    /// function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topic` - The claim topic index.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached`] - If the
    ///   maximum number of claim topics is reached.
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicAlreadyExists`] - If the claim
    ///   topic already exists.
    ///
    /// # Events
    ///
    /// * topics - `["claim_added", claim_topic: u32]`
    /// * data - `[]`
    fn add_claim_topic(e: &Env, claim_topic: u32, operator: Address);

    /// Removes a claim topic (for example: KYC=1, AML=2).
    ///
    /// Only an operator with sufficient permissions should be able to call this
    /// function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topic` - The claim topic index.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist`] - If the claim
    ///   topic does not exist.
    ///
    /// # Events
    ///
    /// * topics - `["claim_removed", claim_topic: u32]`
    /// * data - `[]`
    fn remove_claim_topic(e: &Env, claim_topic: u32, operator: Address);

    /// Returns the claim topics for the security token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_claim_topics(e: &Env) -> Vec<u32>;

    // ################## TRUSTED ISSUERS ##################

    /// Registers a claim issuer contract as trusted claim issuer.
    ///
    /// Only an operator with sufficient permissions should be able to call this
    /// function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `trusted_issuer` - The claim issuer contract address of the trusted
    ///   claim issuer.
    /// * `claim_topics` - The set of claim topics that the trusted issuer is
    ///   allowed to emit.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicsSetCannotBeEmpty`] - If the
    ///   claim topics set is empty.
    /// * [`ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached`] - If the
    ///   maximum number of claim topics is reached.
    /// * [`ClaimTopicsAndIssuersError::MaxIssuersLimitReached`] - If the
    ///   maximum number of issuers is reached.
    /// * [`ClaimTopicsAndIssuersError::IssuerAlreadyExists`] - If the issuer
    ///   already exists.
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist`] - If the claim
    ///   topic does not exist.
    ///
    /// # Events
    ///
    /// * topics - `["issuer_added", trusted_issuer: Address]`
    /// * data - `[claim_topics: Vec<u32>]`
    fn add_trusted_issuer(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    );

    /// Removes the claim issuer contract of a trusted claim issuer.
    ///
    /// Only an operator with sufficient permissions should be able to call this
    /// function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `trusted_issuer` - The claim issuer to remove.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted
    ///   issuer does not exist.
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist`] - If the claim
    ///   topic does not exist.
    ///
    /// # Events
    ///
    /// * topics - `["issuer_removed", trusted_issuer: Address]`
    /// * data - `[]`
    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, operator: Address);

    /// Updates the set of claim topics that a trusted issuer is allowed to
    /// emit.
    ///
    /// Only an operator with sufficient permissions should be able to call this
    /// function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `trusted_issuer` - The claim issuer to update.
    /// * `claim_topics` - The set of claim topics that the trusted issuer is
    ///   allowed to emit.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted
    ///   issuer does not exist.
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicsSetCannotBeEmpty`] - If the
    ///   claim topics set is empty.
    /// * [`ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached`] - If the
    ///   maximum number of claim topics is reached.
    /// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted
    ///   issuer does not exist.
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist`] - If the claim
    ///   topic does not exist.
    ///
    /// # Events
    ///
    /// * topics - `["topics_updated", trusted_issuer: Address]`
    /// * data - `[claim_topics: Vec<u32>]`
    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    );

    /// Returns all the trusted claim issuers stored.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_trusted_issuers(e: &Env) -> Vec<Address>;

    /// Returns all the trusted issuers allowed for a given claim topic.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topic` - The claim topic to get the trusted issuers for.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist`] - If the claim
    ///   topic does not exist.
    fn get_claim_topic_issuers(e: &Env, claim_topic: u32) -> Vec<Address>;

    /// Returns all the claim topics and their corresponding trusted issuers as
    /// a Mapping.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>>;

    /// Checks if the claim issuer contract is trusted.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `issuer` - The address of the claim issuer contract.
    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool;

    /// Returns all the claim topics of trusted claim issuer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `trusted_issuer` - The trusted issuer concerned.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted
    ///   issuer does not exist.
    fn get_trusted_issuer_claim_topics(e: &Env, trusted_issuer: Address) -> Vec<u32>;

    /// Checks if the trusted claim issuer is allowed to emit a certain claim
    /// topic.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `issuer` - The address of the trusted issuer's claim issuer contract.
    /// * `claim_topic` - The claim topic that has to be checked to know if the
    ///   issuer is allowed to emit it.
    ///
    /// # Errors
    ///
    /// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted
    ///   issuer does not exist.
    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool;
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimTopicsAndIssuersError {
    /// Indicates a non-existent claim topic.
    ClaimTopicDoesNotExist = 370,
    /// Indicates a non-existent trusted issuer.
    IssuerDoesNotExist = 371,
    /// Indicates a claim topic already exists.
    ClaimTopicAlreadyExists = 372,
    /// Indicates a trusted issuer already exists.
    IssuerAlreadyExists = 373,
    /// Indicates max claim topics limit is reached.
    MaxClaimTopicsLimitReached = 374,
    /// Indicates max trusted issuers limit is reached.
    MaxIssuersLimitReached = 375,
    /// Indicates claim topics set provided for the issuer cannot be empty.
    ClaimTopicsSetCannotBeEmpty = 376,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const CLAIMS_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const CLAIMS_TTL_THRESHOLD: u32 = CLAIMS_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const ISSUERS_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const ISSUERS_TTL_THRESHOLD: u32 = ISSUERS_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const MAX_CLAIM_TOPICS: u32 = 15;
pub const MAX_ISSUERS: u32 = 50;

// ################## EVENTS ##################

/// Event emitted when a claim topic is added.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTopicAdded {
    #[topic]
    pub claim_topic: u32,
}

/// Emits an event indicating a claim topic has been added.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_topic` - The claim topic that was added.
#[cfg(not(feature = "certora"))]
pub fn emit_claim_topic_added(e: &Env, claim_topic: u32) {
    ClaimTopicAdded { claim_topic }.publish(e);
}

/// Event emitted when a claim topic is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTopicRemoved {
    #[topic]
    pub claim_topic: u32,
}

/// Emits an event indicating a claim topic has been removed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `claim_topic` - The claim topic that was removed.
#[cfg(not(feature = "certora"))]
pub fn emit_claim_topic_removed(e: &Env, claim_topic: u32) {
    ClaimTopicRemoved { claim_topic }.publish(e);
}

/// Event emitted when a trusted issuer is added.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedIssuerAdded {
    #[topic]
    pub trusted_issuer: Address,
    pub claim_topics: Vec<u32>,
}

/// Emits an event indicating a trusted issuer has been added.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `trusted_issuer` - The trusted issuer that was added.
/// * `claim_topics` - The claim topics associated with the trusted issuer.
#[cfg(not(feature = "certora"))]
pub fn emit_trusted_issuer_added(e: &Env, trusted_issuer: &Address, claim_topics: Vec<u32>) {
    TrustedIssuerAdded { trusted_issuer: trusted_issuer.clone(), claim_topics }.publish(e);
}

/// Event emitted when a trusted issuer is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedIssuerRemoved {
    #[topic]
    pub trusted_issuer: Address,
}

/// Emits an event indicating a trusted issuer has been removed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `trusted_issuer` - The trusted issuer that was removed.
#[cfg(not(feature = "certora"))]
pub fn emit_trusted_issuer_removed(e: &Env, trusted_issuer: &Address) {
    TrustedIssuerRemoved { trusted_issuer: trusted_issuer.clone() }.publish(e);
}

/// Event emitted when issuer topics are updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerTopicsUpdated {
    #[topic]
    pub trusted_issuer: Address,
    pub claim_topics: Vec<u32>,
}

/// Emits an event indicating claim topics have been updated for a trusted
/// issuer.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `trusted_issuer` - The trusted issuer whose claim topics were updated.
/// * `claim_topics` - The updated claim topics.
#[cfg(not(feature = "certora"))]
pub fn emit_issuer_topics_updated(e: &Env, trusted_issuer: &Address, claim_topics: Vec<u32>) {
    IssuerTopicsUpdated { trusted_issuer: trusted_issuer.clone(), claim_topics }.publish(e);
}
