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
//! [`Capped`] (for [`crate::fungible::capped::FungibleCapped`]) is additive
//! too, but a cap needs the supply to be tracked, so it requires
//! `TotalSupply` in the list, like `RWA` and `Vault`:
//! `Compose<(Capped, TotalSupply)>` resolves to `TotalSupply`,
//! `Compose<(AllowList, Capped, TotalSupply)>` to the allowlist
//! supply-tracking combination, and a list with `Capped` but without
//! `TotalSupply` is rejected.
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
//! `Capped` require `TotalSupply` next to them, e.g.
//! `Compose<(RWA, TotalSupply)>`; a list with one of them but without
//! `TotalSupply` is rejected with a dedicated compile error, as is a list
//! naming `TotalSupply` twice.
//! `FungibleVotes` tracks voting units, not the token supply, and has no
//! curated combination with `TotalSupply`. The resolved contract type
//! enforces every listed transfer
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
            allowlist::AllowList, blocklist::BlockList, burnable::Burnable, capped::Capped,
            total_supply::TotalSupply, votes::FungibleVotes,
        },
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
    note = "valid single contract types: `Base`, `AllowList`, `BlockList`, `TotalSupply`, \
            `FungibleVotes`",
    note = "curated combinations: `(AllowList, BlockList)`, `(AllowList, FungibleVotes)`, \
            `(BlockList, FungibleVotes)`, `(AllowList, BlockList, FungibleVotes)`, `(AllowList, \
            TotalSupply)`, `(BlockList, TotalSupply)`, `(AllowList, BlockList, TotalSupply)`, \
            `(RWA, TotalSupply)`, `(Vault, TotalSupply)`",
    note = "`RWA`, `Vault` and `Capped` require `TotalSupply` in the list",
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
                (`Burnable`, `Capped`)"
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

    /// Contribution of an entry that only works with the total supply
    /// tracked (`RWA`, `Vault`, `Capped`): the contribution `C` it stands
    /// for, plus an open requirement that `TotalSupply` appears somewhere in
    /// the list.
    ///
    /// This gives `RWA` and `Vault` two roles inside the fold. The entry a
    /// developer writes contributes `NeedsSupply<RWA>`, never the bare `RWA`.
    /// The bare `RWA` only shows up as the running result after `TotalSupply`
    /// has been met. So every row below with a bare `RWA` or `Vault` on its
    /// left-hand side applies to a list that already contains `TotalSupply`.
    ///
    /// Meeting `TotalSupply` removes the wrapper (refer to [`WithSupply`]);
    /// other entries fold into `C` and keep the requirement open; a list that
    /// ends with it open is rejected by [`Finalize`] through
    /// [`SupplyRequirementMet`].
    #[allow(dead_code)] // type-level marker, never constructed
    pub struct NeedsSupply<C>(core::marker::PhantomData<C>);

    /// Resolution of a contribution once `TotalSupply` joins it: the list
    /// policies resolve to their supply-tracking combination, the
    /// self-tracking contract types to themselves, and [`Nil`] to
    /// `TotalSupply`.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot be combined with `TotalSupply` in a `Compose` list",
        note = "no combination of these contract types is curated"
    )]
    pub trait WithSupply {
        type Out;
    }

    /// Error carrier for a list that ends with the `TotalSupply` requirement
    /// open, e.g. `Compose<(RWA,)>`. Nothing implements it, on purpose: it
    /// exists only so that its message is printed when the [`Finalize`] row
    /// for [`NeedsSupply`] fails its `where` clause. Refer to [`Probe`] for
    /// how these error rows work.
    #[diagnostic::on_unimplemented(
        message = "this `Compose` list requires `TotalSupply`",
        note = "`RWA`, `Vault` and `Capped` only work with the total supply tracked; add \
                `TotalSupply` to the list, e.g. `Compose<(RWA, TotalSupply)>`"
    )]
    pub trait SupplyRequirementMet {}

    /// Error carrier for `TotalSupply` appearing twice in a list, e.g.
    /// `Compose<(RWA, TotalSupply, TotalSupply)>`. Nothing implements it, on
    /// purpose: it exists only so that its message is printed when one of the
    /// duplicate rows fails its `where` clause. Refer to [`Probe`] for how
    /// these error rows work.
    #[diagnostic::on_unimplemented(
        message = "`TotalSupply` is listed more than once in this `Compose` list",
        note = "list `TotalSupply` once"
    )]
    pub trait SupplyListedOnce {}

    /// Lends a type parameter to the error rows. It carries no meaning of
    /// its own. Why it is needed, step by step, using the row that rejects a
    /// second `TotalSupply` after `RWA`:
    ///
    /// 1. An error row is an impl that can never succeed. Its `where` clause
    ///    demands a trait nothing implements ([`SupplyListedOnce`]), and that
    ///    trait's `on_unimplemented` attribute carries the message. When the
    ///    compiler selects the row and fails that clause, it prints the message
    ///    instead of the generic [`Combine`] one.
    /// 2. The obvious spelling, `impl Combine<TotalSupply> for RWA where RWA:
    ///    SupplyListedOnce`, does not compile. A `where` clause that mentions
    ///    no type parameter is evaluated while the library itself is being
    ///    compiled, and since it is false, the crate is rejected (Rust calls
    ///    these trivial bounds).
    /// 3. Making the row generic in `Self` or in the trait argument defers the
    ///    check but breaks the message. The compiler picks candidate impls by
    ///    shape alone, so a generic row is the candidate for every unrelated
    ///    pair as well, and prints its message for them.
    /// 4. The row therefore has to stay concrete in both positions and still
    ///    name a type parameter. `impl<T>` provides it and defers the check.
    ///    The clause `RWA: Probe<Never = T>` ties `T` to the impl, which Rust
    ///    requires of every type parameter, and pins it to [`Never`], since
    ///    every type implements `Probe` that way. `T: SupplyListedOnce` is the
    ///    clause that fails at the use site and selects the message.
    pub trait Probe {
        type Never;
    }

    /// The type every [`Probe`] projects to. Empty because nothing is ever
    /// constructed from it; it only exists to be the `T` of the error rows.
    pub enum Never {}

    impl<T> Probe for T {
        type Never = Never;
    }

    /// Final step of the fold: maps the folded contribution to the resolved
    /// contract type. Its only non-trivial rule is [`Nil`] resolving to
    /// `Base` (a list of only additive extensions is a vanilla token).
    pub trait Finalize {
        type Out: super::ContractOverrides;
    }
}

