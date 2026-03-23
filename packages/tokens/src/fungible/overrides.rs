use soroban_sdk::{Address, Env, MuxedAddress, String};

/// Based on the extension, some default behavior of
/// [`crate::fungible::FungibleToken`] might have to be overridden. This is a
/// helper trait that provides this override mechanism in a developer-friendly
/// way.
///
/// The `FungibleToken` trait can also be overridden directly, but this helper
/// trait exists to provide the default implementations in a simpler way.
///
/// The way to provide different default implementations for different
/// extensions is by implementing the trait for different types (unit structs).
/// The problem is, `FungibleToken` trait has to be implemented for the smart
/// contract (which is another struct). Therefore, a level of abstraction is
/// needed by introducing an associated type, which grants
/// `FungibleToken` trait the ability to switch between different default
/// implementations by calling the methods on this associated type.
///
/// This abstraction allows every method of the `FungibleToken` trait to be
/// implemented using
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
///     fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
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
///     fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
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

/// Burnable functionality
///
/// Trait for overriding `burn` and `burn_from` functions.
/// The behavior of `burn` and `burn_from` changes across implementations,
/// i.e. enumerable, consecutive, hence the need for an abstraction
pub trait BurnableOverrides {
    fn burn(e: &Env, from: &Address, amount: i128) {
        Base::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Base::burn_from(e, spender, from, amount);
    }
}

impl BurnableOverrides for Base {}
