extern crate std;

use core::any::TypeId;

use crate::non_fungible::{
    extensions::{
        combinations::{Composable, Compose},
        consecutive::Consecutive,
        enumerable::Enumerable,
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
