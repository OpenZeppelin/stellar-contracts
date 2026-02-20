#![no_std]

// Ensure soroban-sdk's panic handler is linked for cdylib builds.
extern crate soroban_sdk;

#[cfg(feature = "crypto")]
pub mod crypto;
#[cfg(feature = "math")]
pub mod math;
#[cfg(feature = "merkle-distributor")]
pub mod merkle_distributor;
#[cfg(feature = "pausable")]
pub mod pausable;
#[cfg(feature = "upgradeable")]
pub mod upgradeable;
