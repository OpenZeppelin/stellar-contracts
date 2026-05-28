use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, TryFromVal, Val};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::confidential::auditor::{
    emit_auditor_registered, emit_auditor_rotated, AuditorError, AUDITOR_KEY_EXTEND_AMOUNT,
    AUDITOR_KEY_TTL_THRESHOLD,
};

/// Storage keys for the auditor registry.
#[contracttype]
pub enum AuditorStorageKey {
    /// Maps `auditor_id` to its Grumpkin public key encoded as `(x, y)`.
    Key(u32),
}

// ################## QUERY STATE ##################

/// Returns the Grumpkin public key registered under `auditor_id`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor_id` - The identifier of the auditor whose key is requested.
///
/// # Errors
///
/// * [`AuditorError::AuditorNotRegistered`] - When `auditor_id` has no
///   registered key.
pub fn get_key(e: &Env, auditor_id: u32) -> BytesN<64> {
    get_persistent_entry(e, &AuditorStorageKey::Key(auditor_id))
        .unwrap_or_else(|| panic_with_error!(e, AuditorError::AuditorNotRegistered))
}

// ################## CHANGE STATE ##################

/// Registers a Grumpkin public key under a fresh `auditor_id`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor_id` - The identifier to register the key under.
/// * `point` - The Grumpkin public key.
///
/// # Errors
///
/// * [`AuditorError::AuditorAlreadyRegistered`] - When `auditor_id` is already
///   in use.
/// * refer to [`validate_point`] errors.
///
/// # Events
///
/// * topics - `["auditor_registered", auditor_id: u32]`
/// * data - `[point: BytesN<64>]`
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
pub fn register_key(e: &Env, auditor_id: u32, point: &BytesN<64>) {
    let key = AuditorStorageKey::Key(auditor_id);
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, AuditorError::AuditorAlreadyRegistered);
    }

    validate_point(e, point);

    e.storage().persistent().set(&key, point);
    emit_auditor_registered(e, auditor_id, point);
}

/// Replaces the Grumpkin public key registered under `auditor_id`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor_id` - The identifier whose key is being rotated.
/// * `new_point` - The new Grumpkin public key.
///
/// # Errors
///
/// * [`AuditorError::AuditorNotRegistered`] - When `auditor_id` has no
///   registered key.
/// * refer to [`validate_point`] errors.
///
/// # Events
///
/// * topics - `["auditor_rotated", auditor_id: u32]`
/// * data - `[old_point: BytesN<64>, new_point: BytesN<64>]`
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
pub fn rotate_key(e: &Env, auditor_id: u32, new_point: &BytesN<64>) {
    let key = AuditorStorageKey::Key(auditor_id);
    let old_point: BytesN<64> = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, AuditorError::AuditorNotRegistered));

    validate_point(e, new_point);

    e.storage().persistent().set(&key, new_point);
    emit_auditor_rotated(e, auditor_id, &old_point, new_point);
}

// ################## LOW-LEVEL HELPERS ##################

/// Validates that `point` is a usable Grumpkin auditor public key.
///
/// The point is rejected when any of the following holds:
///
/// 1. `(x, y) = (0, 0)` (identity, which would make every auditor ECDH
///    ciphertext trivially decryptable);
/// 2. either coordinate is non-canonical (`x ≥ r` or `y ≥ r`), or `(x, y)` does
///    not satisfy `y² ≡ x³ - 17 (mod r)`. The Grumpkin validator fuses these
///    two checks: a non-canonical encoding has multiple bytewise
///    representations of the same logical point, which would defeat any
///    uniqueness check downstream contracts may run against the raw key bytes.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `point` - The candidate Grumpkin public key.
///
/// # Errors
///
/// * [`AuditorError::IdentityPoint`] - When the point is the identity.
/// * [`AuditorError::PointNotOnCurve`] - When the point is non-canonical or
///   does not satisfy the Grumpkin curve equation.
pub fn validate_point(e: &Env, point: &BytesN<64>) {
    if !Grumpkin::is_not_identity(point) {
        panic_with_error!(e, AuditorError::IdentityPoint);
    }
    if !Grumpkin::is_on_curve(e, point) {
        panic_with_error!(e, AuditorError::PointNotOnCurve);
    }
}

/// Helper function that tries to retrieve a persistent storage value and
/// extend its TTL if the entry exists.
fn get_persistent_entry<T: TryFromVal<Env, Val>>(e: &Env, key: &AuditorStorageKey) -> Option<T> {
    e.storage().persistent().get::<_, T>(key).inspect(|_| {
        e.storage().persistent().extend_ttl(
            key,
            AUDITOR_KEY_TTL_THRESHOLD,
            AUDITOR_KEY_EXTEND_AMOUNT,
        );
    })
}
