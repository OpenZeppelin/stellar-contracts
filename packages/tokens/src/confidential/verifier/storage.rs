use soroban_sdk::{contracttype, panic_with_error, Bytes, Env};

use crate::confidential::verifier::{
    emit_verification_key_registered, emit_verification_key_updated, CircuitType, VerifierError,
};

/// Storage keys for the verifier registry.
#[contracttype]
pub enum VerifierStorageKey {
    /// Maps [`CircuitType`] to its serialized UltraHonk verification key.
    Vk(CircuitType),
}

// ################## QUERY STATE ##################

/// Returns the UltraHonk verification key registered under `circuit_type`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `circuit_type` - The circuit whose key is requested.
///
/// # Errors
///
/// * [`VerifierError::VerificationKeyNotRegistered`] - When `circuit_type` has
///   no registered key.
pub fn get_verification_key(e: &Env, circuit_type: CircuitType) -> Bytes {
    e.storage()
        .instance()
        .get(&VerifierStorageKey::Vk(circuit_type))
        .unwrap_or_else(|| panic_with_error!(e, VerifierError::VerificationKeyNotRegistered))
}

// ################## CHANGE STATE ##################

/// Registers an UltraHonk verification key under a fresh [`CircuitType`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `circuit_type` - The circuit to register the key under.
/// * `vk` - The serialized UltraHonk verification key.
///
/// # Errors
///
/// * [`VerifierError::VerificationKeyAlreadyRegistered`] - When `circuit_type`
///   already has a verification key registered.
///
/// # Events
///
/// * topics - `["verification_key_registered", circuit_type: CircuitType]`
/// * data - `[vk: Bytes]`
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
pub fn register_verification_key(e: &Env, circuit_type: CircuitType, vk: &Bytes) {
    let key = VerifierStorageKey::Vk(circuit_type);
    if e.storage().instance().has(&key) {
        panic_with_error!(e, VerifierError::VerificationKeyAlreadyRegistered);
    }

    e.storage().instance().set(&key, vk);
    emit_verification_key_registered(e, circuit_type, vk);
}

/// Replaces the UltraHonk verification key registered under `circuit_type`.
///
/// # ⚠️ Soundness-Critical Break-Glass Operation
///
/// Updating a verification key is **not** a routine operation and must be
/// treated as an emergency response, not a maintenance task. Concretely:
///
/// - **A wrong VK silently breaks soundness.** This function writes the
///   `new_vk` bytes verbatim; nothing here checks that they correspond to the
///   audited circuit. A corrupted, misderived, or maliciously crafted
///   replacement will happily verify forged proofs — minting tokens, draining
///   accounts, impersonating registered users.
/// - **The update invalidates every in-flight proof for the affected circuit.**
///   Any proof generated against the previous VK fails the instant the new VK
///   is activated; wallets must regenerate against the new VK and resubmit.
/// - **The caller is responsible for governance.** The recommended posture is
///   full immutability. If an update path is exposed at all, it should be gated
///   by multisig + timelock, and the new VK must be independently reproducible
///   from the audited circuit source, pinned toolchain, and SRS transcript
///   (DESIGN §10.6).
///
/// Use only in response to a discovered soundness bug in a circuit or
/// verifier that cannot be fixed by a fresh deployment.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `circuit_type` - The circuit whose key is being updated.
/// * `new_vk` - The new serialized UltraHonk verification key.
///
/// # Errors
///
/// * [`VerifierError::VerificationKeyNotRegistered`] - When `circuit_type` has
///   no registered key.
///
/// # Events
///
/// * topics - `["verification_key_updated", circuit_type: CircuitType]`
/// * data - `[old_vk: Bytes, new_vk: Bytes]`
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
pub fn update_verification_key(e: &Env, circuit_type: CircuitType, new_vk: &Bytes) {
    let key = VerifierStorageKey::Vk(circuit_type);
    let old_vk: Bytes = e
        .storage()
        .instance()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, VerifierError::VerificationKeyNotRegistered));

    e.storage().instance().set(&key, new_vk);
    emit_verification_key_updated(e, circuit_type, &old_vk, new_vk);
}
