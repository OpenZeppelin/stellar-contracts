//! # Lightweight upgradeability framework
//!
//! This module defines a minimal system for managing contract upgrades. It
//! provides the [`Upgradeable`] trait, which generates a standardized client
//! ([`UpgradeableClient`]) for calling upgrades from other contracts (e.g. a
//! helper upgrader, a governance contract, or a multisig).
//!
//! If a rollback is required, the contract can be upgraded to a newer version
//! where the rollback-specific logic is defined and performed as a migration.
//!
//! **IMPORTANT**: While the framework structures the upgrade flow, it does NOT
//! perform deeper checks and verifications such as:
//!
//! - Ensuring that the new contract does not include a constructor, as it will
//!   not be invoked.
//! - Verifying that the new contract includes an upgradability mechanism,
//!   preventing an unintended loss of further upgradability capacity.
//! - Checking for storage consistency, ensuring that the new contract does not
//!   inadvertently introduce storage mismatches.
//!
//!
//! # Simple Upgrade (no migration)
//!
//! Implement the [`Upgradeable`] trait directly and call [`upgrade()`] inside:
//!
//! ```rust,ignore
//! use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};
//! use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
//! use stellar_macros::only_role;
//!
//! #[contract]
//! pub struct ExampleContract;
//!
//! #[contractimpl]
//! impl Upgradeable for ExampleContract {
//!     #[only_role(operator, "admin")]
//!     fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
//!         upgradeable::upgrade(e, &new_wasm_hash);
//!     }
//! }
//! ```
//!
//! # Storage Migration
//!
//! When upgrading contracts, data structures may change (e.g., adding new
//! fields, removing old ones, or restructuring data). This section explains how
//! to handle those changes safely.
//!
//! ## The Problem: Host-Level Type Validation
//!
//! Soroban validates types at the host level when reading from storage. If a
//! data structure's shape changes between versions, the host traps before the
//! SDK can handle the mismatch:
//!
//! ```rust,ignore
//! #[contracttype]
//! pub struct ConfigV1 { pub rate: u32 }
//!
//! #[contracttype]
//! pub struct ConfigV2 { pub rate: u32, pub active: bool }
//!
//! // Developer expects `active` to default to some value, instead this traps
//! // with Error(Object, UnexpectedSize)
//! let config: ConfigV2 = e.storage().instance().get(&key).unwrap();
//! ```
//!
//! ## Pattern 1: Eager Migration (Bounded Data)
//!
//! For bounded data in instance storage (config, metadata, settings), add a
//! `migrate` function to your upgraded contract that reads old-format data and
//! converts it. Use a schema version to guard against double invocation.
//!
//! The old type must be defined in the new contract code so the host
//! can deserialize it correctly.
//!
//! ```rust,ignore
//! // Old type (matches what v1 stored — field names and types must match)
//! #[contracttype]
//! pub struct ConfigV1 {
//!     pub rate: u32,
//! }
//!
//! // New type
//! #[contracttype]
//! pub struct Config {
//!     pub rate: u32,
//!     pub active: bool,
//! }
//!
//! const CONFIG_KEY: Symbol = symbol_short!("CONFIG");
//! const SCHEMA_VERSION: Symbol = symbol_short!("VERSION");
//!
//! pub fn migrate(e: &Env, operator: Address) {
//!     // Guard: prevent double migration
//!     let version: u32 = e.storage().instance()
//!         .get(&SCHEMA_VERSION).unwrap_or(1);
//!     assert!(version < 2, "already migrated");
//!
//!     // Read old data using old type, convert, write back
//!     let old: ConfigV1 = e.storage().instance().get(&CONFIG_KEY).unwrap();
//!     let new = Config { rate: old.rate, active: true };
//!     e.storage().instance().set(&CONFIG_KEY, &new);
//!     e.storage().instance().set(&SCHEMA_VERSION, &2u32);
//! }
//! ```
//!
//! ## Pattern 2: Lazy Migration (Unbounded Data)
//!
//! For unbounded persistent storage (user balances, approvals, etc.),
//! eager migration is impractical as it's impossible to iterate all entries in
//! one transaction without hitting resource limits (200 entries / 132 KB writes
//! per transaction).
//!
//! Instead, use **version markers** alongside each entry and convert lazily on
//! read:
//!
//! ```rust,ignore
//! #[contracttype]
//! pub struct Balance { pub amount: i128 }
//!
//! #[contracttype]
//! pub struct BalanceV2 { pub amount: i128, pub frozen: bool }
//!
//! fn get_balance(e: &Env, account: &Address) -> BalanceV2 {
//!     let version: u32 = e.storage().persistent()
//!         .get(&StorageKey::BalanceVersion(account.clone()))
//!         .unwrap_or(1);
//!
//!     match version {
//!         1 => {
//!             let v1: Balance = e.storage().persistent()
//!                 .get(&StorageKey::Balance(account.clone())).unwrap();
//!             let v2 = BalanceV2 { amount: v1.amount, frozen: false };
//!             // Write back in new format (lazy migration)
//!             set_balance(e, account, &v2);
//!             v2
//!         }
//!         _ => e.storage().persistent()
//!             .get(&StorageKey::BalanceV2(account.clone())).unwrap(),
//!     }
//! }
//!
//! fn set_balance(e: &Env, account: &Address, balance: &BalanceV2) {
//!     e.storage().persistent()
//!         .set(&StorageKey::BalanceVersion(account.clone()), &2u32);
//!     e.storage().persistent()
//!         .set(&StorageKey::BalanceV2(account.clone()), balance);
//! }
//! ```
//!
//! ## Pattern 3: Enum Wrapper (Plan-Ahead)
//!
//! If you anticipate future migrations from the start, wrap stored data in a
//! versioned enum. Soroban serializes enum variants as `(tag, data)`, so the
//! host can distinguish between versions without trapping.
//!
//! ```rust,ignore
//! #[contracttype]
//! pub enum ConfigEntry {
//!     V1(ConfigV1),
//! }
//!
//! // Store wrapped from day one:
//! e.storage().instance().set(&key, &ConfigEntry::V1(config));
//! ```
//!
//! When v2 comes, add a variant and a converter:
//!
//! ```rust,ignore
//! #[contracttype]
//! pub enum ConfigEntry {
//!     V1(ConfigV1),
//!     V2(ConfigV2),
//! }
//!
//! impl ConfigEntry {
//!     pub fn into_latest(self) -> ConfigV2 {
//!         match self {
//!             ConfigEntry::V1(v1) => ConfigV2 { rate: v1.rate, active: true },
//!             ConfigEntry::V2(v2) => v2,
//!         }
//!     }
//! }
//! ```
//!
//! **Note**: This cannot work retroactively, since reading old bare-struct data
//! as an enum would trap.
//!
//! Check the `examples/upgradeable/` directory for the full example, where you
//! can also find a helper `Upgrader` contract that performs upgrade+migrate in
//! a single transaction.

mod storage;

use soroban_sdk::{contractclient, Address, BytesN, Env};

pub use crate::upgradeable::storage::upgrade;

/// A trait exposing an entry point for contract upgrades.
///
/// All access control and authorization checks are the implementor's
/// responsibility.
///
/// # Example
///
/// ```rust,ignore
/// #[contractimpl]
/// impl Upgradeable for MyContract {
///     fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
///         operator.require_auth();
///         // ... access control ...
///         upgradeable::upgrade(e, &new_wasm_hash);
///     }
/// }
/// ```
#[contractclient(name = "UpgradeableClient")]
pub trait Upgradeable {
    /// Upgrades the contract by setting a new WASM bytecode. The
    /// contract will only be upgraded after the invocation has
    /// successfully completed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `new_wasm_hash` - A 32-byte hash identifying the new WASM blob,
    ///   uploaded to the ledger.
    /// * `operator` - The authorized address performing the upgrade.
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address);
}
