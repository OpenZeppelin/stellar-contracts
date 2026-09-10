//! # Soroban Smart Accounts
//!
//! A flexible and modular smart account framework for Soroban that enables
//! advanced authentication and authorization patterns through composable rules,
//! signers, and policies.
//!
//! The crate [README](https://github.com/OpenZeppelin/stellar-contracts/tree/main/packages/accounts)
//! and the [documentation](https://docs.openzeppelin.com/stellar-contracts/accounts/smart-account)
//! cover the client-side authorization flow, including the authorization
//! entry that transaction simulation does not return for delegated signers.
#![no_std]

pub mod policies;
pub mod smart_account;
pub mod verifiers;
