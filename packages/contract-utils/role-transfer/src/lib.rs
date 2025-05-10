//! This module only acts as a utility crate for `Access Control` and `Ownable`.
//! It is not intended to be used directly.

#![no_std]

use soroban_sdk::contracterror;

mod storage;

pub use storage::{accept_transfer, transfer_role};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
enum RoleTransferError {
    Unauthorized = 140,
    NoPendingTransfer = 141,
    InvalidLiveUntilLedger = 142,
    AccountNotFound = 143,
}

mod test;
