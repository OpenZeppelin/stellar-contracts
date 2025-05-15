//! # Stellar Asset Contract (SAC) Admin Module
//!
//! The Stellar Asset Contract (SAC) serves as a bridge between traditional
//! Stellar network assets and the Soroban smart contract environment.
//! When a classic Stellar asset is ported to Soroban, it is represented by
//! a SAC - a smart contract that provides both user-facing and administrative
//! functions for asset management.
//!
//! SACs expose standard functions for handling fungible tokens, such as
//! `transfer`, `approve`, `burn`, etc. Additionally, they include
//! administrative functions (`mint`, `clawback`, `set_admin`, `set_authorized`)
//! that are initially restricted to the issuer (a G-account).
//!
//! The `set_admin` function enables transferring administrative control to a
//! custom contract, allowing for more complex authorization logic. This
//! flexibility opens up possibilities for implementing custom rules, such as
//! role-based access control, two-step admin transfers, mint rate limits, and
//! upgradeability.
//!
//! When implementing a SAC Admin smart contract, there are two main approaches:
//!
//! - **Generic Approach:**
//!   - The new admin contract leverages the `__check_auth` function to handle
//!     authentication and authorization logic.
//!   - This approach allows for injecting any custom authorization logic while
//!     maintaining a unified interface for both user-facing and admin
//!     functions.
//!
//! - **Wrapper Approach:**
//!   - The new admin contract acts as a middleware, defining specific entry
//!     points for each admin function and forwarding calls to the corresponding
//!     SAC functions.
//!   - Custom logic is applied before forwarding the call, providing a
//!     straightforward and modular design, though at the cost of splitting
//!     user-facing and admin interfaces.
//!
//! _Trade-offs_
//! - The generic approach maintains a single interface but requires a more
//!   sophisticated authorization mechanism.
//! - The wrapper approach is simpler to implement and more flexible but
//!   requires additional entry points for each admin function.
//!
//! ## Module Overview
//!
//! This module implements **the wrapper version** for an Admin contract,
//! defining the interface and functions necessary to interact with a SAC.
//!
//! **NOTE**
//!
//! All functions, exposed in the `SACAdmin` trait, include an additional
//! parameter `operator: Address`. This account is the one authorizing the
//! invocation. Having it as a parameter is particularly useful when
//! implementing role-based access controls, in which case there can be mulitple
//! accounts per role.
//!
//! However, this parameter is omitted from the module functions, defined in
//! "storage.rs", because the authorizations are to be handled in the access
//! control helpers or directly implemented.

mod storage;
pub use storage::{
    clawback, get_sac_address, get_sac_client, mint, set_admin, set_authorized, set_sac_address,
};

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
