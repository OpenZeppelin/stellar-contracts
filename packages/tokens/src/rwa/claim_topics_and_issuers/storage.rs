use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, Vec};

#[cfg(not(feature = "certora"))]
use super::{
    emit_claim_topic_added, emit_claim_topic_removed, emit_issuer_topics_updated,
    emit_trusted_issuer_added, emit_trusted_issuer_removed,
};
use crate::rwa::claim_topics_and_issuers::{
    ClaimTopicsAndIssuersError, CLAIMS_EXTEND_AMOUNT, CLAIMS_TTL_THRESHOLD, ISSUERS_EXTEND_AMOUNT,
    ISSUERS_TTL_THRESHOLD, MAX_CLAIM_TOPICS, MAX_ISSUERS,
};

/// Storage keys for the data associated with the claim topics and issuers
/// extension
#[contracttype]
pub enum ClaimTopicsAndIssuersStorageKey {
    /// Stores the claim topics registry
    ClaimTopics,
    /// Stores the trusted issuers registry
    TrustedIssuers,
    /// Stores the claim topics allowed for a specific trusted issuer
    IssuerClaimTopics(Address),
    /// Stores the trusted issuers allowed for a specific claim topic
    ClaimTopicIssuers(u32),
}

// ################## QUERY STATE ##################

/// Returns all stored claim topics. Defaults to empty vector if no topics are
/// stored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_claim_topics(e: &Env) -> Vec<u32> {
    let key = ClaimTopicsAndIssuersStorageKey::ClaimTopics;
    if let Some(claim_topics) = e.storage().persistent().get::<_, Vec<u32>>(&key) {
        e.storage().persistent().extend_ttl(&key, CLAIMS_TTL_THRESHOLD, CLAIMS_EXTEND_AMOUNT);
        claim_topics
    } else {
        Vec::new(e)
    }
}

/// Returns all the trusted claim issuers. Defaults to empty vector if no
/// issuers are stored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_trusted_issuers(e: &Env) -> Vec<Address> {
    let key = ClaimTopicsAndIssuersStorageKey::TrustedIssuers;
    if let Some(trusted_issuers) = e.storage().persistent().get::<_, Vec<Address>>(&key) {
        e.storage().persistent().extend_ttl(&key, ISSUERS_TTL_THRESHOLD, ISSUERS_EXTEND_AMOUNT);
        trusted_issuers
    } else {
        Vec::new(e)
    }
}

/// Returns all the trusted issuers allowed for a given claim topic.
/// Defaults to empty vector if no issuers are allowed for the topic.
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
pub fn get_claim_topic_issuers(e: &Env, claim_topic: u32) -> Vec<Address> {
    let key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(claim_topic);
    if let Some(topic_issuers) = e.storage().persistent().get::<_, Vec<Address>>(&key) {
        e.storage().persistent().extend_ttl(&key, ISSUERS_TTL_THRESHOLD, ISSUERS_EXTEND_AMOUNT);
        topic_issuers
    } else {
        panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist);
    }
}

/// Returns all the claim topics of trusted claim issuer. Defaults to empty
/// vector if the issuer has no claim topics assigned.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `trusted_issuer` - The trusted issuer concerned.
///
/// # Errors
///
/// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted issuer
///   does not exist.
pub fn get_trusted_issuer_claim_topics(e: &Env, trusted_issuer: &Address) -> Vec<u32> {
    let key = ClaimTopicsAndIssuersStorageKey::IssuerClaimTopics(trusted_issuer.clone());
    if let Some(issuer_topics) = e.storage().persistent().get::<_, Vec<u32>>(&key) {
        e.storage().persistent().extend_ttl(&key, CLAIMS_TTL_THRESHOLD, CLAIMS_EXTEND_AMOUNT);
        issuer_topics
    } else {
        panic_with_error!(e, ClaimTopicsAndIssuersError::IssuerDoesNotExist);
    }
}

/// Returns all the claim topics and their corresponding trusted issuers.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
    let mut claim_topics_and_issuers = Map::new(e);
    let claim_topics = get_claim_topics(e);
    for claim_topic in claim_topics {
        let issuers = get_claim_topic_issuers(e, claim_topic);
        claim_topics_and_issuers.set(claim_topic, issuers);
    }
    claim_topics_and_issuers
}

/// Checks if the claim issuer contract is trusted.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `issuer` - The address of the claim issuer contract.
pub fn is_trusted_issuer(e: &Env, issuer: &Address) -> bool {
    let trusted_issuers = get_trusted_issuers(e);
    trusted_issuers.contains(issuer)
}

