//! # Contract Type Composition for Non-Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::non_fungible::NonFungibleToken`] implementation, and that single
//! slot decides all overridable behavior. The contract type is selected
//! uniformly with [`Compose`], by listing the extensions the token is made
//! of.
//!
//! Additive extensions (e.g. [`Burnable`] for
//! [`crate::non_fungible::burnable::NonFungibleBurnable`], or [`Royalties`]
//! for [`crate::non_fungible::royalties::NonFungibleRoyalties`]) may be
//! listed as well: they do not affect the resolved contract type and are
//! enabled by implementing their trait, whether or not they are listed. A
//! list holding only additive extensions resolves to [`Base`].
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl NonFungibleToken for MyToken {
//!     // resolves to `Enumerable`; `Burnable` is declarative
//!     type ContractType = Compose<(Enumerable, Burnable)>;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl NonFungibleEnumerable for MyToken {}
//!
//! #[contractimpl(contracttrait)]
//! impl NonFungibleBurnable for MyToken {}
//! ```
//!
//! No multi-type combinations of contract types are curated for non-fungible
//! tokens yet; every valid list currently holds at most one contract type
//! (plus any additive extensions). Invalid lists do not compile: `Enumerable`
//! and `Consecutive` are mutually exclusive, and implementing an extension
//! trait the list does not back (e.g.
//! [`crate::non_fungible::enumerable::NonFungibleEnumerable`] without
//! `Enumerable` in the list) is rejected by that trait's bound.

#[cfg(test)]
mod test;

use crate::non_fungible::{
    extensions::{
        burnable::Burnable, consecutive::Consecutive, enumerable::Enumerable, royalties::Royalties,
        votes::NonFungibleVotes,
    },
    Base, ContractOverrides,
};

/// Resolves a list of contract types and additive extensions to the combined
/// contract type, e.g. `Compose<(Enumerable,)>` or
/// `Compose<(Enumerable, Burnable)>` (both resolve to `Enumerable`).
///
/// This is shorthand for `<L as Composable>::Out`; refer to [`Composable`] for
/// the valid lists.
pub type Compose<L> = <L as Composable>::Out;

/// Type-level lookup backing [`Compose`]: each valid list resolves to its
/// contract type through the `Out` associated type.
///
/// A list is folded pairwise, left to right. Single contract types resolve to
/// themselves (with or without the one-element tuple form), and additive
/// extensions are ignored. Mutually exclusive contract types (e.g.
/// `Enumerable` and `Consecutive`) have no pairwise combination, so listing
/// them together does not compile.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid contract type combination",
    note = "valid single contract types: `Base`, `Enumerable`, `Consecutive`, `NonFungibleVotes`",
    note = "additive extensions (e.g. `Burnable`, `Royalties`) may be listed alongside a contract \
            type; they do not affect the resolved type",
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
        note = "valid entries are the contract types (`Base`, `Enumerable`, `Consecutive`, \
                `NonFungibleVotes`) and the additive extensions (`Burnable`, `Royalties`)"
    )]
    pub trait Extension {
        type Contribution;
    }

    /// Pairwise combination table for contributions: the fold reduces a list
    /// left to right through this table. A missing row means the pair is
    /// invalid (mutually exclusive, or no curated combination exists).
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot be combined with `{B}` in a `Compose` list",
        note = "the contract types are mutually exclusive, or no combination of them is curated"
    )]
    pub trait Combine<B> {
        type Out;
    }

    /// Final step of the fold: maps the folded contribution to the resolved
    /// contract type. Its only non-trivial rule is [`Nil`] resolving to
    /// `Base` (a list of only additive extensions is a vanilla token).
    pub trait Finalize {
        type Out: super::ContractOverrides;
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
impl Extension for Enumerable {
    type Contribution = Enumerable;
}
impl Extension for Consecutive {
    type Contribution = Consecutive;
}
impl Extension for NonFungibleVotes {
    type Contribution = NonFungibleVotes;
}
// Additive extensions contribute nothing.
impl Extension for Burnable {
    type Contribution = Nil;
}
impl Extension for Royalties {
    type Contribution = Nil;
}

// Identity rows: combining with `Nil` changes nothing. One pair of rows per
// contract type, plus the `Nil`/`Nil` row for lists of only additive
// extensions. Curated multi-type combinations would be added here as
// additional rows (in both orders, so lists stay order-insensitive).
impl Combine<Nil> for Nil {
    type Out = Nil;
}
impl Combine<Nil> for Base {
    type Out = Base;
}
impl Combine<Base> for Nil {
    type Out = Base;
}
impl Combine<Nil> for Enumerable {
    type Out = Enumerable;
}
impl Combine<Enumerable> for Nil {
    type Out = Enumerable;
}
impl Combine<Nil> for Consecutive {
    type Out = Consecutive;
}
impl Combine<Consecutive> for Nil {
    type Out = Consecutive;
}
impl Combine<Nil> for NonFungibleVotes {
    type Out = NonFungibleVotes;
}
impl Combine<NonFungibleVotes> for Nil {
    type Out = NonFungibleVotes;
}

impl Finalize for Nil {
    type Out = Base;
}
impl Finalize for Base {
    type Out = Base;
}
impl Finalize for Enumerable {
    type Out = Enumerable;
}
impl Finalize for Consecutive {
    type Out = Consecutive;
}
impl Finalize for NonFungibleVotes {
    type Out = NonFungibleVotes;
}

// Bare forms: a single entry may also be written without the one-element
// tuple, so a stray trailing comma (or its absence) does not change the
// meaning.
impl Composable for Base {
    type Out = Base;
}
impl Composable for Enumerable {
    type Out = Enumerable;
}
impl Composable for Consecutive {
    type Out = Consecutive;
}
impl Composable for NonFungibleVotes {
    type Out = NonFungibleVotes;
}
impl Composable for Burnable {
    type Out = Base;
}
impl Composable for Royalties {
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
