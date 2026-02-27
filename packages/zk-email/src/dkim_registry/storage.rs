use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, Vec};

use crate::dkim_registry::{
    emit_key_hash_registered, emit_key_hash_revoked, DKIMRegistryError, DKIM_EXTEND_AMOUNT,
    DKIM_TTL_THRESHOLD,
};

// ################## TYPES ##################

/// Composite key for a domain + public key hash entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DKIMKeyEntry {
    pub domain_hash: BytesN<32>,
    pub public_key_hash: BytesN<32>,
}

/// Storage keys for the DKIM registry module.
#[derive(Clone)]
#[contracttype]
pub enum DKIMRegistryStorageKey {
    /// Maps (domain_hash, public_key_hash) to registration status.
    /// Presence indicates the key hash is registered for the domain.
    DomainPublicKey(DKIMKeyEntry),
    /// Maps public_key_hash to revocation status.
    /// Presence indicates the key hash has been globally revoked.
    RevokedKeyHash(BytesN<32>),
}

// ################## QUERY STATE ##################

/// Returns true if the key hash is registered for the domain AND not revoked.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `domain_hash` - Hash of the email domain.
/// * `public_key_hash` - Hash of the DKIM public key.
pub fn is_key_hash_valid(e: &Env, domain_hash: &BytesN<32>, public_key_hash: &BytesN<32>) -> bool {
    if is_key_hash_revoked(e, public_key_hash) {
        return false;
    }

    let key = DKIMRegistryStorageKey::DomainPublicKey(DKIMKeyEntry {
        domain_hash: domain_hash.clone(),
        public_key_hash: public_key_hash.clone(),
    });

    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, DKIM_TTL_THRESHOLD, DKIM_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Returns true if the key hash has been globally revoked.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `public_key_hash` - Hash of the DKIM public key.
pub fn is_key_hash_revoked(e: &Env, public_key_hash: &BytesN<32>) -> bool {
    let key = DKIMRegistryStorageKey::RevokedKeyHash(public_key_hash.clone());

    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, DKIM_TTL_THRESHOLD, DKIM_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

// ################## CHANGE STATE ##################

/// Registers a DKIM public key hash for a domain.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `domain_hash` - Hash of the email domain.
/// * `public_key_hash` - Hash of the DKIM public key.
///
/// # Errors
///
/// * [`DKIMRegistryError::KeyHashRevoked`] - If the public key hash has been
///   revoked.
/// * [`DKIMRegistryError::KeyHashAlreadyRegistered`] - If the public key hash
///   is already registered for the given domain.
///
/// # Events
///
/// Emits [`KeyHashRegistered`] with the domain and public key hashes.
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn set_dkim_public_key_hash(e: &Env, domain_hash: &BytesN<32>, public_key_hash: &BytesN<32>) {
    if is_key_hash_revoked(e, public_key_hash) {
        panic_with_error!(e, DKIMRegistryError::KeyHashRevoked);
    }

    let key = DKIMRegistryStorageKey::DomainPublicKey(DKIMKeyEntry {
        domain_hash: domain_hash.clone(),
        public_key_hash: public_key_hash.clone(),
    });

    if e.storage().persistent().has(&key) {
        panic_with_error!(e, DKIMRegistryError::KeyHashAlreadyRegistered);
    }

    e.storage().persistent().set(&key, &true);

    emit_key_hash_registered(e, domain_hash, public_key_hash);
}

/// Batch registers DKIM public key hashes for a domain.
///
/// Calls [`set_dkim_public_key_hash`] for each entry.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `domain_hash` - Hash of the email domain.
/// * `public_key_hashes` - Hashes of the DKIM public keys.
///
/// # Errors
///
/// * [`DKIMRegistryError::KeyHashRevoked`] - If any public key hash has been
///   revoked.
/// * [`DKIMRegistryError::KeyHashAlreadyRegistered`] - If any public key hash
///   is already registered for the given domain.
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn set_dkim_public_key_hashes(
    e: &Env,
    domain_hash: &BytesN<32>,
    public_key_hashes: &Vec<BytesN<32>>,
) {
    for public_key_hash in public_key_hashes.iter() {
        set_dkim_public_key_hash(e, domain_hash, &public_key_hash);
    }
}

/// Globally revokes a DKIM public key hash. Once revoked, it cannot be
/// re-registered for any domain.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `public_key_hash` - Hash of the DKIM public key.
///
/// # Events
///
/// Emits [`KeyHashRevoked`] with the public key hash.
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn revoke_dkim_public_key_hash(e: &Env, public_key_hash: &BytesN<32>) {
    let key = DKIMRegistryStorageKey::RevokedKeyHash(public_key_hash.clone());
    e.storage().persistent().set(&key, &true);

    emit_key_hash_revoked(e, public_key_hash);
}
