extern crate std;

use core::any::TypeId;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, String};
use stellar_governance::votes::get_voting_units;

use crate::non_fungible::{
    extensions::{
        burnable::Burnable,
        combinations::{Composable, Compose},
        consecutive::Consecutive,
        enumerable::Enumerable,
        royalties::Royalties,
        votes::NonFungibleVotes,
    },
    overrides::{BurnableOverrides, ContractOverrides},
    Base,
};

type EnumerableVotes = Compose<(Enumerable, NonFungibleVotes)>;
// deliberately the swapped order, asserting that the list is
// order-insensitive
type ConsecutiveVotes = Compose<(NonFungibleVotes, Consecutive)>;

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

#[test]
fn votes_combinations_are_order_insensitive() {
    assert_eq!(
        TypeId::of::<Compose<(Enumerable, NonFungibleVotes)>>(),
        TypeId::of::<Compose<(NonFungibleVotes, Enumerable)>>(),
    );
    assert_eq!(
        TypeId::of::<Compose<(Consecutive, NonFungibleVotes)>>(),
        TypeId::of::<Compose<(NonFungibleVotes, Consecutive)>>(),
    );
    // the combined types are distinct from their members
    assert_ne!(TypeId::of::<EnumerableVotes>(), TypeId::of::<Enumerable>());
    assert_ne!(TypeId::of::<EnumerableVotes>(), TypeId::of::<NonFungibleVotes>());
    assert_ne!(TypeId::of::<ConsecutiveVotes>(), TypeId::of::<Consecutive>());
    // additive markers stay ignored around a curated pair
    assert_eq!(
        TypeId::of::<Compose<(Enumerable, Burnable, NonFungibleVotes, Royalties)>>(),
        TypeId::of::<EnumerableVotes>(),
    );
}

#[contract]
struct MockContract;

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        Base::set_metadata(
            &e,
            String::from_str(&e, "https://example.com/"),
            String::from_str(&e, "Test NFT"),
            String::from_str(&e, "TNFT"),
        );
    });

    (e, contract_address)
}

#[test]
fn enumerable_votes_tracks_enumeration_and_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        let token_id = EnumerableVotes::sequential_mint(&e, &alice);
        assert_eq!(Enumerable::total_supply(&e), 1);
        assert_eq!(Enumerable::get_owner_token_id(&e, &alice, 0), token_id);
        assert_eq!(get_voting_units(&e, &alice), 1);

        <EnumerableVotes as ContractOverrides>::transfer(&e, &alice, &bob, token_id);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(get_voting_units(&e, &bob), 1);
        assert_eq!(Enumerable::get_owner_token_id(&e, &bob, 0), token_id);

        <EnumerableVotes as BurnableOverrides>::burn(&e, &bob, token_id);
        assert_eq!(get_voting_units(&e, &bob), 0);
        assert_eq!(Enumerable::total_supply(&e), 0);
    });
}

#[test]
fn consecutive_votes_tracks_batches_and_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        let last_id = ConsecutiveVotes::batch_mint(&e, &alice, 5);
        // one checkpoint update covers the whole batch
        assert_eq!(get_voting_units(&e, &alice), 5);
        // ownership resolves for non-boundary tokens of the batch
        assert_eq!(Consecutive::owner_of(&e, last_id - 2), alice);

        <ConsecutiveVotes as ContractOverrides>::transfer(&e, &alice, &bob, last_id);
        assert_eq!(get_voting_units(&e, &alice), 4);
        assert_eq!(get_voting_units(&e, &bob), 1);
        assert_eq!(Consecutive::owner_of(&e, last_id), bob);

        <ConsecutiveVotes as BurnableOverrides>::burn(&e, &bob, last_id);
        assert_eq!(get_voting_units(&e, &bob), 0);
    });
}
