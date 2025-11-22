//! # Soroban Smart Accounts
//!
//! A flexible and modular smart account framework for Soroban that enables
//! advanced authentication and authorization patterns through composable rules,
//! signers, and policies.
#![no_std]
#![cfg_attr(feature = "certora", allow(unused_variables,unused_imports))]

pub mod policies;
pub mod smart_account;
pub mod verifiers;