/// Checks if the trusted claim issuer is allowed to emit a certain claim
/// topic.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `issuer` - The address of the trusted issuer's claim issuer contract.
/// * `claim_topic` - The claim topic that has to be checked to determine
///   whether the issuer is allowed to emit it.
///
/// # Errors
///
/// * refer to [`get_trusted_issuer_claim_topics`] errors.
pub fn has_claim_topic(e: &Env, issuer: &Address, claim_topic: u32) -> bool {
    let issuer_topics = get_trusted_issuer_claim_topics(e, issuer);
    issuer_topics.contains(claim_topic)
}

// ################## CHANGE STATE ##################

/// Adds a claim topic (for example: KYC=1, AML=2).
///
/// Cannot add more than 15 topics due to gas concerns.
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
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn add_claim_topic(e: &Env, claim_topic: u32) {
    let mut claim_topics = get_claim_topics(e);

    if claim_topics.len() >= MAX_CLAIM_TOPICS {
        panic_with_error!(e, ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached);
    }

    // Check if topic already exists
    if claim_topics.contains(claim_topic) {
        panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicAlreadyExists);
    } else {
        claim_topics.push_back(claim_topic);
        let key = ClaimTopicsAndIssuersStorageKey::ClaimTopics;
        e.storage().persistent().set(&key, &claim_topics);

        // initializing ClaimTopicIssuers for this topic
        let key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(claim_topic);
        e.storage().persistent().set(&key, &Vec::<Address>::new(e));
        #[cfg(not(feature = "certora"))]
        emit_claim_topic_added(e, claim_topic);
    }
}

/// Removes a claim topic (for example: KYC=1, AML=2).
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
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn remove_claim_topic(e: &Env, claim_topic: u32) {
    let mut claim_topics = get_claim_topics(e);

    // Find and remove the topic
    if let Some(index) = claim_topics.iter().position(|x| x == claim_topic) {
        claim_topics.remove(index as u32);
        let key = ClaimTopicsAndIssuersStorageKey::ClaimTopics;
        e.storage().persistent().set(&key, &claim_topics);

        // Remove the topic from all trusted issuers' IssuerClaimTopics mappings
        let trusted_issuers = get_trusted_issuers(e);
        for issuer in trusted_issuers.iter() {
            let issuer_key = ClaimTopicsAndIssuersStorageKey::IssuerClaimTopics(issuer.clone());
            if let Some(mut issuer_topics) =
                e.storage().persistent().get::<_, Vec<u32>>(&issuer_key)
            {
                if let Some(topic_index) = issuer_topics.iter().position(|x| x == claim_topic) {
                    issuer_topics.remove(topic_index as u32);
                    e.storage().persistent().set(&issuer_key, &issuer_topics);
                }
            }
        }

        // removing ClaimTopicIssuers for this topic
        let key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(claim_topic);
        e.storage().persistent().remove(&key);
        #[cfg(not(feature = "certora"))]
        emit_claim_topic_removed(e, claim_topic);
    } else {
        panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist);
    }
}

