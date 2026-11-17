//! Test support shared by the workspace crates.
//!
//! `soroban-sdk` enforces the Stellar Mainnet resource limits on every test
//! invocation, but two things are left to the test author: reaching the state
//! that maximises resource use, and the one dimension the SDK does not count,
//! the top-level return value, which the network adds to the contract events
//! total before comparing against `contract_events_size_bytes`.
//!
//! This crate exposes the limits snapshot the SDK ships, so tests never pin
//! network numbers locally, and an assertion for the combined
//! events-and-return-value size.

pub use soroban_env_host::InvocationResourceLimits;
use soroban_sdk::{testutils::cost_estimate::NetworkInvocationResourceLimits, xdr::ToXdr, Env};

/// Share of a network limit that library-produced output may use.
///
/// The library never controls the whole transaction: the integrating contract
/// adds its own events and return value, and a single call may touch several
/// bounded modules. Max-fill tests for library modules assert against this
/// share rather than the full limit.
pub const LIBRARY_HEADROOM_PERCENT: u32 = 75;

/// Returns the Mainnet invocation resource limits snapshot shipped with the
/// SDK in use.
///
/// The snapshot is updated with the SDK, so a bump that carries new network
/// settings re-validates every test built on these values.
pub fn mainnet_limits() -> InvocationResourceLimits {
    InvocationResourceLimits::mainnet()
}

/// Returns the size in bytes of `value` serialized as a top-level return
/// value.
pub fn return_value_size(e: &Env, value: impl ToXdr) -> u32 {
    value.to_xdr(e).len()
}

/// Asserts that the contract events of the last invocation plus `ret`
/// serialized as its top-level return value fit within `percent` of the
/// Mainnet events-and-return-value limit.
///
/// Must be called right after the invocation that produced `ret`.
pub fn assert_events_and_return_within(e: &Env, ret: impl ToXdr, percent: u32) {
    let events = e.cost_estimate().resources().contract_events_size_bytes;
    let ret_bytes = return_value_size(e, ret);
    let limit = mainnet_limits().contract_events_size_bytes;
    let allowed = (u64::from(limit) * u64::from(percent) / 100) as u32;
    let total = events + ret_bytes;
    assert!(
        total <= allowed,
        "events ({events} B) + return value ({ret_bytes} B) = {total} B exceeds {allowed} B \
         ({percent}% of the {limit} B Mainnet events-and-return-value limit)"
    );
}

#[cfg(test)]
mod test;
