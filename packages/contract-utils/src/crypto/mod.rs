//! # Cryptographic Utilities
//!
//! **Feature**: This module requires the `crypto` feature flag.
//!
//! Provides cryptographic utilities including hash functions and Merkle tree
//! verification for Soroban smart contracts.

pub mod error;
pub mod hashable;
pub mod hasher;
pub mod keccak;
pub mod merkle;
pub mod sha256;

#[cfg(test)]
mod test;
