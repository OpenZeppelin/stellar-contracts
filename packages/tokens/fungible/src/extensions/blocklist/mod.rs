mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{symbol_short, Address, Env};
pub use storage::*;

/// BlockList Trait for Fungible Token
///
/// The `FungibleBlockList` trait extends the `FungibleToken` trait to
/// provide a blocklist mechanism that can be managed by an authorized account.
/// This extension ensures that transfer can only take place if both the sender
/// and the receiver are not blocked. Note that, this restriction does not
/// apply to the spender.
///
/// This trait is designed to be used in conjunction with the `FungibleToken`
/// trait.
///
/// **NOTE**
///
/// All setter functions, exposed in the `FungibleBlockList` trait, include an
/// additional parameter `operator: Address`. This account is the one
/// authorizing the invocation. Having it as a parameter grants the flexibility
/// to introduce simple or complex role-based access controls.
///
/// However, this parameter is omitted from the module functions, defined in
/// "storage.rs", because the authorizations are to be handled in the access
/// control helpers or directly implemented.
#[soroban_sdk::contracttrait(default = BlockList, is_extension = true, extension_required = true)]
pub trait FungibleBlockList {
    /// Returns the blocked status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the blocked status for.
    fn blocked(e: &Env, account: &soroban_sdk::Address) -> bool;

    /// Blocks a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to block.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["block", user: Address]`
    /// * data - `[]`
    fn block_user(e: &Env, user: &soroban_sdk::Address, operator: &soroban_sdk::Address);

    /// Unblocks a user, allowing them to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to unblock.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["unblock", user: Address]`
    /// * data - `[]`
    fn unblock_user(e: &Env, user: &soroban_sdk::Address, operator: &soroban_sdk::Address);
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
