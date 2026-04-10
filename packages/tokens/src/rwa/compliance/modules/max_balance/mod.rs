//! Max balance compliance module — Stellar port of T-REX
//! [`MaxBalanceModule.sol`][trex-src].
//!
//! Tracks effective balances per **identity** (not per wallet), enforcing a
//! per-token cap.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};

/// Emitted when a token's per-identity balance cap is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBalanceSet {
    #[topic]
    pub token: Address,
    pub max_balance: i128,
}

/// Emitted when an identity balance is pre-seeded via `pre_set_module_state`.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IDBalancePreSet {
    #[topic]
    pub token: Address,
    pub identity: Address,
    pub balance: i128,
}
