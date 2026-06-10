//! # Confidential Verifier Registry
//!
//! Stores UltraHonk verification keys used by the confidential token to
//! verify zero-knowledge proofs accompanying every state-changing operation.
//! Each key is indexed by [`CircuitType`]. A single deployment can serve
//! multiple confidential tokens: per-token binding is enforced inside the
//! circuit via the `addr_f` field (DESIGN §2.7, §4.2), not by the verifier, so
//! the same VK set is reusable across every confidential token that targets the
//! same protocol version.
//!
//! # ⚠️ Not Production Ready
//!
//! This module is **unfinished**. [`ConfidentialVerifier::verify_proof`] has no
//! working default implementation because its UltraHonk backend
//! ([`NethermindEth/rs-soroban-ultrahonk`](https://github.com/NethermindEth/rs-soroban-ultrahonk))
//! is still under development and **has not been audited**. Do **not** deploy a
//! contract built on this trait to mainnet or any environment that handles
//! real value. The trait surface, the [`VerifierStorageKey`] layout, and the
//! VK-management helpers in [`storage`] are stable enough for the confidential
//! token to scaffold against, and they are the only part of this
//! module that is intended to be relied upon today.
//!
//! ## Why a Separate Contract
//!
//! Verification keys are referenced by the confidential token on every
//! state-changing operation (register, withdraw, transfer, spender flows).
//! Keeping them in a separate registry allows:
//!
//! - **Isolation**: VK-management privileges are scoped to the verifier admin,
//!   distinct from token admin powers.
//! - **Lifecycle**: per-circuit VKs can be rotated (e.g. when a circuit is
//!   patched) without redeploying the confidential token, subject to the
//!   deployer's governance posture (see DESIGN §3.5).
//!
//! ## VK Encoding
//!
//! Verification keys are opaque [`Bytes`] blobs from this module's point of
//! view; structural validation lives in the future UltraHonk backend, not in
//! the storage layer. The on-disk reference format committed under
//! `circuits/vks/` is a JSON array of hex-encoded `Fr` field elements (one
//! file per circuit, produced by `bb write_vk --output_format fields`).
//!
//! ## Storage
//!
//! Verification keys live in **instance** storage: there is exactly one VK per
//! [`CircuitType`] for the lifetime of the verifier contract, and they are
//! consulted on every confidential token invocation. Per the workspace
//! conventions, instance-TTL management is the contract developer's
//! responsibility — this module's helpers never call `instance().extend_ttl()`.
//!
//! ## Updating a Verification Key Is a Last Resort
//!
//! [`ConfidentialVerifier::update_verification_key`] (and the underlying
//! [`storage::update_verification_key`] helper) is **soundness-critical** and
//! should be treated as a break-glass operation. DESIGN §3.5 makes the
//! strongest possible recommendation: ship VKs immutably and require a fresh
//! deployment for any circuit change. If a deployment exposes an update path
//! at all, it should be reserved for one situation: a discovered soundness
//! bug in a circuit or verifier that needs a fast fix.
//!
//! Concretely, an update:
//!
//! - **Can silently break soundness.** A VK that does not correspond to the
//!   audited circuit will happily verify forged proofs — including proofs that
//!   mint tokens, drain accounts, or impersonate registered users. The on-chain
//!   bytes are an opaque blob to this module; nothing here can detect a wrong,
//!   corrupted, or maliciously crafted replacement.
//! - **Invalidates every in-flight proof for the affected circuit.** Any proof
//!   generated against the previous VK fails verification the instant the new
//!   VK is activated, so the corresponding transactions revert at the
//!   proof-verification boundary. Wallets must regenerate against the new VK
//!   and resubmit (DESIGN §8.6 discusses this for the related auditor-key
//!   rotation case; the same reasoning applies here).
//! - **Should be gated.** Implementors are expected to put the update path
//!   behind strong access control (multisig + timelock at a minimum) and to
//!   publish enough material — circuit source, toolchain pin, SRS transcript
//!   reference (DESIGN §10.6) — that any user can independently reproduce the
//!   new VK before trusting it.
//!
//! Treat every call to `update_verification_key` as an emergency, not a
//! routine operation.

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, contracttype, Address, Bytes, Env};
pub use storage::{
    get_verification_key, register_verification_key, update_verification_key, VerifierStorageKey,
};

/// Identifier of a zero-knowledge circuit whose verification key is stored in
/// the registry. The numeric values are part of the on-chain interface and
/// MUST NOT change.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CircuitType {
    Register = 0,
    Withdraw = 1,
    Transfer = 2,
    SpenderTransfer = 3,
    SetSpender = 4,
    RevokeSpender = 5,
}

/// Trait for managing UltraHonk verification keys used by the confidential
/// token.
///
/// The confidential token calls [`ConfidentialVerifier::verify_proof`] on
/// every state-changing operation, passing the [`CircuitType`] that identifies
/// the circuit, the serialized public inputs, and the proof blob.
/// [`ConfidentialVerifier::register_verification_key`] and
/// [`ConfidentialVerifier::update_verification_key`] are privileged operations
/// expected to be gated by the implementor's access-control scheme.
///
/// # ⚠️ Not Production Ready
///
/// See the module-level warning. `verify_proof` cannot be wired to a real
/// UltraHonk backend until `rs-soroban-ultrahonk` is released and audited.
#[contracttrait]
pub trait ConfidentialVerifier {
    /// Registers an UltraHonk verification key under a fresh [`CircuitType`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `circuit_type` - The circuit to register the key under.
    /// * `vk` - The serialized UltraHonk verification key.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`VerifierError::VerificationKeyAlreadyRegistered`] - When
    ///   `circuit_type` already has a verification key registered.
    ///
    /// # Events
    ///
    /// * topics - `["verification_key_registered", circuit_type: CircuitType]`
    /// * data - `[vk: Bytes]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should
    /// be enforced on `operator` before calling
    /// [`storage::register_verification_key`] for the implementation.
    fn register_verification_key(e: &Env, circuit_type: CircuitType, vk: Bytes, operator: Address);

