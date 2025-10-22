//! This module only acts as a utility crate for `Access Control` and `Ownable`.
//! It is not intended to be used directly.

use soroban_sdk::contracterror;

mod storage;

pub use storage::{accept_transfer, transfer_role};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RoleTransferError {
    NoPendingTransfer = 2200,
    InvalidLiveUntilLedger = 2201,
    InvalidPendingAccount = 2202,
}

#[cfg(test)]
mod test;
