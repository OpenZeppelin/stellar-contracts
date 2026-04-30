//! Transfer restriction (address allowlist) compliance module — Stellar port
//! of T-REX [`TransferRestrictModule.sol`][trex-src].
//!
//! Maintains a per-token address allowlist. Transfers pass if the sender is
//! on the list; otherwise the recipient must be.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TransferRestrictModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};

/// Emitted when an address is added to the transfer allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAllowed {
    #[topic]
    pub token: Address,
    pub user: Address,
}

/// Emitted when an address is removed from the transfer allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserDisallowed {
    #[topic]
    pub token: Address,
    pub user: Address,
}
