extern crate std;

use core::any::TypeId;

use crate::{
    fungible::{
        extensions::{
            allowlist::AllowList,
            blocklist::BlockList,
            burnable::Burnable,
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

#[test]
fn additive_extensions_are_ignored() {
    // Additive markers may appear in any position without changing the
    // resolution.
    assert_composes_to::<(AllowList, Burnable), AllowList>();
    assert_composes_to::<(Burnable, AllowList), AllowList>();
    assert_composes_to::<(Vault, Burnable), Vault>();
    assert_composes_to::<(RWA, Burnable), RWA>();
    // Higher arities, covering every tuple impl.
    assert_composes_to::<(Burnable, FungibleVotes, Burnable), FungibleVotes>();
    assert_composes_to::<(BlockList, Burnable, Burnable, Burnable), BlockList>();
    assert_composes_to::<(Burnable, Burnable, Base, Burnable, Burnable), Base>();
}

#[test]
fn additive_only_lists_resolve_to_base() {
    assert_composes_to::<Burnable, Base>();
    assert_composes_to::<(Burnable,), Base>();
    assert_composes_to::<(Burnable, Burnable), Base>();
}
