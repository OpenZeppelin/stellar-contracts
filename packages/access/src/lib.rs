#![no_std]

#[cfg(feature = "roles")]
pub mod access_control;
#[cfg(feature = "ownable")]
pub mod ownable;

pub(crate) mod role_transfer;

#[cfg(any(feature = "ownable", feature = "roles"))]
pub use role_transfer::RoleTransferError;
