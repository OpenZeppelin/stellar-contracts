//! # Extension Combinations for Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::fungible::FungibleToken`] implementation, and that single slot
//! decides all overridable behavior. When the behaviors of several extensions
//! have to apply at the same time, the slot needs a contract type combining
//! them. Naming a dedicated type for every combination does not scale for the
//! contract author, so this module provides a type-level builder instead:
//! [`Build`] resolves a list of extensions to the curated combined contract
//! type at compile time.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     type ContractType = Build<(AllowList, TotalSupply)>;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleTotalSupply for MyToken {}
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleAllowList for MyToken {
//!     // ...
//! }
//! ```
//!
//! The list is order-insensitive: `Build<(TotalSupply, AllowList)>` and
//! `Build<(AllowList, TotalSupply)>` resolve to the same contract type.
//! Invalid combinations do not compile: `AllowList` and `BlockList` are
//! mutually exclusive, and implementing an extension trait the list does not
//! back (e.g. [`crate::fungible::total_supply::FungibleTotalSupply`] without
//! `TotalSupply` in the list) is rejected by that trait's bound.

mod storage;

#[cfg(test)]
mod test;

use storage::{TotalSupplyAllowList, TotalSupplyBlockList};

use crate::fungible::{
    extensions::{allowlist::AllowList, blocklist::BlockList, total_supply::TotalSupply},
    Base, ContractOverrides,
};

/// Resolves a list of extensions to the combined contract type, e.g.
/// `Build<(AllowList, TotalSupply)>`.
///
/// This is shorthand for `<L as Compose>::Out`; refer to [`Compose`] for the
/// valid combinations.
pub type Build<L> = <L as Compose>::Out;

/// Type-level lookup backing [`Build`]: each valid extension list resolves to
/// its combined contract type through the `Out` associated type.
///
/// Single extensions resolve to themselves (with or without the one-element
/// tuple form), and pairs are declared in both orders, making the list
/// order-insensitive. Mutually exclusive extensions (e.g. `AllowList` and
/// `BlockList`) have no implementation, so combining them does not compile.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid extension combination",
    note = "valid combinations: `Base`, `AllowList`, `BlockList`, `TotalSupply`, `(AllowList, \
            TotalSupply)`, `(BlockList, TotalSupply)`",
    note = "`AllowList` and `BlockList` are mutually exclusive"
)]
pub trait Compose {
    type Out: ContractOverrides;
}

// Single extensions resolve to themselves. The bare form and the one-element
// tuple form are equivalent, so a stray trailing comma does not change the
// meaning.
impl Compose for Base {
    type Out = Base;
}
impl Compose for (Base,) {
    type Out = Base;
}
impl Compose for AllowList {
    type Out = AllowList;
}
impl Compose for (AllowList,) {
    type Out = AllowList;
}
impl Compose for BlockList {
    type Out = BlockList;
}
impl Compose for (BlockList,) {
    type Out = BlockList;
}
impl Compose for TotalSupply {
    type Out = TotalSupply;
}
impl Compose for (TotalSupply,) {
    type Out = TotalSupply;
}

// Pairs are declared in both orders, so the extension list is
// order-insensitive.
impl Compose for (AllowList, TotalSupply) {
    type Out = TotalSupplyAllowList;
}
impl Compose for (TotalSupply, AllowList) {
    type Out = TotalSupplyAllowList;
}
impl Compose for (BlockList, TotalSupply) {
    type Out = TotalSupplyBlockList;
}
impl Compose for (TotalSupply, BlockList) {
    type Out = TotalSupplyBlockList;
}
