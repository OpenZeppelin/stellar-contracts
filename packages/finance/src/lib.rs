//! # Stellar Finance
//!
//! Finance primitives for Soroban contracts.
//!
//! - [`vesting`]: a vesting contract that holds SEP-41 tokens and releases them
//!   to a single receiver on a fixed schedule.

#![no_std]

pub mod vesting;
