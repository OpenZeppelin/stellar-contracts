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
//! [`Capped`] (for [`crate::fungible::capped::FungibleCapped`]) is a contract
//! type: its mint checks the cap. A cap needs the supply to be tracked, so it
//! requires `TotalSupply` in the list, like `RWA` and `Vault`:
//! `Compose<(Capped, TotalSupply)>` resolves to `Capped`, `Capped` combined
//! with `AllowList`, `BlockList` (or both), `RWA` or `Vault` resolves to the
//! matching capped combination (e.g. `Compose<(AllowList, Capped,
//! TotalSupply)>` to `CappedAllowList`), and a list with `Capped` but without
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
//! `Compose<(AllowList, BlockList, TotalSupply)>`. `Capped` can be added to
//! any of those supply-tracking lists, and to `RWA` or `Vault`, e.g.
//! `Compose<(AllowList, Capped, TotalSupply)>` or
//! `Compose<(Vault, Capped, TotalSupply)>`. `RWA`, `Vault` and
//! `Capped` require `TotalSupply` next to them, e.g.
//! `Compose<(RWA, TotalSupply)>`; a list with one of them but without
//! `TotalSupply` (e.g. `Compose<(RWA,)>`) is rejected with a dedicated
//! compile error. The bare form (`Compose<RWA>`) is rejected too, with the
//! generic error.
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
    note = "capped combinations: `(Capped, TotalSupply)`, `(AllowList, Capped, TotalSupply)`, \
            `(BlockList, Capped, TotalSupply)`, `(AllowList, BlockList, Capped, TotalSupply)`, \
            `(RWA, Capped, TotalSupply)`, `(Vault, Capped, TotalSupply)`",
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

    /// Contribution of an entry that only works with the total supply
    /// tracked (`RWA`, `Vault`, `Capped`): the contribution `C` it stands
    /// for, plus an open requirement that `TotalSupply` appears somewhere in
    /// the list.
    ///
    /// The supply state of a running result is tracked by its wrapper: a
    /// bare contribution has no supply (except `TotalSupply` itself, until a
    /// contract type arrives), `NeedsSupply<C>` is waiting for `TotalSupply`,
    /// and [`HasSupply<C>`](HasSupply) has it. The bare `RWA`,
    /// `Vault`, `Capped` and capped combinations therefore only ever appear
    /// inside one of the two wrappers.
    ///
    /// Meeting `TotalSupply` turns the wrapper into [`HasSupply`]; other
    /// entries fold into `C` and keep the requirement open; a list that ends
    /// with it open is rejected by [`Finalize`] through
    /// [`SupplyRequirementMet`].
    #[allow(dead_code)] // type-level marker, never constructed
    pub struct NeedsSupply<C>(core::marker::PhantomData<C>);

    /// Running result of a list whose `TotalSupply` requirement is met: the
    /// contribution `C` it stands for, now tracking the total supply. Reached
    /// when `TotalSupply` meets `AllowList`, `BlockList`, `AllowBlockList` or
    /// an open [`NeedsSupply`] requirement, in either order. (`TotalSupply`
    /// next to only additive entries stays the bare `TotalSupply`.) Later
    /// entries fold into `C` and keep the wrapper. [`Finalize`] maps it to the
    /// supply-tracking contract type of `C` (refer to [`WithSupply`]).
    #[allow(dead_code)] // type-level marker, never constructed
    pub struct HasSupply<C>(core::marker::PhantomData<C>);

    /// Resolution of a [`HasSupply`] contribution by [`Finalize`]: the list
    /// policies resolve to their supply-tracking combination, and the
    /// self-tracking contract types (`RWA`, `Vault`, the capped contract
    /// types) to themselves.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot be combined with `TotalSupply` in a `Compose` list",
        note = "no combination of these contract types is curated"
    )]
    pub trait WithSupply {
        type Out: crate::fungible::ContractOverrides;
    }

    /// Error carrier for a list that ends with the `TotalSupply` requirement
    /// open, e.g. `Compose<(RWA,)>`. Nothing implements it, on purpose: it
    /// exists only so that its message is printed when the [`Finalize`] row
    /// for [`NeedsSupply`] fails its `where` clause.
    #[diagnostic::on_unimplemented(
        message = "this `Compose` list requires `TotalSupply`",
        note = "`RWA`, `Vault` and `Capped` only work with the total supply tracked; add \
                `TotalSupply` to the list, e.g. `Compose<(RWA, TotalSupply)>`"
    )]
    pub trait SupplyRequirementMet {}

    /// Final step of the fold: maps the folded contribution to the resolved
    /// contract type. Contract types resolve to themselves; the non-trivial
    /// rules are [`Nil`] resolving to `Base` (a list of only additive
    /// extensions is a vanilla token), and [`HasSupply<C>`](HasSupply)
    /// resolving to the supply-tracking contract type of `C` through
    /// [`WithSupply`].
    pub trait Finalize {
        type Out: crate::fungible::ContractOverrides;
    }
}

