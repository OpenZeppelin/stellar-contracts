extern crate std;

use core::any::TypeId;

use crate::{
    fungible::{
        extensions::{
            allowlist::AllowList,
            blocklist::BlockList,
            combinations::{Composable, Compose},
            votes::FungibleVotes,
        },
        Base,
    },
    rwa::RWA,
    vault::Vault,
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
    assert_composes_to::<AllowList, AllowList>();
    assert_composes_to::<BlockList, BlockList>();
    assert_composes_to::<RWA, RWA>();
    assert_composes_to::<Vault, Vault>();
    assert_composes_to::<FungibleVotes, FungibleVotes>();
}

#[test]
fn tuple_forms_resolve_to_themselves() {
    assert_composes_to::<(Base,), Base>();
    assert_composes_to::<(AllowList,), AllowList>();
    assert_composes_to::<(BlockList,), BlockList>();
    assert_composes_to::<(RWA,), RWA>();
    assert_composes_to::<(Vault,), Vault>();
    assert_composes_to::<(FungibleVotes,), FungibleVotes>();
}
