mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{symbol_short, Address, Env};
pub use storage::AllowList;


/// AllowList Trait for Fungible Token
///
/// The `FungibleAllowList` trait extends the `FungibleToken` trait to
/// provide an allowlist mechanism that can be managed by an authorized account.
/// This extension ensures that transfer can only take place if the sender and
/// the receiver are both allowed. Note that, spender does not have to be
/// allowed.
///
/// This trait is designed to be used in conjunction with the `FungibleToken`
/// trait.
///
/// **NOTE**
///
/// All setter functions, exposed in the `FungibleAllowList` trait, include an
/// additional parameter `operator: Address`. This account is the one
/// authorizing the invocation. Having it as a parameter grants the flexibility
/// to introduce simple or complex role-based access controls.
///
/// However, this parameter is omitted from the module functions, defined in
/// "storage.rs", because the authorizations are to be handled in the access
/// control helpers or directly implemented.
#[soroban_sdk::contracttrait(default = AllowList, is_extension = true, extension_required = true)]
pub trait FungibleAllowList {

    /// Returns the allowed status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the allowed status for.
    fn allowed(e: &Env, account: &soroban_sdk::Address) -> bool;

    /// Allows a user to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to allow.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["allow", user: Address]`
    /// * data - `[]`
    fn allow_user(e: &Env, user: &soroban_sdk::Address, operator: &soroban_sdk::Address);

    /// Disallows a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to disallow.
    /// * `operator` - The address authorizing the invocation.
    ///
    /// # Events
    ///
    /// * topics - `["disallow", user: Address]`
    /// * data - `[]`
    fn disallow_user(e: &Env, user: &soroban_sdk::Address, operator: &soroban_sdk::Address);
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
/// * topics - `["disallow", user: Address]`
/// * data - `[]`
pub fn emit_user_disallowed(e: &Env, user: &Address) {
    let topics = (symbol_short!("disallow"), user);
    e.events().publish(topics, ());
}
