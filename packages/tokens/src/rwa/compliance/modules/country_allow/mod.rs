//! Country allowlist compliance module — Stellar port of T-REX
//! [`CountryAllowModule.sol`][trex-src].
//!
//! Only recipients whose identity has at least one country code in the
//! allowlist may receive tokens.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/CountryAllowModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};

/// Emitted when a country is added to the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryAllowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}

/// Emitted when a country is removed from the allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryUnallowed {
    #[topic]
    pub token: Address,
    pub country: u32,
}
