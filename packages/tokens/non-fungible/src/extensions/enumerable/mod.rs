pub mod storage;

mod test;

use soroban_sdk::{contracttrait, Env};
pub use storage::Enumerable;

/// Enumerable Trait for Non-Fungible Token
///
/// The `NonFungibleEnumerable` trait extends the `NonFungibleToken` trait to
/// provide the following:
/// * Enumerating the tokens of an account.
/// * Enumerating all the tokens in the smart contract.
///
/// Enumerating all the tokens of an account is achieved via the help of the
/// [`crate::non_fungible::NonFungibleToken::balance()`] function. `Enumerable`
/// extension stores a list of the tokens of an owner, with indices. Every
/// owner's list starts with the local index `0`, and the last token of the
/// owner can be found with `balance() - 1`. To retrieve the `token_id`s, one
/// can call the [`NonFungibleEnumerable::get_owner_token_id()`] function.
///
/// Enumerating all the tokens in the smart contract is achieved via the help
/// of the [`NonFungibleEnumerable::total_supply()`] function. `Enumerable`
/// extension stores a list of all the tokens, with indices. The first token of
/// the contract can be found with `index` `0`, the second with `1`, and so on.
/// To retrieve `token_id`s, one can call the
/// [`NonFungibleEnumerable::get_token_id()`] function.
///
/// This trait is designed to be used in conjunction with the `NonFungibleToken`
/// trait.
///
/// # Notes
///
/// Enumerable trait has its own business logic for creating and destroying
/// tokens. Therefore, this trait is INCOMPATIBLE with the
/// `Consecutive` extension.
///
/// Note that, `Enumerable` trait can also be offloaded to off-chain services.
/// This extension exists for the use-cases where the enumeration is required as
/// an on-chain operation.
///
/// # Notes
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To not confuse the developers, we did not provide
/// the default implementations here, but we are providing a macro to generate
/// the default implementations for you.
///
/// When implementing [`NonFungibleEnumerable`] trait for your Smart Contract,
/// you can follow the below example:
///
/// ```ignore
/// #[default_impl] // **IMPORTANT**: place this above `#[contractimpl]`
/// #[contractimpl]
/// impl NonFungibleEnumerable for MyContract {
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     /* and the macro will generate all the missing default implementations for you */
/// }
/// ```
#[contracttrait(default = Enumerable)]
pub trait NonFungibleEnumerable {
    /// Returns the total amount of tokens stored by the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn total_supply(e: &Env) -> u32;

    /// Returns the `token_id` owned by `owner` at a given `index` in the
    /// owner's local list. Use along with
    /// [`crate::NonFungibleToken::balance()`] to enumerate all of `owner`'s
    /// tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `index` - Index of the token in the owner's local list.
    fn get_owner_token_id(e: &Env, owner: &soroban_sdk::Address, index: u32) -> u32;

    /// Returns the `token_id` at a given `index` in the global token list.
    /// Use along with [`NonFungibleEnumerable::total_supply()`] to enumerate
    /// all the tokens in the contract.
    ///
    /// We do not provide a function to get all the tokens of a contract,
    /// since that would be unbounded. If you need to enumerate all the
    /// tokens of a contract, you can use
    /// [`NonFungibleEnumerable::total_supply()`] to get the total number of
    /// tokens and then use [`NonFungibleEnumerable::get_token_id()`] to get
    /// each token one by one.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `index` - Index of the token in the global list.
    fn get_token_id(e: &Env, index: u32) -> u32;
}
