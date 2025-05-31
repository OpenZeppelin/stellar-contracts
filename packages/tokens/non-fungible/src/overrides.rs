use soroban_sdk::{Address, Env, String};

/// Based on the extension, some default behavior of [`crate::NonFungibleToken`]
/// might have to be overridden. This is a helper trait that allows us this
/// override mechanism that favors the DevX.
///
/// One can also override the `NonFungibleToken` trait directly, but the reason
/// we have another helper trait for the same methods, is to provide the default
/// implementations in an easier way for the developers.
///
/// The way to provide different default implementations for different
/// extensions is by implementing the trait for different types (unit structs).
/// The problem is, `NonFungibleToken` trait has to be implemented for the smart
/// contract (which is another struct) by the developers. So, we need a level
/// of abstraction by introducing an associated type, which will grant
/// `NonFungibleToken` trait the ability to switch between different default
/// implementations by calling the methods on this associated type.
///
/// By introducing this abstraction, we allow the developers to implement
/// every method of the `NonFungibleToken` trait using
/// `Self::ContractType::{function_name}`, which will in turn use either the
/// overridden or the base variant according to the extension, provided by the
/// `ContractOverrides` trait implementation for the respective `ContractType`.
///
/// Example:
///
/// ```rust
/// impl NonFungibleToken for ExampleContract {
///     type ContractType = Consecutive;
///
///     fn balance(e: &Env, owner: &Address) -> u32 {
///         Self::ContractType::balance(e, owner)
///     }
///
///     fn owner_of(e: &Env, token_id: u32) -> &Address {
///         Self::ContractType::owner_of(e, token_id)
///     }
///
///     fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
///         Self::ContractType::transfer(e, from, to, token_id);
///     }
///
///     fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
///         Self::ContractType::transfer_from(e, spender, from, to, token_id);
///     }
///
///     /* and so on */
/// }
/// ```
///
/// or the type can be used directly (in this case `Consecutive`)
/// instead of referring to it as `Self::ContractType`:
///
/// ```rust
/// impl NonFungibleToken for ExampleContract {
///     type ContractType = Consecutive;
///
///     fn balance(e: &Env, owner: &Address) -> u32 {
///         Consecutive::balance(e, owner)
///     }
///
///     fn owner_of(e: &Env, token_id: u32) -> &Address {
///         Consecutive::owner_of(e, token_id)
///     }
///
///     fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
///         Consecutive:transfer(e, from, to, token_id);
///     }
///
///     fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
///         Consecutive::transfer_from(e, spender, from, to, token_id);
///     }
///
///     /* and so on */
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
