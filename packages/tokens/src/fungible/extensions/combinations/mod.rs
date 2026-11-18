//! # Contract Type Composition for Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::fungible::FungibleToken`] implementation, and that single slot
//! decides all overridable behavior. The contract type is selected uniformly
//! with [`Compose`], by listing the extensions the token is made of.
//!
//! The list determines the base type of the token; the extension traits the
//! contract implements decide which entry points it exposes. This gives the
//! rule for what goes in the list: an entry has to be listed when it changes
//! the base type, and an entry that does not change it may be listed or left
//! out (it is accepted and ignored). Two kinds of entries do not change the
//! base type:
//!
//! - Additive extensions (e.g. [`Burnable`] for
//!   [`crate::fungible::burnable::FungibleBurnable`]) are enabled by
//!   implementing their trait, whether or not they are listed. A list holding
//!   only additive extensions resolves to [`Base`].
//! - `TotalSupply` next to a contract type that already tracks the supply
//!   (`RWA`, `Vault`, [`Capped`] and the capped combinations) changes nothing:
//!   `Compose<(RWA, TotalSupply)>` resolves to `RWA`, like `Compose<RWA>`.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl FungibleToken for MyToken {
//!     // resolves to the contract type gating transfers by the allowlist
//!     // and tracking voting units; `Burnable` is declarative
//!     type ContractType = Compose<(AllowList, FungibleVotes, Burnable)>;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleAllowList for MyToken {
//!     // ...
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl Votes for MyToken {}
//!
//! #[contractimpl(contracttrait)]
//! impl FungibleBurnable for MyToken {}
//! ```
//!
//! Curated multi-type combinations: [`AllowList`], [`BlockList`] and
//! [`FungibleVotes`] can be combined with each other, in pairs or all three
//! together, e.g. `Compose<(AllowList, BlockList)>`,
//! `Compose<(AllowList, FungibleVotes)>` or
//! `Compose<(AllowList, BlockList, FungibleVotes)>`. [`TotalSupply`] can be
//! added to `AllowList`, `BlockList` or both, e.g.
//! `Compose<(AllowList, TotalSupply)>` or
//! `Compose<(AllowList, BlockList, TotalSupply)>`. `FungibleVotes` tracks
//! voting units, not the token supply, and has no curated combination with
//! `TotalSupply`.
//!
//! `Capped` is a contract type of its own: its mint checks the cap, and it
//! tracks the supply the cap is checked against. Combined with `AllowList`,
//! `BlockList` (or both), `RWA` or `Vault`, it resolves to the matching capped
//! combination, e.g. `Compose<(AllowList, Capped)>` to `CappedAllowList`.
//!
//! The resolved contract type enforces every listed transfer policy, tracks
//! voting units when `FungibleVotes` is listed, tracks the total supply when
//! `TotalSupply`, `RWA`, `Vault` or `Capped` is listed, and backs the
//! corresponding extension traits. The list is order-insensitive:
//! `Compose<(BlockList, AllowList)>` resolves to the same contract type as
//! `Compose<(AllowList, BlockList)>`. Invalid lists do not compile: contract
//! types without a curated combination (e.g. `AllowList` with `RWA`) cannot
//! be listed together, and implementing an extension trait the resolved
//! contract type does not back (e.g.
//! [`crate::fungible::total_supply::FungibleTotalSupply`] on a plain
//! `AllowList` token) is rejected by that trait's bound.

mod storage;

#[cfg(test)]
mod test;

use storage::{
    AllowBlockList, AllowBlockListVotes, AllowListVotes, BlockListVotes, CappedAllowBlockList,
    CappedAllowList, CappedBlockList, TotalSupplyAllowBlockList, TotalSupplyAllowList,
    TotalSupplyBlockList,
};

use crate::{
    fungible::{
        extensions::{
            allowlist::AllowList, blocklist::BlockList, burnable::Burnable, capped::Capped,
            total_supply::TotalSupply, votes::FungibleVotes,
        },
        Base, ContractOverrides,
    },
    rwa::{CappedRWA, RWA},
    vault::{CappedVault, Vault},
};

