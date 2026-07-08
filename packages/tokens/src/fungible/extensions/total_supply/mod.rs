//! # Total Supply Extension for Fungible Token.
//!
//! Tracking the total supply is not required by SEP-41 and is therefore not
//! part of the base [`crate::fungible::FungibleToken`] trait. Keeping a
//! global supply counter has a scalability cost: every mint and burn writes
//! the same storage entry, and a transaction writing an entry cannot execute
//! in parallel with transactions reading it. Tokens that do not need the
//! counter are better off without it, which is why supply tracking is
//! provided as this opt-in extension.
//!
//! The extension consists of two parts:
//!
//! - [`FungibleTotalSupply`]: exposes the `total_supply()` function on the
//!   contract.
//! - A supply-aware contract type, so that burns performed through
//!   [`crate::fungible::burnable::FungibleBurnable`] decrease the supply:
//!   [`TotalSupply`] for the vanilla behavior, or the combined contract types
//!   in [`crate::fungible::combinations`] to pair tracking with the allowlist
//!   or blocklist policy.
//!
//! Minting has to go through [`mint`] (instead of
//! [`crate::fungible::Base::mint`]) for the supply to be increased.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     type ContractType = TotalSupply;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleTotalSupply for MyToken {}
//! ```
//!
//! The [`crate::rwa::RWA`], [`crate::vault::Vault`] and
//! [`crate::fungible::votes::FungibleVotes`] contract types are inherently
//! supply-aware; [`FungibleTotalSupply`] can be implemented on top of them
//! directly.
//!
//! The supply is stored in its own `persistent` entry, ensuring that mints
//! and burns only conflict with each other and never with plain transfers.

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracttrait, Env};
pub use storage::{
    decrease_total_supply, increase_total_supply, mint, total_supply, TotalSupply,
    TotalSupplyStorageKey,
};

// The trait is defined alongside its siblings (`ContractOverrides`,
// `BurnableOverrides`) in the private `overrides` module; this re-export is
// its public path.
pub use crate::fungible::overrides::TotalSupplyOverrides;
use crate::fungible::FungibleToken;

/// Total Supply Trait for Fungible Token
///
/// The `FungibleTotalSupply` trait extends the `FungibleToken` trait to
/// expose the total amount of tokens in circulation.
///
/// This trait can only be implemented when the contract's `ContractType`
/// accounts for the total supply:
///
/// * [`TotalSupply`] (vanilla behavior),
/// * [`crate::fungible::combinations::TotalSupplyAllowList`] (allowlist
///   transfer policy),
/// * [`crate::fungible::combinations::TotalSupplyBlockList`] (blocklist
///   transfer policy),
/// * [`crate::rwa::RWA`],
/// * [`crate::vault::Vault`],
/// * [`crate::fungible::votes::FungibleVotes`] (the supply is served from the
///   voting checkpoints).
///
/// When using one of the `TotalSupply*` contract types, minting has to be
/// performed with [`mint`] so that the supply is increased; burns through
/// [`crate::fungible::burnable::FungibleBurnable`] decrease it automatically.
#[contracttrait]
pub trait FungibleTotalSupply: FungibleToken<ContractType: TotalSupplyOverrides> {
    /// Returns the total amount of tokens in circulation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn total_supply(e: &Env) -> i128 {
        Self::ContractType::total_supply(e)
    }
}
