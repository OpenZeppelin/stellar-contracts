extern crate std;

use core::any::TypeId;

use crate::non_fungible::{
    extensions::{
        burnable::Burnable,
        combinations::{Composable, Compose},
        consecutive::Consecutive,
        enumerable::Enumerable,
        royalties::Royalties,
        votes::NonFungibleVotes,
    },
    Base,
};

// Pins every `Composable` mapping: the resolved `Out` type must be exactly the
// expected contract type. Invalid lists are rejected at compile time and
// cannot be covered here without a compile-fail harness.
fn assert_composes_to<List, Expected>()
where
    List: Composable,
    Compose<List>: 'static,
    Expected: 'static,
{
    assert_eq!(TypeId::of::<Compose<List>>(), TypeId::of::<Expected>());
}

#[test]
fn bare_forms_resolve_to_themselves() {
    assert_composes_to::<Base, Base>();
    assert_composes_to::<Enumerable, Enumerable>();
    assert_composes_to::<Consecutive, Consecutive>();
    assert_composes_to::<NonFungibleVotes, NonFungibleVotes>();
}

#[test]
fn tuple_forms_resolve_to_themselves() {
    assert_composes_to::<(Base,), Base>();
    assert_composes_to::<(Enumerable,), Enumerable>();
    assert_composes_to::<(Consecutive,), Consecutive>();
    assert_composes_to::<(NonFungibleVotes,), NonFungibleVotes>();
}

#[test]
fn additive_extensions_are_ignored() {
    // Additive markers may appear in any position without changing the
    // resolution.
    assert_composes_to::<(Enumerable, Burnable), Enumerable>();
    assert_composes_to::<(Burnable, Consecutive), Consecutive>();
    assert_composes_to::<(Base, Royalties), Base>();
    // Higher arities, covering every tuple impl.
    assert_composes_to::<(Enumerable, Burnable, Royalties), Enumerable>();
    assert_composes_to::<(Royalties, NonFungibleVotes, Burnable, Royalties), NonFungibleVotes>();
    assert_composes_to::<(Burnable, Royalties, Consecutive, Burnable, Royalties), Consecutive>();
}

#[test]
fn additive_only_lists_resolve_to_base() {
    assert_composes_to::<Burnable, Base>();
    assert_composes_to::<Royalties, Base>();
    assert_composes_to::<(Burnable, Royalties), Base>();
}
