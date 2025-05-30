pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{symbol_short, Address, Env};
pub use storage::BlockList;

use crate::FungibleToken;

/// BlockList Trait for Fungible Token
///
/// The `FungibleBlockList` trait extends the `FungibleToken` trait to
/// provide a blocklist mechanism that can be managed by an authorized account.
/// This extension ensures that blocked accounts cannot transfer tokens or
/// approve token transfers.
///
/// The blocklist provides the guarantee to the contract owner that any blocked
/// account won't be able to execute transfers or approvals.
///
/// This trait is designed to be used in conjunction with the `FungibleToken`
/// trait.
pub trait FungibleBlockList: FungibleToken<ContractType = BlockList> {
    /// Returns the blocked status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the blocked status for.
    fn blocked(e: &Env, account: Address) -> bool;

    /// Blocks a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to block.
    ///
    /// # Events
    ///
    /// * topics - `["user_blocked", user: Address]`
    /// * data - `[]`
    fn block_user(e: &Env, user: Address);

    /// Unblocks a user, allowing them to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to unblock.
    ///
    /// # Events
    ///
    /// * topics - `["user_unblocked", user: Address]`
    /// * data - `[]`
    fn unblock_user(e: &Env, user: Address);
}

// ################## EVENTS ##################

/// Emits an event when a user is blocked from transferring tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `user` - The address that is blocked from transferring tokens.
///
/// # Events
///
/// * topics - `["block", user: Address]`
/// * data - `[]`
pub fn emit_user_blocked(e: &Env, user: &Address) {
    let topics = (symbol_short!("block"), user);
    e.events().publish(topics, ());
}

/// Emits an event when a user is unblocked and allowed to transfer tokens
/// again.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `user` - The address that is unblocked.
///
/// # Events
///
/// * topics - `["unblock", user: Address]`
/// * data - `[]`
pub fn emit_user_unblocked(e: &Env, user: &Address) {
    let topics = (symbol_short!("unblock"), user);
    e.events().publish(topics, ());
}
