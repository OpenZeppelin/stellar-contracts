//! Permissionless fee forwarder example.
//!
//! This example shows how to integrate the `stellar-fee-abstraction` helpers in a
//! **permissionless** setup:
//!
//! - **Anyone** can call the `forward` entrypoint; there is no executor allowlist.
//! - The **relayer** (the transaction submitter) receives the collected fee.
//!
//! In contrast, the permissioned example restricts `forward` to trusted executors
//! and has the contract itself collect fees. This pattern is suitable for open
//! environments where any party can become a relayer and be economically
//! incentivized to forward user transactions.

#![no_std]

mod contract;

#[cfg(test)]
mod test;
