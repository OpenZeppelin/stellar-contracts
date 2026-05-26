use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, TryFromVal, Val};

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
/// Runs [`validate_point`] on `point` before storing it. Reverts if the
/// `auditor_id` already has a key; use [`rotate_key`] to replace.
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
/// Runs [`validate_point`] on `new_point` before storing it.
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
/// 1. `x ≥ r` or `y ≥ r` (non-canonical encoding);
/// 2. `(x, y) = (0, 0)` (identity, which would make every auditor ECDH
///    ciphertext trivially decryptable);
/// 3. `y² ≢ x³ - 17 (mod r)` (not on the Grumpkin curve).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `point` - The candidate Grumpkin public key.
///
/// # Errors
///
/// * [`AuditorError::NonCanonicalPoint`] - When either coordinate is not in
///   `[0, r)`.
/// * [`AuditorError::IdentityPoint`] - When the point is the identity.
/// * [`AuditorError::PointNotOnCurve`] - When the point does not satisfy the
///   Grumpkin curve equation.
pub fn validate_point(e: &Env, point: &BytesN<64>) {
    if !point_validator::is_canonical(e, point) {
        panic_with_error!(e, AuditorError::NonCanonicalPoint);
    }
    if !point_validator::is_not_identity(point) {
        panic_with_error!(e, AuditorError::IdentityPoint);
    }
    if !point_validator::is_on_curve(e, point) {
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

// ################## POINT VALIDATION ##################
//
// Grumpkin point validation. Until the dedicated Grumpkin arithmetic crate
// (see issue #700) lands, these helpers live here so the auditor contract is
// self-contained. The canonical and identity checks are final; the on-curve
// check is a placeholder routed through a single function that will be
// replaced by `stellar_grumpkin::is_on_curve` once available.
mod point_validator {
    use soroban_sdk::{Bytes, BytesN, Env, U256};

    /// BN254 scalar field modulus `r`, in big-endian bytes. This is also the
    /// Grumpkin coordinate field modulus.
    ///
    /// `r = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001`
    const FR_MODULUS_BE: [u8; 32] = [
        0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58,
        0x5d, 0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00,
        0x00, 0x01,
    ];

    /// Returns true when both coordinates of `point` are strictly less than
    /// the Grumpkin coordinate field modulus `r`.
    pub fn is_canonical(e: &Env, point: &BytesN<64>) -> bool {
        let raw = point.to_array();
        let modulus = U256::from_be_bytes(e, &Bytes::from_array(e, &FR_MODULUS_BE));

        let mut x_bytes = [0u8; 32];
        let mut y_bytes = [0u8; 32];
        x_bytes.copy_from_slice(&raw[..32]);
        y_bytes.copy_from_slice(&raw[32..]);

        let x = U256::from_be_bytes(e, &Bytes::from_array(e, &x_bytes));
        let y = U256::from_be_bytes(e, &Bytes::from_array(e, &y_bytes));

        x < modulus && y < modulus
    }

    /// Returns true when `point` is not the identity `(0, 0)`.
    pub fn is_not_identity(point: &BytesN<64>) -> bool {
        point.to_array().iter().any(|byte| *byte != 0)
    }

    /// Returns true when `point` satisfies `y² ≡ x³ - 17 (mod r)`.
    ///
    /// TODO(#700): replace this stub with the on-curve helper from the
    /// dedicated Grumpkin arithmetic crate. Until that crate lands, this
    /// helper accepts any canonical, non-identity point. The auditor
    /// contract's structure is intentionally arranged so the swap is a
    /// one-line change here.
    pub fn is_on_curve(_e: &Env, _point: &BytesN<64>) -> bool {
        true
    }
}
