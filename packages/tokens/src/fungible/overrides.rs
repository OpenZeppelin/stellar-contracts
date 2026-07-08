use soroban_sdk::{Address, Env, MuxedAddress, String};

use crate::fungible::extensions::total_supply::total_supply;

/// Internal override hook for [`crate::fungible::FungibleToken`].
///
/// # Note
///
/// This trait is internal plumbing of the library. As a contract author there
/// is no need to implement it, name it, or import it. It is documented here
/// only to explain how behavior is routed under the hood.
///
/// Some extensions need to change the default behavior of `FungibleToken`
/// (for example, `AllowList` and `BlockList` gate transfers). Instead of
/// forcing every contract to re-wire those methods by hand, the behavior is
/// keyed off the `ContractType` associated type: each `FungibleToken` method
/// delegates to `Self::ContractType::{function_name}`, and this trait's
/// implementation for that type decides whether to run the base logic or an
/// override. The library ships implementations for its contract types
/// (`Base`, `AllowList`, `BlockList`, `RWA`, `Vault`, ...).
///
/// From a contract author's point of view this is invisible. A `ContractType`
/// is picked on the `FungibleToken` implementation and the bodies are left
/// empty; the `#[contractimpl(contracttrait)]` macro fills them in and the
/// correct behavior is selected automatically:
///
/// ```rust
/// #[contractimpl(contracttrait)]
/// impl FungibleToken for ExampleContract {
///     type ContractType = Base;
/// }
/// ```
pub trait ContractOverrides {
    fn balance(e: &Env, account: &Address) -> i128 {
        Base::balance(e, account)
    }

    fn allowance(e: &Env, owner: &Address, spender: &Address) -> i128 {
        Base::allowance(e, owner, spender)
    }

    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        Base::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Base::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Base::approve(e, owner, spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Base::decimals(e)
    }

    fn name(e: &Env) -> String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol(e)
    }
}

/// Default marker type
pub struct Base;

// No override required for the `Base` contract type.
impl ContractOverrides for Base {}

/// Internal override hook for `burn` and `burn_from`.
///
/// # Note
///
/// Like [`ContractOverrides`], this trait is internal plumbing of the
/// library. There is no need to implement or import it: implementing
/// [`crate::fungible::burnable::FungibleBurnable`] with an empty body is
/// enough, and the right burn behavior is picked based on the contract's
/// `ContractType`. The behavior of `burn` and `burn_from` changes across
/// implementations (e.g. blocklist, allowlist), hence the need for this
/// abstraction.
pub trait BurnableOverrides {
    fn burn(e: &Env, from: &Address, amount: i128) {
        Base::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Base::burn_from(e, spender, from, amount);
    }
}

impl BurnableOverrides for Base {}

/// Internal override hook for
/// [`crate::fungible::total_supply::FungibleTotalSupply`].
///
/// # Note
///
/// Like [`ContractOverrides`], this trait is internal plumbing of the
/// library. There is no need to implement or import it: implementing
/// [`crate::fungible::total_supply::FungibleTotalSupply`] with an empty body
/// is enough, and the right behavior is picked based on the contract's
/// `ContractType`. The library ships implementations for its supply-aware
/// contract types ([`crate::fungible::total_supply::TotalSupply`],
/// [`crate::fungible::combinations::TotalSupplyAllowList`],
/// [`crate::fungible::combinations::TotalSupplyBlockList`], `RWA`, `Vault`,
/// `FungibleVotes`).
///
/// Unlike `BurnableOverrides`, there is deliberately no implementation for
/// [`Base`]: exposing the total supply requires a supply-tracking contract
/// type.
pub trait TotalSupplyOverrides {
    fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }
}
