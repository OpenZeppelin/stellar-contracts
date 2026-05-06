//! Time-windowed transfer-limits compliance module — Stellar port of T-REX
//! [`TimeTransfersLimitsModule.sol`][trex-src].
//!
//! Limits transfer volume within configurable time windows, tracking counters
//! per **identity** (not per wallet).
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeTransfersLimitsModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};
pub use storage::{Limit, TransferCounter};

pub const MAX_LIMITS_PER_TOKEN: u32 = 4;

/// Emitted when a time-window limit is added or updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitUpdated {
    #[topic]
    pub token: Address,
    pub limit: Limit,
}

/// Emitted when a time-window limit is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeTransferLimitRemoved {
    #[topic]
    pub token: Address,
    pub limit_time: u64,
}
