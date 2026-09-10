use soroban_sdk::{Address, Env, String};

use crate::non_fungible::royalties::{
    remove_token_royalty_unchecked, royalty_info_unchecked, set_token_royalty_unchecked,
};

/// Internal override hook for [`crate::non_fungible::NonFungibleToken`].
///
/// # Note
///
/// This trait is internal plumbing of the library. As a contract author there
/// is no need to implement it, name it, or import it. It is documented here
/// only to explain how behavior is routed under the hood.
///
/// Some extensions need to change the default behavior of `NonFungibleToken`
/// (for example, `Enumerable` and `Consecutive` keep extra bookkeeping).
/// Instead of forcing every contract to re-wire those methods by hand, the
/// behavior is keyed off the `ContractType` associated type: each
/// `NonFungibleToken` method delegates to
/// `Self::ContractType::{function_name}`, and this trait's implementation for
/// that type decides whether to run the base logic or an override. The
/// library ships implementations for its contract types (`Base`,
/// `Enumerable`, `Consecutive`, ...).
///
/// From a contract author's point of view this is invisible. A `ContractType`
/// is picked on the `NonFungibleToken` implementation and the bodies are left
/// empty; the `#[contractimpl(contracttrait)]` macro fills them in and the
/// correct behavior is selected automatically:
///
/// ```rust
/// #[contractimpl(contracttrait)]
/// impl NonFungibleToken for ExampleContract {
///     type ContractType = Compose<(Consecutive,)>;
/// }
/// ```
pub trait ContractOverrides {
    fn balance(e: &Env, owner: &Address) -> u32 {
        Base::balance(e, owner)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        Base::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Base::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Base::transfer_from(e, spender, from, to, token_id);
    }

    fn approve(
        e: &Env,
        approver: &Address,
        approved: &Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Base::approve(e, approver, approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: &Address, operator: &Address, live_until_ledger: u32) {
        Base::approve_for_all(e, owner, operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        Base::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: &Address, operator: &Address) -> bool {
        Base::is_approved_for_all(e, owner, operator)
    }

    fn name(e: &Env) -> String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol(e)
    }

    fn token_uri(e: &Env, token_id: u32) -> String {
        Base::token_uri(e, token_id)
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
/// [`crate::non_fungible::burnable::NonFungibleBurnable`] with an empty body is
/// enough, and the right burn behavior is picked based on the contract's
/// `ContractType`. The behavior of `burn` and `burn_from` changes across
/// implementations (e.g. enumerable, consecutive), hence the need for this
/// abstraction.
pub trait BurnableOverrides {
    fn burn(e: &Env, from: &Address, token_id: u32) {
        Base::burn(e, from, token_id);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        Base::burn_from(e, spender, from, token_id);
    }
}

impl BurnableOverrides for Base {}

/// Contract-type-dependent entry points of the royalties extension.
///
/// Every royalty operation must first establish that the token
/// exists, and what "exists" means is decided by the contract type's
/// ownership model. [`Base`] materializes an `Owner` entry for every token,
/// so its check is a direct lookup.
/// [`crate::non_fungible::consecutive::Consecutive`] stores ownership
/// sparsely (one entry per batch boundary) and resolves the rest by scanning
/// its ownership buckets, so an existence check hard-wired to
/// [`Base::owner_of`] would reject almost every token of a consecutive
/// collection. The check must therefore be routed through
/// `ContractType::owner_of`.
///
/// This trait is how that routing is made reachable from everywhere it is
/// needed:
///
/// 1. For [`crate::non_fungible::royalties::NonFungibleRoyalties::royalty_info`] alone, no extra machinery
///    would be required. The method has a default implementation, and a default
///    body can already reach the correct check through `Self::ContractType`,
///    whose [`ContractOverrides`] bound carries `owner_of`.
///
/// 2. [`crate::non_fungible::royalties::NonFungibleRoyalties::set_token_royalty`] and
///    [`crate::non_fungible::royalties::NonFungibleRoyalties::remove_token_royalty`] deliberately have no
///    default implementations: they are privileged operations, and the access
///    control is up to the implementing contract. Their bodies are written by
///    the contract author, so the correctly-routed logic must be callable from
///    the author's own code, and the only name available there that knows the
///    ownership model is `Self::ContractType`.
///
/// 3. A function becomes callable on `Self::ContractType` by being defined on a
///    trait that `ContractType` is bound by. [`ContractOverrides`] is not
///    extended with royalty functions, because royalty logic belongs to this
///    extension. Instead, this trait carries the royalty entry points, requires
///    [`ContractOverrides`] as a supertrait so that its default bodies can call
///    `Self::owner_of`, and
///    [`crate::non_fungible::royalties::NonFungibleRoyalties`] bounds
///    `ContractType` by it.
///
/// Like the other dispatch traits, the definition lives in this file with
/// the rest of the plumbing, but unlike them it must be nameable by
/// contract authors, so it is re-exported at
/// `crate::non_fungible::royalties::RoyaltySupport`, next to the trait
/// whose implementations call it.
///
/// # "Support", not "Overrides"
///
/// Unlike `BurnableOverrides` (the internal dispatch behind
/// [`crate::non_fungible::burnable::NonFungibleBurnable`]), where the burn
/// logic genuinely differs across contract types, nothing here is ever
/// overridden. The royalty logic is identical for every contract type; the
/// only type-dependent piece is the `owner_of` existence check, which the
/// default bodies obtain through the supertrait. Every implementation of
/// this trait is therefore empty: implementing it declares that the
/// royalties extension can be used with a contract type, nothing more. A
/// contract type defined outside this library opts in the same way, with an
/// empty `impl`.
///
/// # Usage
///
/// The privileged functions are called on `Self::ContractType` after access
/// control:
///
/// ```ignore
/// use stellar_tokens::non_fungible::royalties::{NonFungibleRoyalties, RoyaltySupport};
///
/// #[contractimpl(contracttrait)]
/// impl NonFungibleRoyalties for MyContract {
///     #[only_admin]
///     fn set_token_royalty(
///         e: &Env,
///         token_id: u32,
///         receiver: Address,
///         basis_points: u32,
///         operator: Address,
///     ) {
///         Self::ContractType::set_token_royalty(e, token_id, &receiver, basis_points);
///     }
///
///     // ...
/// }
/// ```
pub trait RoyaltySupport: ContractOverrides {
    fn set_token_royalty(e: &Env, token_id: u32, receiver: &Address, basis_points: u32) {
        // Verify token exists, through this contract type's ownership model
        let _ = Self::owner_of(e, token_id);

        set_token_royalty_unchecked(e, token_id, receiver, basis_points);
    }

    fn remove_token_royalty(e: &Env, token_id: u32) {
        // Verify token exists, through this contract type's ownership model
        let _ = Self::owner_of(e, token_id);

        remove_token_royalty_unchecked(e, token_id);
    }

    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
        // Verify token exists, through this contract type's ownership model
        let _ = Self::owner_of(e, token_id);

        royalty_info_unchecked(e, token_id, sale_price)
    }
}

impl RoyaltySupport for Base {}
