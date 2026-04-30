//! Initial lockup period compliance module — Stellar port of T-REX
//! [`TimeExchangeLimitsModule.sol`][trex-src].
//!
//! Enforces a lockup period for all investors whenever they receive tokens
//! through primary emissions (mints). Tokens received via peer-to-peer
//! transfers are **not** subject to lockup restrictions.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TimeExchangeLimitsModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};
pub use storage::LockedTokens;

/// Emitted when a token's lockup duration is configured or changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockupPeriodSet {
    #[topic]
    pub token: Address,
    pub lockup_seconds: u64,
}
