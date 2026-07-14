use soroban_sdk::{Address, Env, String};

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
