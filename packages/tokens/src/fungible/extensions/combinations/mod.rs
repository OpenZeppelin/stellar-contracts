//! # Contract Type Composition for Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::fungible::FungibleToken`] implementation, and that single slot
//! decides all overridable behavior. The contract type is selected uniformly
//! with [`Compose`], by listing the extensions the token is made of.
//!
//! Additive extensions (e.g. [`Burnable`] for
//! [`crate::fungible::burnable::FungibleBurnable`]) may be listed as well:
//! they do not affect the resolved contract type and are enabled by
//! implementing their trait, whether or not they are listed. A list holding
//! only additive extensions resolves to [`Base`].
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
//! `Compose<(AllowList, BlockList, TotalSupply)>`. `RWA`, `Vault` and
//! `FungibleVotes` (alone or combined with the lists) already track the
//! supply on their own, so listing `TotalSupply` next to them is rejected
//! with a dedicated compile error. The resolved contract type enforces every
//! listed transfer
//! policy, tracks voting units when `FungibleVotes` is listed, tracks the
//! total supply when `TotalSupply` is listed, and backs the corresponding
//! extension traits. The list is order-insensitive:
//! `Compose<(BlockList, AllowList)>` resolves to the same contract type as
//! `Compose<(AllowList, BlockList)>`. Invalid lists do not compile: contract
//! types without a curated combination (e.g. `AllowList` with `RWA`) cannot
//! be listed together, and implementing an extension trait the list does not
//! back (e.g. [`crate::fungible::total_supply::FungibleTotalSupply`] without
//! `TotalSupply` in the list) is rejected by that trait's bound.

mod storage;

#[cfg(test)]
mod test;

use storage::{
    AllowBlockList, AllowBlockListVotes, AllowListVotes, BlockListVotes, TotalSupplyAllowBlockList,
    TotalSupplyAllowList, TotalSupplyBlockList,
};

use crate::{
    fungible::{
        extensions::{
            allowlist::AllowList, blocklist::BlockList, burnable::Burnable,
            total_supply::TotalSupply, votes::FungibleVotes,
        },
        overrides::TotalSupplyOverrides,
        Base, ContractOverrides,
    },
    rwa::RWA,
    vault::Vault,
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
/// extensions are ignored. Contract types without a curated pairwise
/// combination (e.g. `AllowList` and `RWA`) cannot be listed together, so
/// such lists do not compile.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid contract type combination",
    note = "valid single contract types: `Base`, `AllowList`, `BlockList`, `TotalSupply`, `RWA`, \
            `Vault`, `FungibleVotes`",
    note = "curated combinations: `(AllowList, BlockList)`, `(AllowList, FungibleVotes)`, \
            `(BlockList, FungibleVotes)`, `(AllowList, BlockList, FungibleVotes)`, `(AllowList, \
            TotalSupply)`, `(BlockList, TotalSupply)`, `(AllowList, BlockList, TotalSupply)`",
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
                `TotalSupply`, `RWA`, `Vault`, `FungibleVotes`) and the additive extensions \
                (`Burnable`)"
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

    /// Error carrier for `TotalSupply` listed next to a contract type `By`
    /// that already tracks the supply on its own. It is never implemented:
    /// the redundancy rows below name it in a `where` clause, so the failure
    /// is reported with this message instead of the generic [`Combine`] one.
    /// Each of those rows has to be the only impl unifying with its pair,
    /// otherwise rustc winnows the candidates and falls back to the generic
    /// message.
    #[diagnostic::on_unimplemented(
        message = "`TotalSupply` is redundant in this `Compose` list: `{By}` already tracks the \
                   total supply",
        note = "remove `TotalSupply` from the list; `FungibleTotalSupply` is implemented on the \
                contract directly (`{By}` requires it)"
    )]
    pub trait SupplyAlreadyTracked<By> {}

    /// Contract types built on the `TotalSupply` counter.
    pub trait CountedSupply {}

    /// Final step of the fold: maps the folded contribution to the resolved
    /// contract type. Its only non-trivial rule is [`Nil`] resolving to
    /// `Base` (a list of only additive extensions is a vanilla token).
    pub trait Finalize {
        type Out: super::ContractOverrides;
    }
}

use fold::{Combine, CountedSupply, Extension, Finalize, Nil, SupplyAlreadyTracked};

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
// Additive extensions contribute nothing.
impl Extension for Burnable {
    type Contribution = Nil;
}

