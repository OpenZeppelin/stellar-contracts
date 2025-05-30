//! # Stellar Asset Contract (SAC) Admin Generic Module
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
//! This module provides helper functions for implementing **the generic
//! version** for a SAC Admin contract.
//!
//! - An example contract that follows this approach can be found in
//!   "examples/sac-admin-generic".
//! - An example flow when a `SACAdminGeneric` contract is set as a new
//!   administrator for a SAC can be found [here](./README.md);

mod storage;
pub use storage::{
    extract_sac_contract_context, get_fn_param, get_sac_address, set_sac_address, SacFn,
};

mod test;
