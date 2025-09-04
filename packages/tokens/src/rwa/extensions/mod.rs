//! # RWA Extensions Module
//!
//! This module contains optional extensions for RWA (Real World Assets) tokens
//! that provide additional functionality beyond the core token implementation.
//!
//! ## Available Extensions
//!
//! - **Document Manager**: Provides document management capabilities following
//!   the ERC-1643 standard, allowing contracts to attach, update, and retrieve
//!   documents with associated metadata.
//!
//! ## Usage
//!
//! Extensions are designed to be optional and can be implemented selectively
//! based on the specific requirements of your RWA token contract.
//!
//! ```rust
//! use crate::{rwa::extensions::doc_manager::DocumentManager, token::Token};
//!
//! #[contractimpl]
//! impl DocumentManager for MyTokenContract {
//!     // Implementation of document management functions
//! }
//! ```

pub mod doc_manager;