/// Registers a claim issuer contract as trusted claim issuer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `trusted_issuer` - The claim issuer contract address of the trusted claim
///   issuer.
/// * `claim_topics` - The set of claim topics that the trusted issuer is
///   allowed to emit.
///
/// # Errors
///
/// * [`ClaimTopicsAndIssuersError::ClaimTopicsSetCannotBeEmpty`] - If the claim
///   topics set is empty.
/// * [`ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached`] - If the
///   maximum number of claim topics is reached.
/// * [`ClaimTopicsAndIssuersError::MaxIssuersLimitReached`] - If the maximum
///   number of issuers is reached.
/// * [`ClaimTopicsAndIssuersError::IssuerAlreadyExists`] - If the issuer
///   already exists.
/// * also refer to [`get_claim_topic_issuers`] errors.
///
/// # Events
///
/// * topics - `["issuer_added", trusted_issuer: Address]`
/// * data - `[claim_topics: Vec<u32>]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn add_trusted_issuer(e: &Env, trusted_issuer: &Address, claim_topics: &Vec<u32>) {
    // Validate inputs
    if claim_topics.is_empty() {
        panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicsSetCannotBeEmpty);
    }
    if claim_topics.len() > MAX_CLAIM_TOPICS {
        panic_with_error!(e, ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached);
    }

    // Validate that there are no duplicate topics in the input
    validate_no_duplicate_topics(e, claim_topics);

    // Validate that all topics exist before making any state changes
    validate_topics_exist(e, claim_topics);

    let mut trusted_issuers = get_trusted_issuers(e);

    // Check limit of 50 trusted issuers
    if trusted_issuers.len() >= MAX_ISSUERS {
        panic_with_error!(e, ClaimTopicsAndIssuersError::MaxIssuersLimitReached);
    }

    // Check if issuer already exists
    if trusted_issuers.contains(trusted_issuer) {
        panic_with_error!(e, ClaimTopicsAndIssuersError::IssuerAlreadyExists);
    }

    // Add issuer to trusted issuers list
    trusted_issuers.push_back(trusted_issuer.clone());
    let issuers_key = ClaimTopicsAndIssuersStorageKey::TrustedIssuers;
    e.storage().persistent().set(&issuers_key, &trusted_issuers);

    // Store claim topics for this issuer
    let topics_key = ClaimTopicsAndIssuersStorageKey::IssuerClaimTopics(trusted_issuer.clone());
    e.storage().persistent().set(&topics_key, claim_topics);

    // Update reverse mapping: claim topic -> issuers
    for topic in claim_topics.iter() {
        let mut topic_issuers = get_claim_topic_issuers(e, topic);

        // This is a new issuer, so we don't need to check if it already exists
        topic_issuers.push_back(trusted_issuer.clone());
        let topic_key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(topic);
        e.storage().persistent().set(&topic_key, &topic_issuers);
    }
    #[cfg(not(feature = "certora"))]
    emit_trusted_issuer_added(e, trusted_issuer, claim_topics.clone());
}

/// Removes the trusted issuer contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `trusted_issuer` - The trusted issuer to remove.
///
/// # Errors
///
/// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted issuer
///   does not exist.
/// * also refer to [`get_trusted_issuer_claim_topics`] errors.
/// * also refer to [`get_claim_topic_issuers`] errors.
///
/// # Events
///
/// * topics - `["issuer_removed", trusted_issuer: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn remove_trusted_issuer(e: &Env, trusted_issuer: &Address) {
    let mut trusted_issuers = get_trusted_issuers(e);

    // Find and remove the trusted issuer
    if let Some(index) = trusted_issuers.iter().position(|addr| addr == *trusted_issuer) {
        // Get the claim topics for this issuer before removing
        let issuer_topics = get_trusted_issuer_claim_topics(e, trusted_issuer);

        // Remove issuer from trusted issuers list
        trusted_issuers.remove(index as u32);
        let issuers_key = ClaimTopicsAndIssuersStorageKey::TrustedIssuers;
        e.storage().persistent().set(&issuers_key, &trusted_issuers);

        // Remove issuer's claim topics
        let topics_key = ClaimTopicsAndIssuersStorageKey::IssuerClaimTopics(trusted_issuer.clone());
        e.storage().persistent().remove(&topics_key);

        // Update reverse mapping: remove issuer from claim topic -> issuers
        for topic in issuer_topics.iter() {
            let mut topic_issuers = get_claim_topic_issuers(e, topic);
            if let Some(issuer_index) =
                topic_issuers.iter().position(|addr| addr == *trusted_issuer)
            {
                topic_issuers.remove(issuer_index as u32);
                let topic_key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(topic);
                e.storage().persistent().set(&topic_key, &topic_issuers);
            }
        }
        #[cfg(not(feature = "certora"))]
        emit_trusted_issuer_removed(e, trusted_issuer);
    } else {
        panic_with_error!(e, ClaimTopicsAndIssuersError::IssuerDoesNotExist);
    }
}

