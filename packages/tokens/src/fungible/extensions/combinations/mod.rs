//! # Contract Type Composition for Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::fungible::FungibleToken`] implementation, and that single slot
//! decides all overridable behavior. The contract type is selected uniformly
//! with [`Compose`], by listing the contract types that override the `Base`
//! behavior: a single one, or several whose overrides have to apply at the
//! same time. [`Compose`] resolves the list to the curated contract type at
//! compile time, so no dedicated combination type has to be named by the
//! contract author.
//!
//! Note that [`Compose`] only selects the `ContractType`; it says nothing
//! about extensions that add new functionality without overriding the base
//! behavior. Those (e.g. [`crate::fungible::burnable::FungibleBurnable`] or
//! the [`crate::fungible::capped`] helpers) have no contract type and are
//! simply implemented alongside; there is no `Compose<(Burnable,)>`.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     type ContractType = Compose<(AllowList, TotalSupply)>;
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
//! The list is order-insensitive: `Compose<(TotalSupply, AllowList)>` and
//! `Compose<(AllowList, TotalSupply)>` resolve to the same contract type.
//! Invalid lists do not compile: `AllowList` and `BlockList` are mutually
//! exclusive, and implementing an extension trait the list does not back
//! (e.g. [`crate::fungible::total_supply::FungibleTotalSupply`] without
//! `TotalSupply` in the list) is rejected by that trait's bound.

mod storage;

#[cfg(test)]
mod test;

use storage::{TotalSupplyAllowList, TotalSupplyBlockList};

use crate::{
    fungible::{
        extensions::{
            allowlist::AllowList, blocklist::BlockList, total_supply::TotalSupply,
            votes::FungibleVotes,
        },
        Base, ContractOverrides,
    },
    rwa::RWA,
    vault::Vault,
};

/// Resolves a list of contract types to the combined contract type, e.g.
/// `Compose<(AllowList, TotalSupply)>`.
///
/// This is shorthand for `<L as Composable>::Out`; refer to [`Composable`] for
/// the valid lists.
pub type Compose<L> = <L as Composable>::Out;

/// Type-level lookup backing [`Compose`]: each valid contract type list
/// resolves to its combined contract type through the `Out` associated type.
///
/// Single contract types resolve to themselves (with or without the
/// one-element tuple form), and pairs are declared in both orders, making the
/// list order-insensitive. Mutually exclusive contract types (e.g.
/// `AllowList` and `BlockList`) have no implementation, so combining them
/// does not compile.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid contract type combination",
    note = "valid single contract types: `Base`, `AllowList`, `BlockList`, `TotalSupply`, `RWA`, \
            `Vault`, `FungibleVotes`",
    note = "valid combinations: `(AllowList, TotalSupply)`, `(BlockList, TotalSupply)`",
    note = "`AllowList` and `BlockList` are mutually exclusive"
)]
pub trait Composable {
    type Out: ContractOverrides;
}

// Single contract types resolve to themselves. The bare form and the
// one-element tuple form are equivalent, so a stray trailing comma does not
// change the meaning.
impl Composable for Base {
    type Out = Base;
}
impl Composable for (Base,) {
    type Out = Base;
}
impl Composable for AllowList {
    type Out = AllowList;
}
impl Composable for (AllowList,) {
    type Out = AllowList;
}
impl Composable for BlockList {
    type Out = BlockList;
}
impl Composable for (BlockList,) {
    type Out = BlockList;
}
impl Composable for TotalSupply {
    type Out = TotalSupply;
}
impl Composable for (TotalSupply,) {
    type Out = TotalSupply;
}
impl Composable for RWA {
    type Out = RWA;
}
impl Composable for (RWA,) {
    type Out = RWA;
}
impl Composable for Vault {
    type Out = Vault;
}
impl Composable for (Vault,) {
    type Out = Vault;
}
impl Composable for FungibleVotes {
    type Out = FungibleVotes;
}
impl Composable for (FungibleVotes,) {
    type Out = FungibleVotes;
}

// Pairs are declared in both orders, so the contract type list is
// order-insensitive.
impl Composable for (AllowList, TotalSupply) {
    type Out = TotalSupplyAllowList;
}
impl Composable for (TotalSupply, AllowList) {
    type Out = TotalSupplyAllowList;
}
impl Composable for (BlockList, TotalSupply) {
    type Out = TotalSupplyBlockList;
}
impl Composable for (TotalSupply, BlockList) {
    type Out = TotalSupplyBlockList;
}