use fold::{
    Combine, Extension, Finalize, HasSupply, NeedsSupply, Nil, SupplyRequirementMet, WithSupply,
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
// requirement that `TotalSupply` appears in the list (refer to
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
// `Capped` is a contract type of its own (its mint checks the cap), and like
// `RWA` and `Vault` it needs the supply to be tracked, so it opens the same
// requirement. Combined with a list policy, `RWA` or `Vault`, it resolves to
// the matching capped combination.
impl Extension for Capped {
    type Contribution = NeedsSupply<Capped>;
}
// Additive extensions contribute nothing.
impl Extension for Burnable {
    type Contribution = Nil;
}

// Identity rows: combining with `Nil` changes nothing. Both orders for the
// contract types an entry contributes bare (`TotalSupply` has its own rows
// below), plus the `Nil`/`Nil` row for lists of only additive extensions.
// Combined types only need `X + Nil`, since they are never the right-hand
// side. `Nil + RWA`, `Nil + Vault` and `Nil + Capped` serve the requirement
// rows below (`Nil: Combine<Y>` when a list starts with additive entries).
// Curated multi-type combinations are additional rows, declared in both
// orders so lists stay order-insensitive.
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
impl Combine<Nil> for FungibleVotes {
    type Out = FungibleVotes;
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

// Curated capped combinations. They are only ever reached inside a
// `NeedsSupply` or `HasSupply` wrapper, since `Capped`, `RWA` and `Vault`
// always contribute through `NeedsSupply`.
impl Combine<AllowList> for Capped {
    type Out = CappedAllowList;
}
impl Combine<Capped> for AllowList {
    type Out = CappedAllowList;
}
impl Combine<BlockList> for Capped {
    type Out = CappedBlockList;
}
impl Combine<Capped> for BlockList {
    type Out = CappedBlockList;
}
impl Combine<Capped> for AllowBlockList {
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

// `TotalSupply` meets a running result without a supply requirement: the
// result gets the supply. After only additive entries, the running result
// is the bare `TotalSupply` itself.
impl Combine<TotalSupply> for Nil {
    type Out = TotalSupply;
}
impl Combine<TotalSupply> for AllowList {
    type Out = HasSupply<AllowList>;
}
impl Combine<TotalSupply> for BlockList {
    type Out = HasSupply<BlockList>;
}
impl Combine<TotalSupply> for AllowBlockList {
    type Out = HasSupply<AllowBlockList>;
}
// `TotalSupply` as the running result (first entry, or after only additive
// entries): additive entries keep it bare, and the next contract type gets
// the supply. There is no row for a second `TotalSupply`, so a duplicate
// fails right here.
impl Combine<Nil> for TotalSupply {
    type Out = TotalSupply;
}
impl Combine<AllowList> for TotalSupply {
    type Out = HasSupply<AllowList>;
}
impl Combine<BlockList> for TotalSupply {
    type Out = HasSupply<BlockList>;
}
impl<Y> Combine<NeedsSupply<Y>> for TotalSupply {
    type Out = HasSupply<Y>;
}

// Open `TotalSupply` requirement: the running result is `NeedsSupply<X>`.
//
// `TotalSupply` meets it.
impl<X> Combine<TotalSupply> for NeedsSupply<X> {
    type Out = HasSupply<X>;
}
// Any other entry folds into the wrapped contribution and keeps it open.
impl<X> Combine<Nil> for NeedsSupply<X> {
    type Out = NeedsSupply<X>;
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

// Met `TotalSupply` requirement: the running result is `HasSupply<X>`. Every
// entry except a second `TotalSupply` folds into the wrapped contribution;
// whether the result may track the supply is checked once, by `Finalize`.
impl<X> Combine<Nil> for HasSupply<X> {
    type Out = HasSupply<X>;
}
impl<X: Combine<Base>> Combine<Base> for HasSupply<X> {
    type Out = HasSupply<Pair<X, Base>>;
}
impl<X: Combine<AllowList>> Combine<AllowList> for HasSupply<X> {
    type Out = HasSupply<Pair<X, AllowList>>;
}
impl<X: Combine<BlockList>> Combine<BlockList> for HasSupply<X> {
    type Out = HasSupply<Pair<X, BlockList>>;
}
impl<X: Combine<FungibleVotes>> Combine<FungibleVotes> for HasSupply<X> {
    type Out = HasSupply<Pair<X, FungibleVotes>>;
}
impl<X: Combine<Y>, Y> Combine<NeedsSupply<Y>> for HasSupply<X> {
    type Out = HasSupply<Pair<X, Y>>;
}

// A running result without the supply takes an incoming requirement over.
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

// What a contribution resolves to once it tracks the total supply: the list
// policies resolve to their supply-tracking combination, and the
// self-tracking contract types to themselves.
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
impl WithSupply for Capped {
    type Out = Capped;
}
impl WithSupply for CappedAllowList {
    type Out = CappedAllowList;
}
impl WithSupply for CappedBlockList {
    type Out = CappedBlockList;
}
impl WithSupply for CappedAllowBlockList {
    type Out = CappedAllowBlockList;
}
impl WithSupply for CappedRWA {
    type Out = CappedRWA;
}
impl WithSupply for CappedVault {
    type Out = CappedVault;
}

impl Finalize for Nil {
    type Out = Base;
}
// A list that ends with the `TotalSupply` requirement open, e.g. `(RWA,)`.
// This row never succeeds; it only selects the error message (refer to
// `SupplyRequirementMet`).
impl<X> Finalize for NeedsSupply<X>
where
    NeedsSupply<X>: SupplyRequirementMet,
{
    type Out = Base;
}
// A list that ends with the requirement met resolves to the supply-tracking
// contract type of its contribution.
impl<X: WithSupply> Finalize for HasSupply<X> {
    type Out = <X as WithSupply>::Out;
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

// Bare forms: a single entry may also be written without the one-element
// tuple, so a stray trailing comma (or its absence) does not change the
// meaning. `RWA`, `Vault` and `Capped` have no bare form: they need
// `TotalSupply` in the list, so `Compose<RWA>` is rejected (with the generic
// `Composable` error).
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
