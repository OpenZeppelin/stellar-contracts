pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracttrait, Address, Env};
pub use storage::{Enumerable, EnumerableContractType};

use crate::non_fungible::NonFungibleToken;

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
/// 1. The `Enumerable` trait has its own business logic for creating and
///    destroying tokens. Therefore, this trait is INCOMPATIBLE with the
///    `Consecutive` extension.
/// 2. Enumerability can also be offloaded to off-chain services. This extension
///    exists for the use-cases where the enumeration is required as an on-chain
///    operation.
/// 3. Concurrent transactions on the same list: for typical collections, such
///    as transfers between individual accounts and occasional mints or burns,
///    this is an unlikely edge case and needs no special handling. It only
///    matters for lists that many transactions change at the same time.
///
///    Some storage keys accessed by these operations are chosen by the current
///    state of the lists, not only by the call's arguments:
///    * a transfer uses the owner lists of `from` and `to`, indexed by their
///      balances.
///    * a mint or burn also uses the global list, indexed by the total supply.
///    * removing a token that is not the last one in a list moves the last
///      token into the freed slot, so it depends on which token is last.
///
///    Soroban transactions declare their storage keys (the footprint) at
///    simulation time. If another transaction changes the same list in the
///    short window between simulation and execution (usually a few seconds),
///    the prepared transaction may access a key outside its footprint and
///    fail. The sender pays the fee, and can re-simulate and submit again.
///
///    Lists that see heavy concurrent use are the ones affected:
///    * the owner list of a contract that holds tokens on behalf of many users
///      (staking, escrow, marketplaces, lending), since all deposits and
///      withdrawals go through it. Any holder of a token from the collection
///      can also cause failures on purpose by repeatedly depositing and
///      withdrawing its own token.
///    * the global list during bursts of mints or burns (e.g. a public mint),
///      since every mint and burn changes the total supply.
///
///    Collections expected to be used this way should prefer off-chain
///    enumeration over this extension.
#[contracttrait]
pub trait NonFungibleEnumerable: NonFungibleToken<ContractType: EnumerableContractType> {
    /// Returns the total amount of tokens stored by the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn total_supply(e: &Env) -> u32 {
        Enumerable::total_supply(e)
    }

    /// Returns the `token_id` owned by `owner` at a given `index` in the
    /// owner's local list. Use along with
    /// [`crate::non_fungible::NonFungibleToken::balance`] to enumerate all of
    /// `owner`'s tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `index` - Index of the token in the owner's local list.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::TokenNotFoundInOwnerList`] - When the token
    ///   ID is not found in the owner's enumeration.
    fn get_owner_token_id(e: &Env, owner: Address, index: u32) -> u32 {
        Enumerable::get_owner_token_id(e, &owner, index)
    }

    /// Returns the `token_id` at a given `index` in the global token list.
    /// Use along with [`NonFungibleEnumerable::total_supply`] to enumerate
    /// all the tokens in the contract.
    ///
    /// A function to get all tokens of a contract is not provided because that
    /// would be unbounded. To enumerate all tokens of a contract, use
    /// [`NonFungibleEnumerable::total_supply`] to get the total number of
    /// tokens and then use [`NonFungibleEnumerable::get_token_id`] to retrieve
    /// each token one by one.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `index` - Index of the token in the global list.
    ///
    /// # Errors
    ///
    /// * [`crate::non_fungible::NonFungibleTokenError::TokenNotFoundInGlobalList`] - When the token
    ///   ID is not found in the global enumeration.
    fn get_token_id(e: &Env, index: u32) -> u32 {
        Enumerable::get_token_id(e, index)
    }
}
