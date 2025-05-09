pub mod storage;

mod test;

use soroban_sdk::{Address, Env};

pub trait SACAdmin {
    /// Sets the administrator to the specified address `new_admin`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_admin` - The address which will henceforth be the administrator
    ///   of the token contract.
    /// * `operator` - The address authorizing the invocation.
    fn set_admin(e: Env, new_admin: Address, operator: Address);

    /// Sets whether the account is authorized to use its balance. If
    /// `authorized` is true, `id` should be able to use its balance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `id` - The address being (de-)authorized.
    /// * `authorize` - Whether or not `id` can use its balance.
    /// * `operator` - The address authorizing the invocation.
    fn set_authorized(e: Env, id: Address, authorize: bool, operator: Address);

    /// Mints `amount` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `to` - The address which will receive the minted tokens.
    /// * `amount` - The amount of tokens to be minted.
    /// * `operator` - The address authorizing the invocation.
    fn mint(e: Env, to: Address, amount: i128, operator: Address);

    /// Clawback `amount` from `from` account. `amount` is burned in the
    /// clawback process.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `from` - The address holding the balance from which the clawback will
    ///   take tokens.
    /// * `amount` - The amount of tokens to be clawed back.
    /// * `operator` - The address authorizing the invocation.
    fn clawback(e: Env, from: Address, amount: i128, operator: Address);
}
