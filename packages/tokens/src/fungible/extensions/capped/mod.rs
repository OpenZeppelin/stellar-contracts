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
//! - [`FungibleCapped`]: exposes the `cap()` function on the contract.
//! - The [`Capped`] type: [`Capped::set_cap`] in the constructor and
//!   [`Capped::mint`] in the contract's own `mint` function, which checks the
//!   cap and then mints through the supply counter. Minting is not part of any
//!   trait, so the check is not automatic. Contract types with a mint of their
//!   own, such as `RWA`, call [`check_cap`] before that mint instead.
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
//!         Capped::mint(e, &to, amount);
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
//! In the [`crate::fungible::combinations::Compose`] list, `Capped` is
//! additive but requires `TotalSupply` next to it, like `RWA` and `Vault`:
//! `Compose<(Capped, TotalSupply)>` resolves to
//! [`crate::fungible::total_supply::TotalSupply`], `Compose<(AllowList,
//! Capped, TotalSupply)>` to the allowlist supply-tracking combination, and a
//! list with `Capped` but without `TotalSupply` is rejected with a dedicated
//! compile error.

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracttrait, Env};
pub use storage::{check_cap, query_cap, set_cap, CapStorageKey, Capped};

use crate::fungible::total_supply::FungibleTotalSupply;

/// Capped Trait for Fungible Token
///
/// The `FungibleCapped` trait extends the `FungibleTotalSupply` trait to
/// expose the maximum total supply of the token.
///
/// The cap is checked against the total supply, so this trait can only be
/// implemented alongside [`FungibleTotalSupply`]. The check itself is
/// performed by [`Capped::mint`] in the contract's `mint` function.
#[contracttrait]
pub trait FungibleCapped: FungibleTotalSupply {
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