    /// Replaces the UltraHonk verification key registered under `circuit_type`.
    ///
    /// # ⚠️ Soundness-Critical Break-Glass Operation
    ///
    /// Updating a verification key is **not** a routine operation. A wrong,
    /// corrupted, or maliciously crafted replacement makes the affected
    /// circuit accept forged proofs — minting, draining, impersonation, and
    /// every other constraint the circuit was meant to enforce silently
    /// collapse. Activating a new VK also invalidates every in-flight proof
    /// produced against the previous one.
    ///
    /// The recommended posture is **full immutability**: ship VKs fixed at
    /// deployment and require a fresh deployment for any circuit change.
    /// Exposing this method should be reserved for fixing a discovered
    /// soundness bug in a circuit or verifier, and should be behind strong
    /// governance (multisig + timelock at minimum). The new VK must be
    /// independently reproducible from the audited circuit source, pinned
    /// toolchain, and SRS transcript (DESIGN §10.6).
    ///
    /// Only an operator with sufficient permissions should be able to call
    /// this function.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `circuit_type` - The circuit whose key is being updated.
    /// * `new_vk` - The new serialized UltraHonk verification key.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Errors
    ///
    /// * [`VerifierError::VerificationKeyNotRegistered`] - When `circuit_type`
    ///   has no registered key.
    ///
    /// # Events
    ///
    /// * topics - `["verification_key_updated", circuit_type: CircuitType]`
    /// * data - `[old_vk: Bytes, new_vk: Bytes]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should
    /// be enforced on `operator` before calling
    /// [`storage::update_verification_key`] for the implementation.
    fn update_verification_key(
        e: &Env,
        circuit_type: CircuitType,
        new_vk: Bytes,
        operator: Address,
    );

    /// Verifies an UltraHonk proof against the verification key registered
    /// under `circuit_type` and returns `true` iff the proof is valid for the
    /// given `public_inputs`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `circuit_type` - The circuit the proof was produced against.
    /// * `public_inputs` - The serialized public inputs the prover committed
    ///   to.
    /// * `proof` - The serialized UltraHonk proof.
    ///
    /// # Errors
    ///
    /// * [`VerifierError::VerificationKeyNotRegistered`] - When `circuit_type`
    ///   has no registered key.
    ///
    /// # Notes
    ///
    /// No default implementation is provided. The UltraHonk verification
    /// backend lives in `NethermindEth/rs-soroban-ultrahonk`, which is still
    /// under development and has not been audited (see the module-level
    /// warning). Implementors MUST NOT ship a stub that returns `true`
    /// unconditionally to any environment that handles real value.
    fn verify_proof(e: &Env, circuit_type: CircuitType, public_inputs: Bytes, proof: Bytes)
        -> bool;

    /// Returns the UltraHonk verification key registered under `circuit_type`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `circuit_type` - The circuit whose key is requested.
    ///
    /// # Errors
    ///
    /// * [`VerifierError::VerificationKeyNotRegistered`] - When `circuit_type`
    ///   has no registered key.
    fn get_verification_key(e: &Env, circuit_type: CircuitType) -> Bytes {
        storage::get_verification_key(e, circuit_type)
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VerifierError {
    /// Indicates `circuit_type` already has a verification key registered.
    VerificationKeyAlreadyRegistered = 3400,
    /// Indicates no verification key is registered under `circuit_type`.
    VerificationKeyNotRegistered = 3401,
    /// Indicates the proof failed UltraHonk verification.
    InvalidProof = 3402,
}

// ################## EVENTS ##################

/// Event emitted when a new verification key is registered.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationKeyRegistered {
    #[topic]
    pub circuit_type: CircuitType,
    pub vk: Bytes,
}

/// Emits an event indicating a verification key has been registered.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `circuit_type` - The circuit the key was registered under.
/// * `vk` - The serialized UltraHonk verification key.
pub fn emit_verification_key_registered(e: &Env, circuit_type: CircuitType, vk: &Bytes) {
    VerificationKeyRegistered { circuit_type, vk: vk.clone() }.publish(e);
}

/// Event emitted when a verification key is updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationKeyUpdated {
    #[topic]
    pub circuit_type: CircuitType,
    pub old_vk: Bytes,
    pub new_vk: Bytes,
}

/// Emits an event indicating a verification key has been updated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `circuit_type` - The circuit whose key was updated.
/// * `old_vk` - The previously registered verification key.
/// * `new_vk` - The newly registered verification key.
pub fn emit_verification_key_updated(
    e: &Env,
    circuit_type: CircuitType,
    old_vk: &Bytes,
    new_vk: &Bytes,
) {
    VerificationKeyUpdated { circuit_type, old_vk: old_vk.clone(), new_vk: new_vk.clone() }
        .publish(e);
}
