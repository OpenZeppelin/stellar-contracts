use soroban_sdk::{Address, Env, String};

/// Based on the extension, some default behavior of
/// [`crate::fungible::FungibleToken`] might have to be overridden. This is a
/// helper trait that allows us this override mechanism that favors the DevX.
///
/// One can also override the `FungibleToken` trait directly, but the reason
/// we have another helper trait for the same methods, is to provide the default
/// implementations in an easier way for the developers.
///
/// The way to provide different default implementations for different
/// extensions is by implementing the trait for different types (unit structs).
/// The problem is, `FungibleToken` trait has to be implemented for the smart
/// contract (which is another struct) by the developers. So, we need a level
/// of abstraction by introducing an associated type, which will grant
/// `FungibleToken` trait the ability to switch between different default
/// implementations by calling the methods on this associated type.
///
/// By introducing this abstraction, we allow the developers to implement
/// every method of the `FungibleToken` trait using
/// `Self::ContractType::{function_name}`, which will in turn use either the
/// overridden or the base variant according to the extension, provided by the
/// `ContractOverrides` trait implementation for the respective `ContractType`.
///
/// Example:
///
/// ```rust
/// impl FungibleToken for ExampleContract {
///     type ContractType = Base;
///
///     fn balance(e: &Env, account: Address) -> i128 {
///         Self::ContractType::balance(e, &account)
///     }
///
///     fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
///         Self::ContractType::transfer(e, &from, &to, amount);
///     }
///
///     /* and so on */
/// }
/// ```
///
/// or the type can be used directly (in this case `Base`)
/// instead of referring to it as `Self::ContractType`:
///
/// ```rust
/// impl FungibleToken for ExampleContract {
///     type ContractType = Base;
///
///     fn balance(e: &Env, account: Address) -> i128 {
///         Base::balance(e, &account)
///     }
///
///     fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
///         Base::transfer(e, &from, &to, amount);
///     }
///
///     /* and so on */
/// }
/// ```
pub trait ContractOverrides {
    fn total_supply(e: &Env) -> i128 {
        Base::total_supply(e)
    }

    fn balance(e: &Env, account: &Address) -> i128 {
        Base::balance(e, account)
    }

    fn allowance(e: &Env, owner: &Address, spender: &Address) -> i128 {
        Base::allowance(e, owner, spender)
    }

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
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