/// Updates the set of claim topics that a trusted issuer is allowed to
/// emit.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `trusted_issuer` - The trusted issuer to update.
/// * `claim_topics` - The set of claim topics that the trusted issuer is
///   allowed to emit.
///
/// # Errors
///
/// * [`ClaimTopicsAndIssuersError::IssuerDoesNotExist`] - If the trusted issuer
///   does not exist.
/// * [`ClaimTopicsAndIssuersError::ClaimTopicsSetCannotBeEmpty`] - If the claim
///   topics set is empty.
/// * [`ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached`] - If the
///   maximum number of claim topics is reached.
/// * also refer to [`get_trusted_issuer_claim_topics`] errors.
/// * also refer to [`get_claim_topic_issuers`] errors.
///
/// # Events
///
/// * topics - `["topics_updated", trusted_issuer: Address]`
/// * data - `[claim_topics: Vec<u32>]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn update_issuer_claim_topics(e: &Env, trusted_issuer: &Address, claim_topics: &Vec<u32>) {
    // Validate inputs
    if claim_topics.is_empty() {
        panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicsSetCannotBeEmpty);
    }
    if claim_topics.len() > MAX_CLAIM_TOPICS {
        panic_with_error!(e, ClaimTopicsAndIssuersError::MaxClaimTopicsLimitReached);
    }

    // Validate that there are no duplicate topics in the input
    validate_no_duplicate_topics(e, claim_topics);

    // Validate that all topics exist before making any state changes
    validate_topics_exist(e, claim_topics);

    // Check if issuer exists
    if !is_trusted_issuer(e, trusted_issuer) {
        panic_with_error!(e, ClaimTopicsAndIssuersError::IssuerDoesNotExist);
    }

    // Get old claim topics to calculate differences
    let old_topics = get_trusted_issuer_claim_topics(e, trusted_issuer);

    // Calculate topics to remove (in old but not in new)
    #[cfg(not(feature = "certora"))]
    let topics_to_remove: Vec<u32> =
        Vec::from_iter(e, old_topics.iter().filter(|old_topic| !claim_topics.contains(old_topic)));

    #[cfg(feature = "certora")]
    let topics_to_remove: Vec<u32> = {
        let mut v: Vec<u32> = Vec::new(e);
        for old_topic in old_topics.iter().filter(|old_topic| !claim_topics.contains(old_topic)) {
            v.push_back(old_topic);
        }
        v
    };

    // Calculate topics to add (in new but not in old)
    #[cfg(not(feature = "certora"))]
    let topics_to_add: Vec<u32> =
        Vec::from_iter(e, claim_topics.iter().filter(|new_topic| !old_topics.contains(new_topic)));

    #[cfg(feature = "certora")]
    let topics_to_add: Vec<u32> = {
        let mut v: Vec<u32> = Vec::new(e);
        for claim_topic in claim_topics.iter().filter(|new_topic| !old_topics.contains(new_topic)) {
            v.push_back(claim_topic);
        }
        v
    };

    // Update issuer's claim topics
    let topics_key = ClaimTopicsAndIssuersStorageKey::IssuerClaimTopics(trusted_issuer.clone());
    e.storage().persistent().set(&topics_key, claim_topics);

    // Remove issuer from topics that are no longer assigned
    for topic_to_remove in topics_to_remove {
        let mut topic_issuers = get_claim_topic_issuers(e, topic_to_remove);
        if let Some(index) = topic_issuers.iter().position(|addr| addr == *trusted_issuer) {
            topic_issuers.remove(index as u32);
            let topic_key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(topic_to_remove);
            e.storage().persistent().set(&topic_key, &topic_issuers);
        }
    }

    // Add issuer to new topics
    for topic_to_add in topics_to_add {
        let mut topic_issuers = get_claim_topic_issuers(e, topic_to_add);
        // We are sure that the issuer is not in the list, because `topics_to_add` only
        // consists of the difference between old and new topics
        topic_issuers.push_back(trusted_issuer.clone());
        let topic_key = ClaimTopicsAndIssuersStorageKey::ClaimTopicIssuers(topic_to_add);
        e.storage().persistent().set(&topic_key, &topic_issuers);
    }
    #[cfg(not(feature = "certora"))]
    emit_issuer_topics_updated(e, trusted_issuer, claim_topics.clone());
}

// ################## HELPER FUNCTIONS ##################

/// Validates that all topics in the provided vector exist in the global claim
/// topics registry.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `topics` - The vector of topics to validate.
///
/// # Errors
///
/// * [`ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist`] - If any topic
///   doesn't exist.
fn validate_topics_exist(e: &Env, topics: &Vec<u32>) {
    let global_topics = get_claim_topics(e);
    for topic in topics.iter() {
        if !global_topics.contains(topic) {
            panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicDoesNotExist);
        }
    }
}

/// Validates that a vector of topics contains no duplicates.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `topics` - The vector of topics to validate.
///
/// # Errors
///
/// * [`ClaimTopicsAndIssuersError::ClaimTopicAlreadyExists`] - If duplicates
///   are found.
fn validate_no_duplicate_topics(e: &Env, topics: &Vec<u32>) {
    // Check for duplicates using Map for O(n) complexity instead of O(n²)
    let mut seen = Map::<u32, ()>::new(e);
    for i in 0..topics.len() {
        let topic = topics.get_unchecked(i);
        if seen.contains_key(topic) {
            panic_with_error!(e, ClaimTopicsAndIssuersError::ClaimTopicAlreadyExists);
        }
        seen.set(topic, ());
    }
}
