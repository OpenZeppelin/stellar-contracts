//! # Auditor Key Registry
//!
//! Stores Grumpkin public keys used by the confidential token wrapper to
//! produce auditor ECDH ciphertexts. Each key is indexed by a `u32`
//! `auditor_id` and a single deployment can serve multiple wrapped tokens.
//!
//! ## Why a Separate Contract
//!
//! Auditor keys are referenced by the wrapper on every operation that produces
//! an auditor ciphertext (withdraw, transfer, operator transfer, set/revoke
//! operator). Keeping them in a separate registry allows:
//!
//! - **Reuse**: one registry can serve many wrapped tokens;
//! - **Lifecycle**: registration and rotation can evolve (e.g. versioned keys,
//!   activation ledgers) without redeploying the wrapper;
//! - **Isolation**: key-management privileges are scoped to the registry admin,
//!   distinct from token admin powers.
//!
//! ## Point Encoding
//!
//! Keys are Grumpkin affine points encoded as [`BytesN<64>`], with the first
//! 32 bytes holding the `x` coordinate and the last 32 bytes holding the `y`
//! coordinate, each in big-endian byte order. Both coordinates live in the
//! Grumpkin coordinate field `F_r`, which coincides with the BN254 scalar
//! field. The identity point is `(0, 0)`.
//!
//! ## Validation
//!
//! Every write path runs the same checks, delegated to
//! [`stellar_contract_utils::crypto::grumpkin::Grumpkin`]:
//!
//! 1. **Not identity**: `(x, y) ≠ (0, 0)`. The identity is a valid group
//!    element, but using it as a public key would make every auditor ECDH
//!    ciphertext trivially decryptable, since the salt is public on-chain.
//! 2. **Canonical + on curve**: each coordinate is a canonical 32-byte
//!    representative (`< r`) and `(x, y)` satisfies `y² ≡ x³ - 17 (mod r)`
//!    (Grumpkin: `a = 0`, `b = -17`). The two checks are fused because the
//!    underlying validator rejects non-canonical encodings to keep byte
//!    equality on validated points sound.

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, BytesN, Env};
pub use storage::{get_key, register_key, rotate_key, AuditorStorageKey};

/// Trait for managing Grumpkin auditor public keys used by the confidential
/// token wrapper.
///
/// The wrapper queries [`AuditorRegistry::get_key`] on every operation that
/// produces an auditor ciphertext (withdraw, transfer, operator transfer,
/// set/revoke operator). [`AuditorRegistry::register_key`] and
/// [`AuditorRegistry::rotate_key`] are privileged operations expected to be
/// gated by the implementor's access-control scheme.
#[contracttrait]
pub trait AuditorRegistry {
    /// Registers a Grumpkin public key under a fresh `auditor_id`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `auditor_id` - The identifier to register the key under.
    /// * `point` - The Grumpkin public key, encoded as 32 bytes of `x` followed
    ///   by 32 bytes of `y` (big-endian).
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`AuditorError::AuditorAlreadyRegistered`] - When `auditor_id` is
    ///   already in use.
    /// * [`AuditorError::IdentityPoint`] - When the point is the identity.
    /// * [`AuditorError::PointNotOnCurve`] - When the point is non-canonical or
    ///   does not satisfy the Grumpkin curve equation.
    ///
    /// # Events
    ///
    /// * topics - `["auditor_registered", auditor_id: u32]`
    /// * data - `[point: BytesN<64>]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::register_key`] for the
    /// implementation.
    fn register_key(e: &Env, auditor_id: u32, point: BytesN<64>, operator: Address);

    /// Replaces the Grumpkin public key registered under `auditor_id`.
    ///
    /// Only an operator with sufficient permissions should be able to call
    /// this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `auditor_id` - The identifier whose key is being rotated.
    /// * `new_point` - The new Grumpkin public key.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`AuditorError::AuditorNotRegistered`] - When `auditor_id` has no
    ///   registered key.
    /// * [`AuditorError::IdentityPoint`] - When the point is the identity.
    /// * [`AuditorError::PointNotOnCurve`] - When the point is non-canonical or
    ///   does not satisfy the Grumpkin curve equation.
    ///
    /// # Events
    ///
    /// * topics - `["auditor_rotated", auditor_id: u32]`
    /// * data - `[old_point: BytesN<64>, new_point: BytesN<64>]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::rotate_key`] for the
    /// implementation.
    fn rotate_key(e: &Env, auditor_id: u32, new_point: BytesN<64>, operator: Address);

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
    fn get_key(e: &Env, auditor_id: u32) -> BytesN<64> {
        storage::get_key(e, auditor_id)
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AuditorError {
    /// Indicates the `auditor_id` is already registered.
    AuditorAlreadyRegistered = 3300,
    /// Indicates no key is registered under `auditor_id`.
    AuditorNotRegistered = 3301,
    /// Indicates the point is the identity `(0, 0)`, which is forbidden as an
    /// auditor public key.
    IdentityPoint = 3302,
    /// Indicates the point is non-canonical or does not satisfy
    /// `y² ≡ x³ - 17 (mod r)`.
    PointNotOnCurve = 3303,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const AUDITOR_KEY_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const AUDITOR_KEY_TTL_THRESHOLD: u32 = AUDITOR_KEY_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when a new auditor key is registered.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditorRegistered {
    #[topic]
    pub auditor_id: u32,
    pub point: BytesN<64>,
}

/// Emits an event indicating an auditor key has been registered.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor_id` - The identifier the key was registered under.
/// * `point` - The Grumpkin public key.
pub fn emit_auditor_registered(e: &Env, auditor_id: u32, point: &BytesN<64>) {
    AuditorRegistered { auditor_id, point: point.clone() }.publish(e);
}

/// Event emitted when an auditor key is rotated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditorRotated {
    #[topic]
    pub auditor_id: u32,
    pub old_point: BytesN<64>,
    pub new_point: BytesN<64>,
}

/// Emits an event indicating an auditor key has been rotated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor_id` - The identifier whose key was rotated.
/// * `old_point` - The previously registered key.
/// * `new_point` - The newly registered key.
pub fn emit_auditor_rotated(
    e: &Env,
    auditor_id: u32,
    old_point: &BytesN<64>,
    new_point: &BytesN<64>,
) {
    AuditorRotated { auditor_id, old_point: old_point.clone(), new_point: new_point.clone() }
        .publish(e);
}