/// Resolves a list of contract types and additive extensions to the combined
/// contract type, e.g. `Compose<(AllowList,)>` or
/// `Compose<(AllowList, Burnable)>` (both resolve to `AllowList`), or
/// `Compose<(AllowList, FungibleVotes)>` (resolving to the curated
/// combination of the two).
///
/// This is shorthand for `<L as Composable>::Out`; refer to [`Composable`] for
/// the valid lists.
pub type Compose<L> = <L as Composable>::Out;

/// Type-level lookup backing [`Compose`]: each valid list resolves to its
/// combined contract type through the `Out` associated type.
///
/// A list is folded pairwise, left to right. Single contract types resolve to
/// themselves (with or without the one-element tuple form), curated pairs
/// resolve to their combined contract type (in either order), and additive
/// extensions are ignored, as is `TotalSupply` next to a contract type that
/// already tracks the supply. Contract types without a curated pairwise
/// combination (e.g. `AllowList` and `RWA`) cannot be listed together, so
/// such lists do not compile.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid contract type combination",
    note = "valid single contract types: `Base`, `AllowList`, `BlockList`, `TotalSupply`, \
            `FungibleVotes`, `RWA`, `Vault`, `Capped`",
    note = "curated combinations: `(AllowList, BlockList)`, `(AllowList, FungibleVotes)`, \
            `(BlockList, FungibleVotes)`, `(AllowList, BlockList, FungibleVotes)`, `(AllowList, \
            TotalSupply)`, `(BlockList, TotalSupply)`, `(AllowList, BlockList, TotalSupply)`",
    note = "capped combinations: `(AllowList, Capped)`, `(BlockList, Capped)`, `(AllowList, \
            BlockList, Capped)`, `(RWA, Capped)`, `(Vault, Capped)`",
    note = "`TotalSupply` may be listed next to `RWA`, `Vault` or `Capped`, which already track \
            the supply; it does not change the resolved type",
    note = "additive extensions (e.g. `Burnable`) may be listed alongside a contract type; they \
            do not affect the resolved type",
    note = "lists of up to 5 entries are supported"
)]
pub trait Composable {
    type Out: ContractOverrides;
}

// The resolution machinery below is internal: contract developers only
// interact with `Compose`, and the traits in this module are unnameable
// outside the crate.
mod fold {
    /// Identity element of composition: the contribution of an additive
    /// extension. Folding it into any contribution leaves that contribution
    /// unchanged, which is how additive extensions are ignored.
    pub enum Nil {}

    /// Implemented by every name that may appear in a `Compose` list. The
    /// `Contribution` is what the entry adds to the resolved contract type:
    /// contract types contribute themselves, additive extensions contribute
    /// [`Nil`].
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot appear in a `Compose` list",
        note = "valid entries are the contract types (`Base`, `AllowList`, `BlockList`, \
                `TotalSupply`, `RWA`, `Vault`, `FungibleVotes`, `Capped`) and the additive \
                extension `Burnable`"
    )]
    pub trait Extension {
        type Contribution;
    }

    /// Pairwise combination table for contributions: the fold reduces a list
    /// left to right through this table. A missing row means the pair is
    /// invalid (no curated combination exists).
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot be combined with `{B}` in a `Compose` list",
        note = "no combination of these contract types is curated"
    )]
    pub trait Combine<B> {
        type Out;
    }

    /// Final step of the fold: maps the folded contribution to the resolved
    /// contract type. Contract types resolve to themselves; [`Nil`] resolves
    /// to `Base` (a list of only additive extensions is a vanilla token).
    pub trait Finalize {
        type Out: crate::fungible::ContractOverrides;
    }
}

use fold::{Combine, Extension, Finalize, Nil};

type Contrib<T> = <T as Extension>::Contribution;
type Pair<A, B> = <A as Combine<B>>::Out;
type Final<T> = <T as Finalize>::Out;

// What each list entry contributes to the resolved contract type.
impl Extension for Base {
    type Contribution = Base;
}
impl Extension for AllowList {
    type Contribution = AllowList;
}
impl Extension for BlockList {
    type Contribution = BlockList;
}
impl Extension for TotalSupply {
    type Contribution = TotalSupply;
}
impl Extension for RWA {
    type Contribution = RWA;
}
impl Extension for Vault {
    type Contribution = Vault;
}
impl Extension for FungibleVotes {
    type Contribution = FungibleVotes;
}
impl Extension for Capped {
    type Contribution = Capped;
}
// Additive extensions contribute nothing.
impl Extension for Burnable {
    type Contribution = Nil;
}

