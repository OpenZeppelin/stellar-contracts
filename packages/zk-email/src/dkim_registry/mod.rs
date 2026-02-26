//! # DKIM Registry Module
//!
//! This module provides the foundational trust anchor for the zkEmail
//! ecosystem. It stores which DKIM public key hashes are valid for which email
//! domains. All downstream modules (TBD) query the registry via
//! [`is_key_hash_valid`].
//!
//! The registry is hash-function agnostic: it stores pre-hashed `BytesN<32>`
//! values for both domain names and public keys. Callers hash these off-chain
//! using whatever hash function their system requires (Poseidon, SHA256,
//! Keccak256, etc.).
//!
//! # Usage
//!
//! ```ignore
//! use stellar_macros::only_role;
//! use stellar_zk_email::dkim_registry::{self, DKIMRegistry};
//!
//! #[contract]
//! pub struct MyRegistry;
//!
//! impl DKIMRegistry for MyRegistry {
//!     #[only_role(operator, "governance")]
//!     fn set_dkim_public_key_hash(
//!         e: &Env,
//!         domain_hash: BytesN<32>,
//!         public_key_hash: BytesN<32>,
//!         operator: Address,
//!     ) {
//!         dkim_registry::set_dkim_public_key_hash(e, &domain_hash, &public_key_hash);
//!     }
//!
//!     #[only_role(operator, "governance")]
//!     fn set_dkim_public_key_hashes(
//!         e: &Env,
//!         domain_hash: BytesN<32>,
//!         public_key_hashes: Vec<BytesN<32>>,
//!         operator: Address,
//!     ) {
//!         dkim_registry::set_dkim_public_key_hashes(e, &domain_hash, &public_key_hashes);
//!     }
//!
//!     #[only_role(operator, "governance")]
//!     fn revoke_dkim_public_key_hash(
//!         e: &Env,
//!         public_key_hash: BytesN<32>,
//!         operator: Address,
//!     ) {
//!         dkim_registry::revoke_dkim_public_key_hash(e, &public_key_hash);
//!     }
//! }
//! ```

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, Address, BytesN, Env, Vec};

pub use crate::dkim_registry::storage::{
    is_key_hash_revoked, is_key_hash_valid, revoke_dkim_public_key_hash, set_dkim_public_key_hash,
    set_dkim_public_key_hashes, DKIMKeyEntry, DKIMRegistryStorageKey,
};

// ################## TRAIT ##################

/// Trait for DKIM public key hash registry contracts.
///
/// Implements the registry interface from
/// [ERC-7969 (IDKIMRegistry)](https://eips.ethereum.org/EIPS/eip-7969).
pub trait DKIMRegistry {
    /// Returns true if the key hash is registered for the domain AND not
    /// revoked.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `domain_hash` - Hash of the email domain.
    /// * `public_key_hash` - Hash of the DKIM public key.
    fn is_key_hash_valid(e: &Env, domain_hash: BytesN<32>, public_key_hash: BytesN<32>) -> bool {
        is_key_hash_valid(e, &domain_hash, &public_key_hash)
    }

    /// Returns true if the key hash has been globally revoked.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `public_key_hash` - Hash of the DKIM public key.
    fn is_key_hash_revoked(e: &Env, public_key_hash: BytesN<32>) -> bool {
        is_key_hash_revoked(e, &public_key_hash)
    }

    /// Registers a DKIM public key hash for a domain.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `domain_hash` - Hash of the email domain.
    /// * `public_key_hash` - Hash of the DKIM public key.
    /// * `operator` - The address performing the operation.
    ///
    /// # Notes
    ///
    /// It is recommended to use [`set_dkim_public_key_hash`] when implementing
    /// this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of [`set_dkim_public_key_hash`]
    /// intentionally lacks authorization controls. Proper authorization must be
    /// implemented in the contract.
    fn set_dkim_public_key_hash(
        e: &Env,
        domain_hash: BytesN<32>,
        public_key_hash: BytesN<32>,
        operator: Address,
    );

    /// Batch registers DKIM public key hashes for a domain.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `domain_hash` - Hash of the email domain.
    /// * `public_key_hashes` - Hashes of the DKIM public keys.
    /// * `operator` - The address performing the operation.
    ///
    /// # Notes
    ///
    /// It is recommended to use [`set_dkim_public_key_hashes`] when
    /// implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of [`set_dkim_public_key_hashes`]
    /// intentionally lacks authorization controls. Proper authorization must be
    /// implemented in the contract.
    fn set_dkim_public_key_hashes(
        e: &Env,
        domain_hash: BytesN<32>,
        public_key_hashes: Vec<BytesN<32>>,
        operator: Address,
    );

    /// Globally revokes a DKIM public key hash. Once revoked, it cannot be
    /// re-registered.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `public_key_hash` - Hash of the DKIM public key.
    /// * `operator` - The address performing the operation.
    ///
    /// # Notes
    ///
    /// It is recommended to use [`revoke_dkim_public_key_hash`] when
    /// implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of
    /// [`revoke_dkim_public_key_hash`] intentionally lacks authorization
    /// controls. Proper authorization must be implemented in the contract.
    fn revoke_dkim_public_key_hash(e: &Env, public_key_hash: BytesN<32>, operator: Address);
}

// ################## ERRORS ##################

/// Errors that can occur in DKIM registry operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DKIMRegistryError {
    /// The public key hash has been revoked and cannot be re-registered.
    KeyHashRevoked = 5100,
    /// The public key hash is already registered for the given domain.
    KeyHashAlreadyRegistered = 5101,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for DKIM registry storage entries (in ledgers).
pub const DKIM_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// TTL threshold for extending DKIM registry storage entries (in ledgers).
pub const DKIM_TTL_THRESHOLD: u32 = DKIM_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when a DKIM public key hash is registered for a domain.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyHashRegistered {
    #[topic]
    pub domain_hash: BytesN<32>,
    pub public_key_hash: BytesN<32>,
}

/// Emits a [`KeyHashRegistered`] event.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `domain_hash` - Hash of the email domain.
/// * `public_key_hash` - Hash of the DKIM public key.
pub fn emit_key_hash_registered(e: &Env, domain_hash: &BytesN<32>, public_key_hash: &BytesN<32>) {
    KeyHashRegistered {
        domain_hash: domain_hash.clone(),
        public_key_hash: public_key_hash.clone(),
    }
    .publish(e);
}

/// Event emitted when a DKIM public key hash is globally revoked.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyHashRevoked {
    #[topic]
    pub public_key_hash: BytesN<32>,
}

/// Emits a [`KeyHashRevoked`] event.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `public_key_hash` - Hash of the DKIM public key.
pub fn emit_key_hash_revoked(e: &Env, public_key_hash: &BytesN<32>) {
    KeyHashRevoked { public_key_hash: public_key_hash.clone() }.publish(e);
}
