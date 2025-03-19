mod storage;
// pub use self::storage::{burn, burn_from};

mod test;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Enumerable Trait for Non-Fungible Token
///
/// The `NonFungibleEnumerable` trait extends the `NonFungibleToken` trait to
/// provide the following:
/// * Iterate through the tokens of an account
/// * Get the total number of tokens stored by the contract
///
/// We expect the common use case to use sequential `token_id`s for the tokens.
///
/// This trait is designed to be used in
/// conjunction with the `NonFungibleToken` trait.
///
/// # Notes
/// Enumerable trait has its own business logic for creating and destroying tokens. Therefore,
/// this trait is INCOMPATIBLE with the `Mintable`, `Burnable`, and `Consecutive` extensions.
#[contractclient(name = "NonFungibleEnumerableClient")]
pub trait NonFungibleEnumerable {
    /// Returns the total amount of tokens stored by the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// We recommend using [`crate::enumerable::total_supply()`] when implementing this function.
    fn total_supply(e: &Env) -> u32;

    /// Returns the `token_id` owned by `owner` at a given `index` of its token list.
    /// Use along with [`crate::NonFungibleToken::balance()`] to enumerate all of `owner`'s tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `index` - Index of the token in the owner's list.
    ///
    /// We recommend using [`crate::enumerable::get_owner_token()`] when implementing this function.
    fn get_owner_token(e: &Env, owner: &Address, index: u32) -> u32;
}
