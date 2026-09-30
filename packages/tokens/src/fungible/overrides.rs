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
///     type ContractType = Compose<(Base,)>;
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

/// Internal override hook for minting.
///
/// # Why this trait exists
///
/// Minting is not part of any public trait: the entry point is written by the
/// contract author, because its signature and authorization vary per contract
/// (`#[only_owner]` with no extra argument, a role check with an `operator`
/// argument, ...). Only the underlying primitive, "create `amount` tokens for
/// `to`", is the same everywhere, and what it has to do depends on the
/// contract type: `Base` only updates the balance, `TotalSupply` also
/// increases the supply counter, `FungibleVotes` also moves voting units,
/// `Capped` also checks the cap, `RWA` also runs its compliance checks.
///
/// This trait gives that primitive a single dispatch point. The author's
/// entry point calls `Self::ContractType::mint`, which reaches the mint of
/// whatever contract type `Compose` resolved to:
///
/// ```ignore
/// #[only_owner]
/// pub fn mint(e: &Env, to: Address, amount: i128) {
///     <Self as FungibleToken>::ContractType::mint(e, &to, amount);
/// }
/// ```
///
/// Calling a specific primitive such as `Base::mint` instead is the mistake
/// this protects against: on a supply-tracking contract type it would skip
/// the supply counter (and a later burn would panic on underflow), and on a
/// capped contract type it would skip the cap.
///
/// # Note
///
/// Like [`ContractOverrides`], this trait is internal plumbing of the
/// library. As a contract author there is no need to implement it, name it,
/// or import it. Every fungible contract type that can mint freely keeps an
/// inherent `mint`, and in the contract `Self::ContractType` is already the
/// concrete type `Compose` resolved to, so the call above reaches that
/// inherent function directly. (Importing this trait there would even be
/// flagged as an unused import.)
///
/// # What the trait adds
///
/// Choosing the right mint is done by the `ContractType` associated type, not
/// by this trait. The trait is the checked promise behind that call: every
/// implementing contract type has a mint with this exact signature, and code
/// that is generic over the contract type (where inherent functions are not
/// reachable) can mint through it. Each implementation delegates to the
/// type's inherent `mint`, so both paths run the same code.
///
/// The method is required, with no default body, on purpose: a default
/// falling back to `Base::mint` would let a new supply-tracking contract type
/// compile while silently skipping its own bookkeeping.
///
/// [`crate::vault::Vault`] does not implement this trait. Vault shares are
/// only created against deposited assets (`deposit` and `mint` of
/// [`crate::vault::FungibleVault`]), and a free share mint would dilute every
/// depositor.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no free mint",
    note = "vault shares are only created against deposited assets, through the `deposit` and \
            `mint` entry points of `FungibleVault`"
)]
pub trait MintOverrides {
    fn mint(e: &Env, to: &Address, amount: i128);
}

impl MintOverrides for Base {
    fn mint(e: &Env, to: &Address, amount: i128) {
        Base::mint(e, to, amount);
    }
}

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
/// contract types ([`crate::fungible::total_supply::TotalSupply`], the
/// combined contract types resolved by
/// [`crate::fungible::combinations::Compose`], `RWA`, `Vault`, `Capped` and
/// the capped combinations).
///
/// Unlike `BurnableOverrides`, there is deliberately no implementation for
/// [`Base`]: exposing the total supply requires a supply-tracking contract
/// type.
pub trait TotalSupplyOverrides {
    fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }
}
