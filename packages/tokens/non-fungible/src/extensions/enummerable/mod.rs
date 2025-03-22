#![allow(unused_variables)]
pub mod storage;

use soroban_sdk::{contractclient, Address, Env};
use storage::{
    add_to_global_enumeration, add_to_owner_enumeration, decrement_total_supply,
    increment_total_supply, remove_from_global_enumeration, remove_from_owner_enumeration,
};

use crate::NonFungibleToken;

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
/// Enumerating all the tokens differs based on the minting strategy.
/// * Sequential `token_id`s: Token with `token_id` `0` is the first token,
///   `token_id` `1` is the second token, and so on, till the last token with
///   `token_id` [`NonFungibleEnumerable::total_supply()`] `- 1`.
/// * Non-sequential `token_id`s: The same strategy for `OwnedTokens` applies.
///   `Enumerable` extension stores a list of the all tokens, with indices. The
///   first token of the contract can be found with `index` `0`, and so on. To
///   retrieve `token_id`s, one can call the
///   [`NonFungibleEnumerable::get_token_id()`] function.
///
/// This trait is designed to be used in
/// conjunction with the `NonFungibleToken` trait.
///
/// # Notes
/// Enumerable trait has its own business logic for creating and destroying
/// tokens. Therefore, this trait is INCOMPATIBLE with the `Mintable`,
/// `Burnable`, and `Consecutive` extensions.
///
/// Note that, `Enumerable` trait can also be offloaded to off-chain services.
/// This extension exists for the use-cases where the enumeration is required as
/// an on-chain operation.
#[contractclient(name = "NonFungibleEnumerableClient")]
pub trait NonFungibleEnumerable: NonFungibleToken + Sized {
    type EnumerationStrategy: Enumeration<Self>;

    /// Returns the total amount of tokens stored by the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// We recommend using [`crate::enumerable::total_supply()`] when
    /// implementing this function.
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
    ///
    /// We recommend using [`crate::enumerable::get_owner_token_id()`] when
    /// implementing this function.
    fn get_owner_token_id(e: &Env, owner: &Address, index: u32) -> u32;

    /// Returns the `token_id` at a given `index` in the global token list.
    /// Use along with [`NonFungibleEnumerable::total_supply()`] to enumerate
    /// all the tokens in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `index` - Index of the token in the owner's local list.
    ///
    /// We recommend using [`crate::enumerable::get_token_id()`] when
    /// implementing this function.
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function is only intended for non-sequential
    /// `token_id`s. For sequential `token_id`s, no need to call a function,
    /// the `token_id` itself acts as the global index.
    fn get_token_id(e: &Env, index: u32) -> u32;
}

pub struct Sequential;
pub struct NonSequential;

pub trait Enumeration<T: NonFungibleEnumerable> {
    fn add(e: &Env, to: &Address, token_id: u32);

    fn remove(e: &Env, from: &Address, token_id: u32);
}

impl<T: NonFungibleEnumerable> Enumeration<T> for Sequential {
    fn add(e: &Env, to: &Address, token_id: u32) {
        add_to_owner_enumeration::<T>(e, to, token_id);
        let _ = increment_total_supply::<T>(e);
    }

    fn remove(e: &Env, from: &Address, token_id: u32) {
        remove_from_owner_enumeration::<T>(e, from, token_id);
        let _ = decrement_total_supply::<T>(e);
    }
}

impl<T: NonFungibleEnumerable> Enumeration<T> for NonSequential {
    fn add(e: &Env, to: &Address, token_id: u32) {
        add_to_owner_enumeration::<T>(e, to, token_id);
        let total_supply = increment_total_supply::<T>(e);
        add_to_global_enumeration(e, token_id, total_supply);
    }

    fn remove(e: &Env, from: &Address, token_id: u32) {
        remove_from_owner_enumeration::<T>(e, from, token_id);
        let total_supply = decrement_total_supply::<T>(e);
        remove_from_global_enumeration::<T>(e, token_id, total_supply);
    }
}
