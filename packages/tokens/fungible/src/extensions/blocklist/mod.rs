pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{Address, Env};
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
/// This trait is designed to be used in conjunction with the `FungibleToken` trait.
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
    /// * `admin` - The address of the admin performing the operation.
    /// * `user` - The address to block.
    ///
    /// # Events
    ///
    /// * topics - `["user_blocked", user: Address]`
    /// * data - `[]`
    fn block_user(e: &Env, admin: Address, user: Address);

    /// Unblocks a user, allowing them to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address of the admin performing the operation.
    /// * `user` - The address to unblock.
    ///
    /// # Events
    ///
    /// * topics - `["user_unblocked", user: Address]`
    /// * data - `[]`
    fn unblock_user(e: &Env, admin: Address, user: Address);
}