use fold::{
    Combine, Extension, Finalize, NeedsSupply, Nil, Probe, SupplyListedOnce, SupplyRequirementMet,
    WithSupply,
};

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
// `RWA` and `Vault` only work with the total supply tracked, so they open a
// requirement that `TotalSupply` appears in the list. Inside the fold, the
// bare `RWA` and `Vault` mean that requirement has been met (refer to
// `NeedsSupply`).
impl Extension for RWA {
    type Contribution = NeedsSupply<RWA>;
}
impl Extension for Vault {
    type Contribution = NeedsSupply<Vault>;
}
impl Extension for FungibleVotes {
    type Contribution = FungibleVotes;
}
// Additive extensions contribute nothing.
impl Extension for Burnable {
    type Contribution = Nil;
}
// `Capped` is additive as well, but a cap needs the supply to be tracked, so
// it contributes nothing plus the requirement that `TotalSupply` appears in
// the list.
impl Extension for Capped {
    type Contribution = NeedsSupply<Nil>;
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

// What a contribution resolves to once `TotalSupply` joins it.
impl WithSupply for Nil {
    type Out = TotalSupply;
}
impl WithSupply for AllowList {
    type Out = TotalSupplyAllowList;
}
impl WithSupply for BlockList {
    type Out = TotalSupplyBlockList;
}
impl WithSupply for AllowBlockList {
    type Out = TotalSupplyAllowBlockList;
}
impl WithSupply for RWA {
    type Out = RWA;
}
impl WithSupply for Vault {
    type Out = Vault;
}

// Open `TotalSupply` requirement, seen from the accumulator side: the running
// result is `NeedsSupply<X>` and the next entry arrives.
//
// `TotalSupply` closes it.
impl<X: WithSupply> Combine<TotalSupply> for NeedsSupply<X> {
    type Out = <X as WithSupply>::Out;
}
// Any other entry folds into the wrapped contribution and keeps it open.
impl<X: Combine<Nil>> Combine<Nil> for NeedsSupply<X> {
    type Out = NeedsSupply<Pair<X, Nil>>;
}
impl<X: Combine<Base>> Combine<Base> for NeedsSupply<X> {
    type Out = NeedsSupply<Pair<X, Base>>;
}
impl<X: Combine<AllowList>> Combine<AllowList> for NeedsSupply<X> {
    type Out = NeedsSupply<Pair<X, AllowList>>;
}
impl<X: Combine<BlockList>> Combine<BlockList> for NeedsSupply<X> {
    type Out = NeedsSupply<Pair<X, BlockList>>;
}
impl<X: Combine<FungibleVotes>> Combine<FungibleVotes> for NeedsSupply<X> {
    type Out = NeedsSupply<Pair<X, FungibleVotes>>;
}
impl<X: Combine<Y>, Y> Combine<NeedsSupply<Y>> for NeedsSupply<X> {
    type Out = NeedsSupply<Pair<X, Y>>;
}

// Open `TotalSupply` requirement, seen from the entry side: the next entry is
// `RWA` or `Vault`, contributing `NeedsSupply<Y>`.
//
// An accumulator that already holds the supply satisfies it on the spot.
impl<Y: WithSupply> Combine<NeedsSupply<Y>> for TotalSupply {
    type Out = <Y as WithSupply>::Out;
}
impl<Y> Combine<NeedsSupply<Y>> for TotalSupplyAllowList
where
    TotalSupplyAllowList: Combine<Y>,
{
    type Out = Pair<TotalSupplyAllowList, Y>;
}
impl<Y> Combine<NeedsSupply<Y>> for TotalSupplyBlockList
where
    TotalSupplyBlockList: Combine<Y>,
{
    type Out = Pair<TotalSupplyBlockList, Y>;
}
impl<Y> Combine<NeedsSupply<Y>> for TotalSupplyAllowBlockList
where
    TotalSupplyAllowBlockList: Combine<Y>,
{
    type Out = Pair<TotalSupplyAllowBlockList, Y>;
}
impl<Y> Combine<NeedsSupply<Y>> for RWA
where
    RWA: Combine<Y>,
{
    type Out = Pair<RWA, Y>;
}
impl<Y> Combine<NeedsSupply<Y>> for Vault
where
    Vault: Combine<Y>,
{
    type Out = Pair<Vault, Y>;
}
// An accumulator without the supply takes the requirement over.
impl<Y> Combine<NeedsSupply<Y>> for Nil
where
    Nil: Combine<Y>,
{
    type Out = NeedsSupply<Pair<Nil, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for Base
where
    Base: Combine<Y>,
{
    type Out = NeedsSupply<Pair<Base, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for AllowList
where
    AllowList: Combine<Y>,
{
    type Out = NeedsSupply<Pair<AllowList, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for BlockList
where
    BlockList: Combine<Y>,
{
    type Out = NeedsSupply<Pair<BlockList, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for AllowBlockList
where
    AllowBlockList: Combine<Y>,
{
    type Out = NeedsSupply<Pair<AllowBlockList, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for FungibleVotes
where
    FungibleVotes: Combine<Y>,
{
    type Out = NeedsSupply<Pair<FungibleVotes, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for AllowListVotes
where
    AllowListVotes: Combine<Y>,
{
    type Out = NeedsSupply<Pair<AllowListVotes, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for BlockListVotes
where
    BlockListVotes: Combine<Y>,
{
    type Out = NeedsSupply<Pair<BlockListVotes, Y>>;
}
impl<Y> Combine<NeedsSupply<Y>> for AllowBlockListVotes
where
    AllowBlockListVotes: Combine<Y>,
{
    type Out = NeedsSupply<Pair<AllowBlockListVotes, Y>>;
}

// `TotalSupply` listed twice. These rows never succeed: they exist only to
// select the error message (refer to `Probe`). Their left-hand side is the
// running result, not the entry a developer wrote, and a bare `RWA` or `Vault`
// only becomes the running result after `TotalSupply` was met (refer to
// `NeedsSupply`), so a `TotalSupply` arriving here is always a second one. One
// concrete row per pair.
impl<T> Combine<TotalSupply> for TotalSupply
where
    TotalSupply: Probe<Never = T>,
    T: SupplyListedOnce,
{
    type Out = TotalSupply;
}
impl<T> Combine<TotalSupply> for TotalSupplyAllowList
where
    TotalSupplyAllowList: Probe<Never = T>,
    T: SupplyListedOnce,
{
    type Out = TotalSupplyAllowList;
}
impl<T> Combine<TotalSupply> for TotalSupplyBlockList
where
    TotalSupplyBlockList: Probe<Never = T>,
    T: SupplyListedOnce,
{
    type Out = TotalSupplyBlockList;
}
impl<T> Combine<TotalSupply> for TotalSupplyAllowBlockList
where
    TotalSupplyAllowBlockList: Probe<Never = T>,
    T: SupplyListedOnce,
{
    type Out = TotalSupplyAllowBlockList;
}
impl<T> Combine<TotalSupply> for RWA
where
    RWA: Probe<Never = T>,
    T: SupplyListedOnce,
{
    type Out = RWA;
}
impl<T> Combine<TotalSupply> for Vault
where
    Vault: Probe<Never = T>,
    T: SupplyListedOnce,
{
    type Out = Vault;
}

impl Finalize for Nil {
    type Out = Base;
}
// A list that ends with the `TotalSupply` requirement open, e.g. `(RWA,)`.
// This row never succeeds; it only selects the error message (refer to
// `Probe`).
impl<X> Finalize for NeedsSupply<X>
where
    NeedsSupply<X>: SupplyRequirementMet,
{
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
// `RWA`, `Vault` and `Capped` written bare, i.e. `Compose<RWA>`, miss
// `TotalSupply` by construction. These rows never succeed; they only select
// the error message (refer to `Probe`).
impl<T> Composable for Capped
where
    Capped: Probe<Never = T>,
    T: SupplyRequirementMet,
{
    type Out = TotalSupply;
}
impl<T> Composable for RWA
where
    RWA: Probe<Never = T>,
    T: SupplyRequirementMet,
{
    type Out = RWA;
}
impl<T> Composable for Vault
where
    Vault: Probe<Never = T>,
    T: SupplyRequirementMet,
{
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
