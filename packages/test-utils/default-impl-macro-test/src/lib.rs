//! This crate is a test crate for the default-impl-macro.
//! Proc-macros cannot be tested within their own crate due to Rust's
//! limitations, hence a separate crate for testing is used for testing the
//! proc-macro.
//!
//! This crate is not intended for use in any other context. And this `lib.rs`
//! file is empty on purpose.

// A conditional attribute that applies `no_std` only for wasm targets.
// This prevents Cargo from implicitly injecting std::prelude imports into empty crates
// when building for wasm targets that don't support std (like wasm32v1-none).
#![cfg_attr(target_family = "wasm", no_std)]
