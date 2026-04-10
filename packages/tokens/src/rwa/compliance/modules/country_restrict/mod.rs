//! Country restriction compliance module — Stellar port of T-REX
//! [`CountryRestrictModule.sol`][trex-src].
//!
//! Recipients whose identity has a country code on the restriction list are
//! blocked from receiving tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryRestrictModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};

/// Emitted when a country is added to the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryRestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emitted when a country is removed from the restriction list.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnrestricted {
    #[topic]
    pub token: Address,
    pub country: u32,
}
