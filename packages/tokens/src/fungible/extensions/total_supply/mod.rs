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
//!   [`TotalSupply`] for the vanilla behavior, or a combination resolved by
//!   [`crate::fungible::combinations::Compose`] (e.g. `Compose<(AllowList,
//!   TotalSupply)>`) to pair tracking with the allowlist policy, the blocklist
//!   policy, or both.
//!
//! Minting has to be routed through the contract type
//! (`Self::ContractType::mint`, resolving to [`TotalSupply::mint`] or to the
//! combined contract type's `mint`) instead of [`crate::fungible::Base::mint`]
//! for the supply to be increased.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl]
//! impl MyToken {
//!     #[only_owner]
//!     pub fn mint(e: &Env, to: Address, amount: i128) {
//!         // outside a trait impl, `Self::ContractType` has to be spelled out
//!         <Self as FungibleToken>::ContractType::mint(e, &to, amount);
//!     }
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     type ContractType = Compose<(TotalSupply,)>;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleTotalSupply for MyToken {}
//! ```
//!
//! The [`crate::rwa::RWA`] and [`crate::vault::Vault`] contract types track
//! the supply as part of their own accounting. They require `TotalSupply`
//! next to them in the list, e.g. `Compose<(RWA, TotalSupply)>`, and
//! [`crate::rwa::RWAToken`] and [`crate::vault::FungibleVault`] require
//! [`FungibleTotalSupply`] as well.
//! [`crate::fungible::votes::FungibleVotes`] tracks voting units, which are
//! not the token supply, so it does not back this extension.
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
/// Whether the supply is tracked is decided by the contract type selected on
/// the `FungibleToken` implementation. `Compose<(Base,)>` states that no
/// behavior is overridden, so the supply is not tracked and this trait cannot
/// be implemented. [`TotalSupply`] is the contract type that adds the supply
/// tracking: `Compose<(TotalSupply,)>` on its own, or listed together with
/// [`crate::fungible::allowlist::AllowList`] and
/// [`crate::fungible::blocklist::BlockList`] in any of their valid
/// combinations (refer to [`crate::fungible::combinations::Compose`]).
///
/// [`crate::rwa::RWA`] and [`crate::vault::Vault`] track the supply as part
/// of their own accounting. They require `TotalSupply` next to them in the
/// list, e.g. `Compose<(RWA, TotalSupply)>`, and this trait alongside:
/// [`crate::rwa::RWAToken`] and [`crate::vault::FungibleVault`] have it as a
/// supertrait, so implementing them without it does not compile.
///
/// [`crate::fungible::votes::FungibleVotes`] (alone or combined with the list
/// policies) tracks voting units rather than the token supply and does not
/// back this trait.
///
/// When `TotalSupply` is part of the contract type, minting has to be routed
/// through it (`Self::ContractType::mint`) so that the supply is increased;
/// burns through [`crate::fungible::burnable::FungibleBurnable`] decrease it
/// automatically.
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