// Identity rows: combining with `Nil` changes nothing. One pair of rows per
// contract type, plus the `Nil`/`Nil` row for lists of only additive
// extensions. Curated multi-type combinations are additional rows, declared
// in both orders so lists stay order-insensitive.
impl Combine<Nil> for Nil {
    type Out = Nil;
}
impl Combine<Nil> for Base {
    type Out = Base;
}
impl Combine<Base> for Nil {
    type Out = Base;
}
impl Combine<Nil> for AllowList {
    type Out = AllowList;
}
impl Combine<AllowList> for Nil {
    type Out = AllowList;
}
impl Combine<Nil> for BlockList {
    type Out = BlockList;
}
impl Combine<BlockList> for Nil {
    type Out = BlockList;
}
impl Combine<Nil> for TotalSupply {
    type Out = TotalSupply;
}
impl Combine<TotalSupply> for Nil {
    type Out = TotalSupply;
}
impl Combine<Nil> for RWA {
    type Out = RWA;
}
impl Combine<RWA> for Nil {
    type Out = RWA;
}
impl Combine<Nil> for Vault {
    type Out = Vault;
}
impl Combine<Vault> for Nil {
    type Out = Vault;
}
impl Combine<Nil> for FungibleVotes {
    type Out = FungibleVotes;
}
impl Combine<FungibleVotes> for Nil {
    type Out = FungibleVotes;
}
impl Combine<Nil> for AllowBlockList {
    type Out = AllowBlockList;
}
impl Combine<Nil> for AllowListVotes {
    type Out = AllowListVotes;
}
impl Combine<Nil> for BlockListVotes {
    type Out = BlockListVotes;
}
impl Combine<Nil> for AllowBlockListVotes {
    type Out = AllowBlockListVotes;
}
impl Combine<Nil> for TotalSupplyAllowList {
    type Out = TotalSupplyAllowList;
}
impl Combine<Nil> for TotalSupplyBlockList {
    type Out = TotalSupplyBlockList;
}
impl Combine<Nil> for TotalSupplyAllowBlockList {
    type Out = TotalSupplyAllowBlockList;
}

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
impl Combine<TotalSupply> for AllowBlockList {
    type Out = TotalSupplyAllowBlockList;
}
impl Combine<BlockList> for TotalSupplyAllowList {
    type Out = TotalSupplyAllowBlockList;
}
impl Combine<AllowList> for TotalSupplyBlockList {
    type Out = TotalSupplyAllowBlockList;
}

// Redundant `TotalSupply`: contract types that already answer
// `total_supply` (`RWA`, `Vault`, `FungibleVotes` and its combinations, or
// the counter itself) reject the marker with a dedicated error. The `where`
// clauses are never satisfied; they only select the message.
impl CountedSupply for TotalSupply {}
impl CountedSupply for TotalSupplyAllowList {}
impl CountedSupply for TotalSupplyBlockList {}
impl CountedSupply for TotalSupplyAllowBlockList {}
impl<S: TotalSupplyOverrides> Combine<TotalSupply> for S
where
    S: SupplyAlreadyTracked<S>,
{
    type Out = S;
}
impl<C: CountedSupply> Combine<RWA> for C
where
    C: SupplyAlreadyTracked<RWA>,
{
    type Out = C;
}
impl<C: CountedSupply> Combine<Vault> for C
where
    C: SupplyAlreadyTracked<Vault>,
{
    type Out = C;
}
impl<C: CountedSupply> Combine<FungibleVotes> for C
where
    C: SupplyAlreadyTracked<FungibleVotes>,
{
    type Out = C;
}

impl Finalize for Nil {
    type Out = Base;
}
impl Finalize for Base {
    type Out = Base;
}
impl Finalize for AllowList {
    type Out = AllowList;
}
impl Finalize for BlockList {
    type Out = BlockList;
}
impl Finalize for TotalSupply {
    type Out = TotalSupply;
}
impl Finalize for RWA {
    type Out = RWA;
}
impl Finalize for Vault {
    type Out = Vault;
}
impl Finalize for FungibleVotes {
    type Out = FungibleVotes;
}
impl Finalize for AllowBlockList {
    type Out = AllowBlockList;
}
impl Finalize for AllowListVotes {
    type Out = AllowListVotes;
}
impl Finalize for BlockListVotes {
    type Out = BlockListVotes;
}
impl Finalize for AllowBlockListVotes {
    type Out = AllowBlockListVotes;
}
impl Finalize for TotalSupplyAllowList {
    type Out = TotalSupplyAllowList;
}
impl Finalize for TotalSupplyBlockList {
    type Out = TotalSupplyBlockList;
}
impl Finalize for TotalSupplyAllowBlockList {
    type Out = TotalSupplyAllowBlockList;
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
impl Composable for RWA {
    type Out = RWA;
}
impl Composable for Vault {
    type Out = Vault;
}
impl Composable for FungibleVotes {
    type Out = FungibleVotes;
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
