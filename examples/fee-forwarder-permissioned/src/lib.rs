//! Permissioned fee forwarder example.
//!
//! This example shows how to integrate the `stellar-fee-abstraction` helpers in a
//! **permissioned** setup:
//!
//! - Only **trusted executors** configured by the contract are allowed to call the
//!   `forward` entrypoint.
//! - The **forwarder contract itself** collects the fees, which can later be
//!   swept or otherwise managed according to the contract logic.
//!
//! This pattern is suitable when you want a curated set of executors with tighter
//! operational control and auditability.

#![no_std]

mod contract;

#[cfg(test)]
mod test;
