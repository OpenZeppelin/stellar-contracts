//! Permissioned fee forwarder example.
//!
//! This example shows how to integrate the `stellar-fee-abstraction` helpers in
//! a **permissioned** setup:
//!
//! - Only **trusted executors** configured by the contract are allowed to call
//!   the `forward` entrypoint.
//! - The **forwarder contract itself** collects the fees, which can later be
//!   swept or otherwise managed according to the contract logic.
//!
//! This pattern is suitable for environments with a curated set of executors
//! with tighter operational control.

#![no_std]
#![allow(clippy::too_many_arguments)]

mod contract;

#[cfg(test)]
mod test;
