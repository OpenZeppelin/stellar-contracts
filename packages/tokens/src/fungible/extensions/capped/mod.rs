//! # Capped Extension for Fungible Token.
//!
//! Enforces a maximum total supply on minting. The cap is compared against
//! the tracked total supply, so this extension builds on
//! [`crate::fungible::total_supply::FungibleTotalSupply`]:
//! [`FungibleCapped`] has it as a supertrait, and implementing
//! [`FungibleCapped`] on a contract that does not expose the supply does not
//! compile.
//!
//! The extension consists of two parts:
//!
//! - The [`Capped`] contract type, selected by listing `Capped` in the
//!   [`crate::fungible::combinations::Compose`] list. Its mint checks the cap
//!   and then mints through the supply counter. [`Capped::set_cap`] is best
//!   called once, in the constructor, so holders can rely on a fixed cap. If
//!   the token owner or governance needs to adjust the cap later, it can also
//!   be called from an access-controlled function. The contract's own `mint`
//!   function mints through `Self::ContractType::mint`, which resolves to the
//!   capped mint, so the cap is enforced without being called explicitly.
//! - [`FungibleCapped`]: exposes the `cap()` function on the contract. It can
//!   only be implemented when the resolved contract type enforces the cap
//!   (refer to [`CappedContractType`]), so a contract cannot report a cap it
//!   does not enforce.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl]
//! impl MyToken {
//!     pub fn __constructor(e: &Env, cap: i128) {
//!         Capped::set_cap(e, cap);
//!     }
//!
//!     #[only_owner]
//!     pub fn mint(e: &Env, to: Address, amount: i128) {
//!         <Self as FungibleToken>::ContractType::mint(e, &to, amount);
//!     }
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     type ContractType = Compose<(Capped, TotalSupply)>;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleTotalSupply for MyToken {}
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleCapped for MyToken {}
//! ```
//!
//! In the [`crate::fungible::combinations::Compose`] list, `Capped` requires
//! `TotalSupply` next to it, like `RWA` and `Vault`, and a list with `Capped`
//! but without `TotalSupply` is rejected with a dedicated compile error.
//! `Compose<(Capped, TotalSupply)>` resolves to [`Capped`]; `Capped` combined
//! with `AllowList`, `BlockList` (or both), `RWA` or `Vault` resolves to the
//! matching capped combination, e.g. `Compose<(AllowList, Capped,
//! TotalSupply)>` to `CappedAllowList`.
//!
//! On a capped vault, the cap bounds the supply of shares: it is enforced on
//! `deposit` and `mint`, and reflected in `max_deposit` and `max_mint`.

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracttrait, Env};
pub use storage::{check_cap, query_cap, set_cap, CapStorageKey, Capped, CappedContractType};

use crate::fungible::{total_supply::FungibleTotalSupply, FungibleToken};

/// Capped Trait for Fungible Token
///
/// The `FungibleCapped` trait extends the `FungibleTotalSupply` trait to
/// expose the maximum total supply of the token.
///
/// The cap is checked against the total supply, so this trait can only be
/// implemented alongside [`FungibleTotalSupply`]. It can also only be
/// implemented when the contract's `ContractType` enforces the cap (refer to
/// [`CappedContractType`]), i.e. when `Capped` is in the `Compose` list.
#[contracttrait]
pub trait FungibleCapped:
    FungibleTotalSupply + FungibleToken<ContractType: CappedContractType>
{
    /// Returns the maximum total supply of tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`query_cap`] errors.
    fn cap(e: &Env) -> i128 {
        Capped::cap(e)
    }
}
