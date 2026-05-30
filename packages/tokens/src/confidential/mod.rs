//! # Confidential Token Module
//!
//! Building blocks for the confidential token wrapper, which encrypts balances
//! and transfer amounts under Grumpkin ElGamal commitments and proves
//! correctness via zero-knowledge circuits.
//!
//! ## Modules
//!
//! - **Auditor Registry**: per-`auditor_id` Grumpkin public key store consumed
//!   by the wrapper to build auditor ECDH ciphertexts on withdraw, transfer,
//!   and operator flows.
//! - **Verifier**: per-`CircuitType` UltraHonk verification key store consumed
//!   by the wrapper to verify proofs accompanying every state-changing
//!   operation (register, withdraw, transfer, operator flows).
//! - **Wrapper**: the confidential token wrapper itself — holds the SEP-41
//!   balance, manages per-account encrypted state, and delegates proof
//!   verification and auditor-key lookup to the two registries above.

pub mod auditor;
pub mod verifier;
pub mod wrapper;
