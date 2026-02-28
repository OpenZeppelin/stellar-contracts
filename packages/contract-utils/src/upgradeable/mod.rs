//! # Lightweight upgradeability framework
//!
//! This module defines a minimal system for managing contract upgrades. It
//! provides the [`Upgradeable`] trait, which generates a standardized client
//! ([`UpgradeableClient`]) for calling upgrades from other contracts (e.g. a
//! helper upgrader, a governance contract, or a multisig).
//!
//! **IMPORTANT**: While the module provides an upgrade entrypoint, it does NOT
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
//! An upgrade replaces the contract's executable code while preserving all
//! existing storage.
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
//! ## Why there is no `Migratable` trait
//!
//! Migration is deliberately not standardized into a trait. The reasons are:
//!
//! - Migration rarely has a single entrypoint: a contract may need to migrate
//!   several independent storage structures at different times.
//! - A fixed trait signature would force all migration arguments into a single
//!   `#[contracttype]` struct, removing the flexibility to choose argument
//!   types, authorization roles, or split migration across multiple functions.
//! - Lazy migration (Pattern 2) has no discrete migration call at all.
//!
//! The patterns below are therefore guidelines rather than enforced interfaces.
//!
//! ## The Problem: Host-Level Type Validation
//!
//! Soroban validates types at the host level when reading from storage. If a
//! data structure's shape changes between versions, the host traps before the
//! SDK can handle the mismatch:
//!
//! ```rust,ignore
//! // V1 stored this type:
//! #[contracttype]
//! pub struct Config { pub rate: u32 }
//!
//! // V2 adds a field. Reading old storage with the new type traps, because
//! // the host validates field count before the SDK sees the value.
//! #[contracttype]
//! pub struct Config { pub rate: u32, pub active: bool }
//!
//! // Traps with Error(Object, UnexpectedSize)
//! let config: Config = e.storage().instance().get(&key).unwrap();
//! ```
//!
//! ## Pattern 1: Eager Migration (Bounded Data)
//!
//! For bounded data in instance storage (config, metadata, settings), add a
//! `migrate` function to the upgraded contract that reads old-format data and
//! converts it. Use [`set_schema_version`] / [`get_schema_version`] to guard
//! against double invocation.
//!
//! The old type must be defined in the new contract code so the host
//! can deserialize it correctly.
//!
//! ```rust,ignore
//! // Old type (matches what v1 stored, field names and types must match)
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
//!
//! pub fn migrate(e: &Env, operator: Address) {
//!     assert!(upgradeable::get_schema_version(e) < 2, "already migrated");
//!
//!     let old: ConfigV1 = e.storage().instance().get(&CONFIG_KEY).unwrap();
//!     let new = Config { rate: old.rate, active: true };
//!     e.storage().instance().set(&CONFIG_KEY, &new);
//!
//!     upgradeable::set_schema_version(e, 2);
//! }
//! ```
//!
//! Migration must happen in a separate transaction after the upgrade completes,
//! or atomically via a third-party upgrader contract that calls upgrade in one
//! cross-contract call and migrate in a second (see `examples/upgradeable/
//! upgrader`).
//!
//! ## Pattern 2: Lazy Migration (Unbounded Data)
//!
//! For unbounded persistent storage (user balances, approvals, etc.),
//! eager migration is impractical as it's impossible to iterate all entries in
//! one transaction without hitting resource limits (e.g. max number of keys in
//! the footprint).
//!
//! Instead, use **version markers** alongside each entry and convert lazily on
//! read:
//!
//! ```rust,ignore
//! // Old type must match what v1 stored exactly.
//! #[contracttype]
//! pub struct BalanceV1 { pub amount: i128 }
//!
//! // New type with an added field.
//! #[contracttype]
//! pub struct Balance { pub amount: i128, pub frozen: bool }
//!
//! #[contracttype]
//! pub enum StorageKey {
//!     Balance(Address),
//!     // New in v2: tracks the schema version of each individual entry.
//!     // Absent for v1 entries, which default to version 1.
//!     BalanceVersion(Address),
//! }
//!
//! fn get_balance(e: &Env, account: &Address) -> Balance {
//!     let version: u32 = e.storage().persistent()
//!         .get(&StorageKey::BalanceVersion(account.clone()))
//!         .unwrap_or(1);
//!
//!     match version {
//!         1 => {
//!             let v1: BalanceV1 = e.storage().persistent()
//!                 .get(&StorageKey::Balance(account.clone())).unwrap();
//!             let migrated = Balance { amount: v1.amount, frozen: false };
//!             // Write back in new format so subsequent reads are direct.
//!             set_balance(e, account, &migrated);
//!             migrated
//!         }
//!         _ => e.storage().persistent()
//!             .get(&StorageKey::Balance(account.clone())).unwrap(),
//!     }
//! }
//!
//! fn set_balance(e: &Env, account: &Address, balance: &Balance) {
//!     // Version marker and data share the same address key but different
//!     // variants, so they occupy separate storage entries.
//!     e.storage().persistent()
//!         .set(&StorageKey::BalanceVersion(account.clone()), &2u32);
//!     e.storage().persistent()
//!         .set(&StorageKey::Balance(account.clone()), balance);
//! }
//! ```
//!
//! ## Pattern 3: Enum Wrapper (Plan-Ahead)
//!
//! For contracts that anticipate future migrations from the start, wrap stored
//! data in a versioned enum. Soroban serializes enum variants as `(tag, data)`,
//! so the host can distinguish between versions without trapping.
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
//! If a rollback is required, the contract can be upgraded to a newer version
//! where the rollback-specific logic is defined and performed as a migration.
//!
//! See the `examples/upgradeable/` directory for full examples:
//! - `v1` / `v2` — eager migration of bounded instance storage, with an
//!   `Upgrader` helper that atomically combines upgrade+migrate.
//! - `lazy-v1` / `lazy-v2` — lazy per-entry migration of unbounded persistent
//!   storage, where entries are converted on first read after the upgrade.

mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractclient, Address, BytesN, Env};

pub use crate::upgradeable::storage::{get_schema_version, set_schema_version, upgrade};

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