// Identity rows: combining with `Nil` changes nothing. Any running result
// absorbs a following additive entry; a list that starts with additive
// entries takes the first contract type as its running result.
impl<X> Combine<Nil> for X {
    type Out = X;
}
impl Combine<Base> for Nil {
    type Out = Base;
}
impl Combine<AllowList> for Nil {
    type Out = AllowList;
}
impl Combine<BlockList> for Nil {
    type Out = BlockList;
}
impl Combine<TotalSupply> for Nil {
    type Out = TotalSupply;
}
impl Combine<FungibleVotes> for Nil {
    type Out = FungibleVotes;
}
impl Combine<RWA> for Nil {
    type Out = RWA;
}
impl Combine<Vault> for Nil {
    type Out = Vault;
}
impl Combine<Capped> for Nil {
    type Out = Capped;
}

// Curated multi-type combinations are declared in both orders so lists stay
// order-insensitive.
//
// Curated pairs.
impl Combine<BlockList> for AllowList {
    type Out = AllowBlockList;
}
impl Combine<AllowList> for BlockList {
    type Out = AllowBlockList;
}
impl Combine<FungibleVotes> for AllowList {
    type Out = AllowListVotes;
}
impl Combine<AllowList> for FungibleVotes {
    type Out = AllowListVotes;
}
impl Combine<FungibleVotes> for BlockList {
    type Out = BlockListVotes;
}
impl Combine<BlockList> for FungibleVotes {
    type Out = BlockListVotes;
}

// Curated triples: reached from any of their pairs by adding the missing
// member, so every ordering of the three resolves to the same type.
impl Combine<FungibleVotes> for AllowBlockList {
    type Out = AllowBlockListVotes;
}
impl Combine<BlockList> for AllowListVotes {
    type Out = AllowBlockListVotes;
}
impl Combine<AllowList> for BlockListVotes {
    type Out = AllowBlockListVotes;
}

// `TotalSupply` with the list policies: the result tracks the supply.
impl Combine<TotalSupply> for AllowList {
    type Out = TotalSupplyAllowList;
}
impl Combine<AllowList> for TotalSupply {
    type Out = TotalSupplyAllowList;
}
impl Combine<TotalSupply> for BlockList {
    type Out = TotalSupplyBlockList;
}
impl Combine<BlockList> for TotalSupply {
    type Out = TotalSupplyBlockList;
}
impl Combine<TotalSupply> for AllowBlockList {
    type Out = TotalSupplyAllowBlockList;
}
impl Combine<BlockList> for TotalSupplyAllowList {
    type Out = TotalSupplyAllowBlockList;
}
impl Combine<AllowList> for TotalSupplyBlockList {
    type Out = TotalSupplyAllowBlockList;
}

// `TotalSupply` next to a contract type that tracks the supply on its own is
// ignored, in either order. The rows are concrete (not one generic row over
// the self-tracking types) so that an invalid pair such as
// `(FungibleVotes, TotalSupply)` keeps the `Combine` error message.
impl Combine<TotalSupply> for RWA {
    type Out = RWA;
}
impl Combine<TotalSupply> for Vault {
    type Out = Vault;
}
impl Combine<TotalSupply> for Capped {
    type Out = Capped;
}
impl Combine<TotalSupply> for CappedAllowList {
    type Out = CappedAllowList;
}
impl Combine<TotalSupply> for CappedBlockList {
    type Out = CappedBlockList;
}
impl Combine<TotalSupply> for CappedAllowBlockList {
    type Out = CappedAllowBlockList;
}
impl Combine<TotalSupply> for CappedRWA {
    type Out = CappedRWA;
}
impl Combine<TotalSupply> for CappedVault {
    type Out = CappedVault;
}
impl Combine<RWA> for TotalSupply {
    type Out = RWA;
}
impl Combine<Vault> for TotalSupply {
    type Out = Vault;
}
impl Combine<Capped> for TotalSupply {
    type Out = Capped;
}

