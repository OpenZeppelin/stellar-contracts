use soroban_sdk::{contractclient, contracttype, panic_with_error, Address, Env};

use crate::rwa::{
    claim_issuer::ClaimIssuerClient,
    claim_topics_and_issuers::ClaimTopicsAndIssuersClient,
    identity_claims::{generate_claim_id, Claim, IdentityClaimsClient},
    RWAError,
};

#[cfg(not(feature = "certora"))]
use crate::rwa::{
    emit_claim_topics_and_issuers_set,
};

/// Storage keys for the data associated with `RWA` token
#[contracttype]
pub enum IdentityVerifierStorageKey {
    /// Claim Topics and Issuers contract address
    ClaimTopicsAndIssuers,
    /// Identity Registry Storage contract address
    IdentityRegistryStorage,
}

// We need to declare an `IdentityRegistryStorageClient` here, instead of
// importing one from the dedicated module, as the trait there can't be used
// with `#[contractclient]` macro, because it has an associated type, which is
// not supported by the `#[contractclient]` macro.
// Another option would have been to use `e.invoke_contract`, but we stick with
// the above choice for consistency reasons.
#[allow(unused)]
#[contractclient(name = "IdentityRegistryStorageClient")]
trait IdentityRegistryStorage {
    fn stored_identity(e: &Env, account: Address) -> Address;

    fn get_recovered_to(e: &Env, old_account: Address) -> Option<Address>;
}

/// Returns the Claim Topics and Issuers contract linked to the token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`RWAError::ClaimTopicsAndIssuersNotSet`] - When the claim topics and
///   issuers contract is not set.
pub fn claim_topics_and_issuers(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&IdentityVerifierStorageKey::ClaimTopicsAndIssuers)
        .unwrap_or_else(|| panic_with_error!(e, RWAError::ClaimTopicsAndIssuersNotSet))
}

/// Returns the Identity Registry Storage contract linked to the token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`RWAError::IdentityRegistryStorageNotSet`] - When the identity registry
///   storage contract is not set.
pub fn identity_registry_storage(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&IdentityVerifierStorageKey::IdentityRegistryStorage)
        .unwrap_or_else(|| panic_with_error!(e, RWAError::IdentityRegistryStorageNotSet))
}

/// Verifies that the identity of an user address has the required valid
/// claims.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The account to verify.
///
/// # Errors
///
/// * [`RWAError::IdentityVerificationFailed`] - When the identity of the
///   account cannot be verified.
pub fn verify_identity(e: &Env, account: &Address) {
    let irs_addr = identity_registry_storage(e);
    let irs_client = IdentityRegistryStorageClient::new(e, &irs_addr);

    let identity_addr = irs_client.stored_identity(account);
    let identity_client = IdentityClaimsClient::new(e, &identity_addr);

    let cti_addr = claim_topics_and_issuers(e);
    let cti_client = ClaimTopicsAndIssuersClient::new(e, &cti_addr);

    let topics_and_issuers = cti_client.get_claim_topics_and_issuers();

    for (claim_topic, issuers) in topics_and_issuers.iter() {
        let issuers_with_claim_ids = issuers.iter().enumerate().map(|(i, issuer)| {
            (
                issuer.clone(),
                generate_claim_id(e, &issuer, claim_topic),
                i as u32 == issuers.len() - 1,
            )
        });
        let account_claim_ids = identity_client.get_claim_ids_by_topic(&claim_topic);

        for (issuer, claim_id, is_last) in issuers_with_claim_ids {
            if account_claim_ids.contains(&claim_id) {
                // Here, we can assume claim exists so no need to use `try_get_claim()`.
                let claim: Claim = identity_client.get_claim(&claim_id);

                if validate_claim(e, &claim, claim_topic, &issuer, &identity_addr) {
                    break;
                } else if is_last {
                    panic_with_error!(e, RWAError::IdentityVerificationFailed)
                }
            } else if is_last {
                panic_with_error!(e, RWAError::IdentityVerificationFailed)
            }
        }
    }
}

/// Validates a claim against the expected topic and issuer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `claim` - The claim to validate.
/// * `claim_topic` - The expected claim topic.
/// * `issuer` - The issuer address.
/// * `identity_addr` - The identity address of the investor.
///
/// # Returns
///
/// Returns `true` if the claim is valid, `false` otherwise.
pub fn validate_claim(
    e: &Env,
    claim: &Claim,
    claim_topic: u32,
    issuer: &Address,
    identity_addr: &Address,
) -> bool {
    if claim.topic == claim_topic && claim.issuer == *issuer {
        let validation = ClaimIssuerClient::new(e, issuer).try_is_claim_valid(
            identity_addr,
            &claim_topic,
            &claim.scheme,
            &claim.signature,
            &claim.data,
        );
        matches!(validation, Ok(Ok(_)))
    } else {
        false
    }
}

/// Returns the target address for the recovery process for the old account.
/// If the old account is not a target of a recovery process, `None` is
/// returned.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_account` - The address of the old account.
pub fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
    let irs_addr = identity_registry_storage(e);
    let irs_client = IdentityRegistryStorageClient::new(e, &irs_addr);

    irs_client.get_recovered_to(old_account)
}

/// Sets the claim topics and issuers contract of the token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `claim_topics_and_issuers` - The address of the claim topics and issuers
///   contract.
///
/// # Events
///
/// * topics - `["claim_topics_issuers_set", claim_topics_and_issuers: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used internally or in admin functions that implement their own
/// authorization logic.
pub fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: &Address) {
    e.storage()
        .instance()
        .set(&IdentityVerifierStorageKey::ClaimTopicsAndIssuers, claim_topics_and_issuers);
    #[cfg(not(feature = "certora"))]
    emit_claim_topics_and_issuers_set(e, claim_topics_and_issuers);
}

/// Sets the identity registry storage contract of the token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `identity_registry_storage` - The address of the identity registry storage
///   contract.
///
/// # Events
///
/// * topics - `["identity_registry_storage_set", identity_registry_storage:
///   Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used internally or in admin functions that implement their own
/// authorization logic.
pub fn set_identity_registry_storage(e: &Env, identity_registry_storage: &Address) {
    e.storage()
        .instance()
        .set(&IdentityVerifierStorageKey::IdentityRegistryStorage, identity_registry_storage);
}
