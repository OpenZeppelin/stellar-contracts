extern crate std;

use core::any::TypeId;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, MuxedAddress};

use crate::{
    fungible::{
        extensions::{
            allowlist::AllowList,
            blocklist::BlockList,
            burnable::Burnable,
            combinations::{Composable, Compose},
            total_supply::{mint, total_supply, TotalSupply},
            votes::FungibleVotes,
        },
        overrides::BurnableOverrides,
        Base, ContractOverrides,
    },
    rwa::RWA,
    vault::Vault,
};

type AllowListWithSupply = Compose<(AllowList, TotalSupply)>;
// deliberately the swapped order, asserting that the list is
// order-insensitive
type BlockListWithSupply = Compose<(TotalSupply, BlockList)>;

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
    assert_composes_to::<TotalSupply, TotalSupply>();
    assert_composes_to::<RWA, RWA>();
    assert_composes_to::<Vault, Vault>();
    assert_composes_to::<FungibleVotes, FungibleVotes>();
}

#[test]
fn tuple_forms_resolve_to_themselves() {
    assert_composes_to::<(Base,), Base>();
    assert_composes_to::<(AllowList,), AllowList>();
    assert_composes_to::<(BlockList,), BlockList>();
    assert_composes_to::<(TotalSupply,), TotalSupply>();
    assert_composes_to::<(RWA,), RWA>();
    assert_composes_to::<(Vault,), Vault>();
    assert_composes_to::<(FungibleVotes,), FungibleVotes>();
}

#[test]
fn pair_lists_are_order_insensitive() {
    assert_eq!(
        TypeId::of::<Compose<(AllowList, TotalSupply)>>(),
        TypeId::of::<Compose<(TotalSupply, AllowList)>>(),
    );
    assert_eq!(
        TypeId::of::<Compose<(BlockList, TotalSupply)>>(),
        TypeId::of::<Compose<(TotalSupply, BlockList)>>(),
    );
}

#[contract]
struct MockContract;

#[test]
fn allowlist_burn_decreases_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        AllowList::allow_user(&e, &account);
        <AllowListWithSupply as BurnableOverrides>::burn(&e, &account, 40);
        assert_eq!(Base::balance(&e, &account), 60);
        assert_eq!(total_supply(&e), 60);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn allowlist_burn_respects_policy() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        // `account` is not allowed, the allowlist policy has to reject the
        // burn
        <AllowListWithSupply as BurnableOverrides>::burn(&e, &account, 40);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn allowlist_transfer_respects_policy() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &from, 100);
        // neither account is allowed, the allowlist policy has to reject the
        // transfer
        <AllowListWithSupply as ContractOverrides>::transfer(
            &e,
            &from,
            &MuxedAddress::from(recipient),
            30,
        );
    });
}

#[test]
fn blocklist_burn_from_decreases_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 40, e.ledger().sequence() + 100);
        <BlockListWithSupply as BurnableOverrides>::burn_from(&e, &spender, &owner, 40);
        assert_eq!(Base::balance(&e, &owner), 60);
        assert_eq!(total_supply(&e), 60);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn blocklist_burn_respects_policy() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        BlockList::block_user(&e, &account);
        // `account` is blocked, the blocklist policy has to reject the burn
        <BlockListWithSupply as BurnableOverrides>::burn(&e, &account, 40);
    });
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
    // Curated pairs are not disturbed by additive markers either.
    assert_composes_to::<(AllowList, TotalSupply, Burnable), AllowListWithSupply>();
    assert_composes_to::<(Burnable, TotalSupply, BlockList), BlockListWithSupply>();
}

#[test]
fn additive_only_lists_resolve_to_base() {
    assert_composes_to::<Burnable, Base>();
    assert_composes_to::<(Burnable,), Base>();
    assert_composes_to::<(Burnable, Burnable), Base>();
}
