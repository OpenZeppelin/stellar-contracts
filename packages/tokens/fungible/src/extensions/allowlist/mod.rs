pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{Address, Env};
pub use storage::AllowList;

use crate::FungibleToken;

/// AllowList Trait for Fungible Token
///
/// The `FungibleAllowList` trait extends the `FungibleToken` trait to
/// provide an allowlist mechanism that can be managed by an authorized account.
/// This extension ensures that only allowed accounts can transfer tokens or 
/// approve token transfers.
///
/// The allowlist provides the guarantee to the contract owner that any account 
/// won't be able to execute transfers or approvals if it's not explicitly allowed.
///
/// This trait is designed to be used in conjunction with the `FungibleToken` trait.
pub trait FungibleAllowList: FungibleToken<ContractType = AllowList> {
    /// Returns the allowed status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the allowed status for.
    fn allowed(e: &Env, account: Address) -> bool;

    /// Allows a user to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address of the admin performing the operation.
    /// * `user` - The address to allow.
    ///
    /// # Events
    ///
    /// * topics - `["user_allowed", user: Address]`
    /// * data - `[]`
    fn allow_user(e: &Env, admin: Address, user: Address);

    /// Disallows a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address of the admin performing the operation.
    /// * `user` - The address to disallow.
    ///
    /// # Events
    ///
    /// * topics - `["user_disallowed", user: Address]`
    /// * data - `[]`
    fn disallow_user(e: &Env, admin: Address, user: Address);
}
