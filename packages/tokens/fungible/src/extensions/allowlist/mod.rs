pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{symbol_short, Address, Env};
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
/// won't be able to execute transfers or approvals if it's not explicitly
/// allowed.
///
/// This trait is designed to be used in conjunction with the `FungibleToken`
/// trait.
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
    /// * `user` - The address to allow.
    ///
    /// # Events
    ///
    /// * topics - `["user_allowed", user: Address]`
    /// * data - `[]`
    fn allow_user(e: &Env, user: Address);

    /// Disallows a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to disallow.
    ///
    /// # Events
    ///
    /// * topics - `["user_disallowed", user: Address]`
    /// * data - `[]`
    fn disallow_user(e: &Env, user: Address);
}

// ################## EVENTS ##################

/// Emits an event when a user is allowed to transfer tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `user` - The address that is allowed to transfer tokens.
///
/// # Events
///
/// * topics - `["allow", user: Address]`
/// * data - `[]`
pub fn emit_user_allowed(e: &Env, user: &Address) {
    let topics = (symbol_short!("allow"), user);
    e.events().publish(topics, ());
}

/// Emits an event when a user is disallowed from transferring tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `user` - The address that is disallowed from transferring tokens.
///
/// # Events
///
/// * topics - `["deny", user: Address]`
/// * data - `[]`
pub fn emit_user_disallowed(e: &Env, user: &Address) {
    let topics = (symbol_short!("deny"), user);
    e.events().publish(topics, ());
}
