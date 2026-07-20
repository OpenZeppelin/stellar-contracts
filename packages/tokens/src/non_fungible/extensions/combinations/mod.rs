//! # Contract Type Composition for Non-Fungible Token.
//!
//! A contract selects exactly one `ContractType` on its
//! [`crate::non_fungible::NonFungibleToken`] implementation, and that single
//! slot decides all overridable behavior. The contract type is selected
//! uniformly with [`Compose`], by listing the contract types that override
//! the `Base` behavior.
//!
//! Note that [`Compose`] only selects the `ContractType`; it says nothing
//! about extensions that add new functionality without overriding the base
//! behavior. Those (e.g. [`crate::non_fungible::burnable::NonFungibleBurnable`]
//! or [`crate::non_fungible::royalties::NonFungibleRoyalties`]) have no
//! contract type and are simply implemented alongside; there is no
//! `Compose<(Burnable,)>`.
//!
//! Usage:
//!
//! ```ignore
//! #[contractimpl(contracttrait)]
//! impl NonFungibleToken for MyToken {
//!     type ContractType = Compose<(Enumerable,)>;
//! }
//!
//! #[contractimpl(contracttrait)]
//! impl NonFungibleEnumerable for MyToken {}
//! ```
//!
//! No multi-type combinations are curated for non-fungible tokens yet; every
//! valid list currently holds a single contract type. Invalid lists do not
//! compile: `Enumerable` and `Consecutive` are mutually exclusive, and
//! implementing an extension trait the list does not back (e.g.
//! [`crate::non_fungible::enumerable::NonFungibleEnumerable`] without
//! `Enumerable` in the list) is rejected by that trait's bound.

use crate::non_fungible::{
    extensions::{consecutive::Consecutive, enumerable::Enumerable, votes::NonFungibleVotes},
    Base, ContractOverrides,
};

/// Resolves a list of contract types to the combined contract type, e.g.
/// `Compose<(Enumerable,)>`.
///
/// This is shorthand for `<L as Composable>::Out`; refer to [`Composable`] for
/// the valid lists.
pub type Compose<L> = <L as Composable>::Out;

/// Type-level lookup backing [`Compose`]: each valid contract type list
/// resolves to its contract type through the `Out` associated type.
///
/// Single contract types resolve to themselves (with or without the
/// one-element tuple form). Mutually exclusive contract types (e.g.
/// `Enumerable` and `Consecutive`) have no implementation, so combining them
/// does not compile.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid contract type combination",
    note = "valid single contract types: `Base`, `Enumerable`, `Consecutive`, `NonFungibleVotes`",
    note = "no multi-type combinations are curated for non-fungible tokens yet",
    note = "`Enumerable` and `Consecutive` are mutually exclusive"
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
impl Composable for Enumerable {
    type Out = Enumerable;
}
impl Composable for (Enumerable,) {
    type Out = Enumerable;
}
impl Composable for Consecutive {
    type Out = Consecutive;
}
impl Composable for (Consecutive,) {
    type Out = Consecutive;
}
impl Composable for NonFungibleVotes {
    type Out = NonFungibleVotes;
}
impl Composable for (NonFungibleVotes,) {
    type Out = NonFungibleVotes;
}