// Curated capped combinations. A list policy that already tracks the supply
// (`TotalSupply` listed before `Capped`) resolves to the same capped type.
impl Combine<AllowList> for Capped {
    type Out = CappedAllowList;
}
impl Combine<Capped> for AllowList {
    type Out = CappedAllowList;
}
impl Combine<Capped> for TotalSupplyAllowList {
    type Out = CappedAllowList;
}
impl Combine<BlockList> for Capped {
    type Out = CappedBlockList;
}
impl Combine<Capped> for BlockList {
    type Out = CappedBlockList;
}
impl Combine<Capped> for TotalSupplyBlockList {
    type Out = CappedBlockList;
}
impl Combine<Capped> for AllowBlockList {
    type Out = CappedAllowBlockList;
}
impl Combine<Capped> for TotalSupplyAllowBlockList {
    type Out = CappedAllowBlockList;
}
impl Combine<BlockList> for CappedAllowList {
    type Out = CappedAllowBlockList;
}
impl Combine<AllowList> for CappedBlockList {
    type Out = CappedAllowBlockList;
}
impl Combine<RWA> for Capped {
    type Out = CappedRWA;
}
impl Combine<Capped> for RWA {
    type Out = CappedRWA;
}
impl Combine<Vault> for Capped {
    type Out = CappedVault;
}
impl Combine<Capped> for Vault {
    type Out = CappedVault;
}

// Every running result other than `Nil` is a contract type and resolves to
// itself.
impl Finalize for Nil {
    type Out = Base;
}
impl<X: ContractOverrides> Finalize for X {
    type Out = X;
}

// Bare forms: a single entry may also be written without the one-element
// tuple, so a stray trailing comma (or its absence) does not change the
// meaning.
impl Composable for Base {
    type Out = Base;
}
impl Composable for AllowList {
    type Out = AllowList;
}
impl Composable for BlockList {
    type Out = BlockList;
}
impl Composable for TotalSupply {
    type Out = TotalSupply;
}
impl Composable for FungibleVotes {
    type Out = FungibleVotes;
}
impl Composable for RWA {
    type Out = RWA;
}
impl Composable for Vault {
    type Out = Vault;
}
impl Composable for Capped {
    type Out = Capped;
}
impl Composable for Burnable {
    type Out = Base;
}

// Lists are folded left to right through the `Combine` table; additive
// entries contribute `Nil`, the identity, so they vanish. Longer lists are
// supported by adding one impl per arity.
impl<A> Composable for (A,)
where
    A: Extension,
    Contrib<A>: Finalize,
{
    type Out = Final<Contrib<A>>;
}

impl<A, B> Composable for (A, B)
where
    A: Extension,
    B: Extension,
    Contrib<A>: Combine<Contrib<B>>,
    Pair<Contrib<A>, Contrib<B>>: Finalize,
{
    type Out = Final<Pair<Contrib<A>, Contrib<B>>>;
}

impl<A, B, C> Composable for (A, B, C)
where
    A: Extension,
    B: Extension,
    C: Extension,
    Contrib<A>: Combine<Contrib<B>>,
    Pair<Contrib<A>, Contrib<B>>: Combine<Contrib<C>>,
    Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>: Finalize,
{
    type Out = Final<Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>>;
}

impl<A, B, C, D> Composable for (A, B, C, D)
where
    A: Extension,
    B: Extension,
    C: Extension,
    D: Extension,
    Contrib<A>: Combine<Contrib<B>>,
    Pair<Contrib<A>, Contrib<B>>: Combine<Contrib<C>>,
    Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>: Combine<Contrib<D>>,
    Pair<Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>, Contrib<D>>: Finalize,
{
    type Out = Final<Pair<Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>, Contrib<D>>>;
}

impl<A, B, C, D, E> Composable for (A, B, C, D, E)
where
    A: Extension,
    B: Extension,
    C: Extension,
    D: Extension,
    E: Extension,
    Contrib<A>: Combine<Contrib<B>>,
    Pair<Contrib<A>, Contrib<B>>: Combine<Contrib<C>>,
    Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>: Combine<Contrib<D>>,
    Pair<Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>, Contrib<D>>: Combine<Contrib<E>>,
    Pair<Pair<Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>, Contrib<D>>, Contrib<E>>: Finalize,
{
    type Out =
        Final<Pair<Pair<Pair<Pair<Contrib<A>, Contrib<B>>, Contrib<C>>, Contrib<D>>, Contrib<E>>>;
}
