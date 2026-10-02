extern crate std;

use core::any::TypeId;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, MuxedAddress};
use stellar_governance::votes::get_voting_units;

// The capped combinations are named directly (not through `Compose`), so the
// resolution tests below compare against the real types.
use super::{CappedAllowBlockList, CappedAllowList, CappedBlockList};
use crate::{
    fungible::{
        extensions::{
            allowlist::{AllowList, AllowListContractType},
            blocklist::{BlockList, BlockListContractType},
            burnable::Burnable,
            capped::{Capped, CappedContractType},
            combinations::{Composable, Compose},
            total_supply::{mint, total_supply, TotalSupply},
            votes::FungibleVotes,
        },
        overrides::{BurnableOverrides, MintOverrides, TotalSupplyOverrides},
        Base, ContractOverrides,
    },
    rwa::{CappedRWA, RWA},
    vault::{CappedVault, Vault},
};

// deliberately the swapped orders, asserting that the lists are
// order-insensitive
type AllowBlockList = Compose<(BlockList, AllowList)>;
type AllowListVotes = Compose<(FungibleVotes, AllowList)>;
type BlockListVotes = Compose<(BlockList, FungibleVotes)>;
type AllowBlockListVotes = Compose<(FungibleVotes, BlockList, AllowList)>;
type TotalSupplyAllowList = Compose<(AllowList, TotalSupply)>;
type TotalSupplyBlockList = Compose<(TotalSupply, BlockList)>;
type TotalSupplyAllowBlockList = Compose<(BlockList, TotalSupply, AllowList)>;

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
    assert_composes_to::<FungibleVotes, FungibleVotes>();
}

#[test]
fn tuple_forms_resolve_to_themselves() {
    assert_composes_to::<(Base,), Base>();
    assert_composes_to::<(AllowList,), AllowList>();
    assert_composes_to::<(BlockList,), BlockList>();
    assert_composes_to::<(TotalSupply,), TotalSupply>();
    assert_composes_to::<(FungibleVotes,), FungibleVotes>();
}

#[test]
fn additive_extensions_are_ignored() {
    // Additive markers may appear in any position without changing the
    // resolution.
    assert_composes_to::<(AllowList, Burnable), AllowList>();
    assert_composes_to::<(Burnable, AllowList), AllowList>();
    assert_composes_to::<(Vault, TotalSupply, Burnable), Vault>();
    assert_composes_to::<(RWA, Burnable, TotalSupply), RWA>();
    // Higher arities, covering every tuple impl.
    assert_composes_to::<(Burnable, FungibleVotes, Burnable), FungibleVotes>();
    assert_composes_to::<(BlockList, Burnable, Burnable, Burnable), BlockList>();
    assert_composes_to::<(Burnable, Burnable, Base, Burnable, Burnable), Base>();
    // Curated pairs are not disturbed by additive markers either.
    assert_composes_to::<(AllowList, TotalSupply, Burnable), TotalSupplyAllowList>();
    assert_composes_to::<(Burnable, TotalSupply, BlockList), TotalSupplyBlockList>();
}

#[test]
fn additive_only_lists_resolve_to_base() {
    assert_composes_to::<Burnable, Base>();
    assert_composes_to::<(Burnable,), Base>();
    assert_composes_to::<(Burnable, Burnable), Base>();
}

// `Capped` is a contract type of its own and, like `RWA` and `Vault`, requires
// `TotalSupply` in the list. Every capped list resolves to its capped contract
// type in every order. Lists missing `TotalSupply` are rejected at compile
// time and cannot be covered here.
#[test]
fn capped_lists_resolve_to_the_capped_contract_types() {
    assert_composes_to::<(Capped, TotalSupply), Capped>();
    assert_composes_to::<(TotalSupply, Capped), Capped>();
    assert_composes_to::<(Burnable, Capped, TotalSupply), Capped>();
    assert_composes_to::<(Capped, Burnable, TotalSupply, Burnable), Capped>();
    assert_composes_to::<(AllowList, Capped, TotalSupply), CappedAllowList>();
    assert_composes_to::<(AllowList, TotalSupply, Capped), CappedAllowList>();
    assert_composes_to::<(Capped, AllowList, TotalSupply), CappedAllowList>();
    assert_composes_to::<(Capped, TotalSupply, AllowList), CappedAllowList>();
    assert_composes_to::<(TotalSupply, AllowList, Capped), CappedAllowList>();
    assert_composes_to::<(TotalSupply, Capped, AllowList), CappedAllowList>();
    assert_composes_to::<(BlockList, Capped, TotalSupply), CappedBlockList>();
    assert_composes_to::<(BlockList, TotalSupply, Capped), CappedBlockList>();
    assert_composes_to::<(Capped, BlockList, TotalSupply), CappedBlockList>();
    assert_composes_to::<(Capped, TotalSupply, BlockList), CappedBlockList>();
    assert_composes_to::<(TotalSupply, BlockList, Capped), CappedBlockList>();
    assert_composes_to::<(TotalSupply, Capped, BlockList), CappedBlockList>();
    assert_composes_to::<(AllowList, BlockList, Capped, TotalSupply), CappedAllowBlockList>();
    assert_composes_to::<(AllowList, BlockList, TotalSupply, Capped), CappedAllowBlockList>();
    assert_composes_to::<(AllowList, Capped, BlockList, TotalSupply), CappedAllowBlockList>();
    assert_composes_to::<(AllowList, Capped, TotalSupply, BlockList), CappedAllowBlockList>();
    assert_composes_to::<(AllowList, TotalSupply, BlockList, Capped), CappedAllowBlockList>();
    assert_composes_to::<(AllowList, TotalSupply, Capped, BlockList), CappedAllowBlockList>();
    assert_composes_to::<(BlockList, AllowList, Capped, TotalSupply), CappedAllowBlockList>();
    assert_composes_to::<(BlockList, AllowList, TotalSupply, Capped), CappedAllowBlockList>();
    assert_composes_to::<(BlockList, Capped, AllowList, TotalSupply), CappedAllowBlockList>();
    assert_composes_to::<(BlockList, Capped, TotalSupply, AllowList), CappedAllowBlockList>();
    assert_composes_to::<(BlockList, TotalSupply, AllowList, Capped), CappedAllowBlockList>();
    assert_composes_to::<(BlockList, TotalSupply, Capped, AllowList), CappedAllowBlockList>();
    assert_composes_to::<(Capped, AllowList, BlockList, TotalSupply), CappedAllowBlockList>();
    assert_composes_to::<(Capped, AllowList, TotalSupply, BlockList), CappedAllowBlockList>();
    assert_composes_to::<(Capped, BlockList, AllowList, TotalSupply), CappedAllowBlockList>();
    assert_composes_to::<(Capped, BlockList, TotalSupply, AllowList), CappedAllowBlockList>();
    assert_composes_to::<(Capped, TotalSupply, AllowList, BlockList), CappedAllowBlockList>();
    assert_composes_to::<(Capped, TotalSupply, BlockList, AllowList), CappedAllowBlockList>();
    assert_composes_to::<(TotalSupply, AllowList, BlockList, Capped), CappedAllowBlockList>();
    assert_composes_to::<(TotalSupply, AllowList, Capped, BlockList), CappedAllowBlockList>();
    assert_composes_to::<(TotalSupply, BlockList, AllowList, Capped), CappedAllowBlockList>();
    assert_composes_to::<(TotalSupply, BlockList, Capped, AllowList), CappedAllowBlockList>();
    assert_composes_to::<(TotalSupply, Capped, AllowList, BlockList), CappedAllowBlockList>();
    assert_composes_to::<(TotalSupply, Capped, BlockList, AllowList), CappedAllowBlockList>();
    assert_composes_to::<(RWA, Capped, TotalSupply), CappedRWA>();
    assert_composes_to::<(RWA, TotalSupply, Capped), CappedRWA>();
    assert_composes_to::<(Capped, RWA, TotalSupply), CappedRWA>();
    assert_composes_to::<(Capped, TotalSupply, RWA), CappedRWA>();
    assert_composes_to::<(TotalSupply, RWA, Capped), CappedRWA>();
    assert_composes_to::<(TotalSupply, Capped, RWA), CappedRWA>();
    assert_composes_to::<(Vault, Capped, TotalSupply), CappedVault>();
    assert_composes_to::<(Vault, TotalSupply, Capped), CappedVault>();
    assert_composes_to::<(Capped, Vault, TotalSupply), CappedVault>();
    assert_composes_to::<(Capped, TotalSupply, Vault), CappedVault>();
    assert_composes_to::<(TotalSupply, Vault, Capped), CappedVault>();
    assert_composes_to::<(TotalSupply, Capped, Vault), CappedVault>();
    assert_composes_to::<(Vault, Burnable, Capped, TotalSupply), CappedVault>();
}

#[test]
fn capped_contract_types_are_distinct() {
    assert_ne!(TypeId::of::<Capped>(), TypeId::of::<TotalSupply>());
    assert_ne!(TypeId::of::<CappedAllowList>(), TypeId::of::<TotalSupplyAllowList>());
    assert_ne!(TypeId::of::<CappedBlockList>(), TypeId::of::<TotalSupplyBlockList>());
    assert_ne!(TypeId::of::<CappedAllowBlockList>(), TypeId::of::<TotalSupplyAllowBlockList>());
    assert_ne!(TypeId::of::<CappedRWA>(), TypeId::of::<RWA>());
    assert_ne!(TypeId::of::<CappedVault>(), TypeId::of::<Vault>());
}

// The capped contract types have to back every extension their members back:
// `FungibleCapped`, `FungibleTotalSupply`, and the list extensions. A missing
// impl fails here at compile time instead of at some downstream contract's
// build.
#[test]
fn capped_contract_types_back_their_extensions() {
    fn assert_capped<T: CappedContractType + TotalSupplyOverrides>() {}
    fn assert_allowlist<T: AllowListContractType>() {}
    fn assert_blocklist<T: BlockListContractType>() {}
    assert_capped::<Capped>();
    assert_capped::<CappedAllowList>();
    assert_capped::<CappedBlockList>();
    assert_capped::<CappedAllowBlockList>();
    assert_capped::<CappedRWA>();
    assert_capped::<CappedVault>();
    assert_allowlist::<CappedAllowList>();
    assert_allowlist::<CappedAllowBlockList>();
    assert_blocklist::<CappedBlockList>();
    assert_blocklist::<CappedAllowBlockList>();
}

// Every contract type that can mint freely has to implement `MintOverrides`,
// so that `Self::ContractType::mint` resolves for every valid list without
// `Vault`.
#[test]
fn free_minting_contract_types_implement_mint_overrides() {
    fn assert_mint<T: MintOverrides>() {}
    assert_mint::<Base>();
    assert_mint::<AllowList>();
    assert_mint::<BlockList>();
    assert_mint::<AllowBlockList>();
    assert_mint::<TotalSupply>();
    assert_mint::<TotalSupplyAllowList>();
    assert_mint::<TotalSupplyBlockList>();
    assert_mint::<TotalSupplyAllowBlockList>();
    assert_mint::<FungibleVotes>();
    assert_mint::<AllowListVotes>();
    assert_mint::<BlockListVotes>();
    assert_mint::<AllowBlockListVotes>();
    assert_mint::<RWA>();
    assert_mint::<Capped>();
    assert_mint::<CappedAllowList>();
    assert_mint::<CappedBlockList>();
    assert_mint::<CappedAllowBlockList>();
    assert_mint::<CappedRWA>();
}

#[test]
fn list_combination_is_order_insensitive() {
    assert_eq!(
        TypeId::of::<Compose<(AllowList, BlockList)>>(),
        TypeId::of::<Compose<(BlockList, AllowList)>>(),
    );
    // the combined type is distinct from its members
    assert_ne!(TypeId::of::<AllowBlockList>(), TypeId::of::<AllowList>());
    assert_ne!(TypeId::of::<AllowBlockList>(), TypeId::of::<BlockList>());
    // additive markers stay ignored around a curated pair
    assert_composes_to::<(AllowList, Burnable, BlockList), AllowBlockList>();
    assert_composes_to::<(Burnable, BlockList, AllowList, Burnable), AllowBlockList>();
}

#[test]
fn votes_combinations_are_order_insensitive() {
    assert_eq!(
        TypeId::of::<Compose<(AllowList, FungibleVotes)>>(),
        TypeId::of::<Compose<(FungibleVotes, AllowList)>>(),
    );
    assert_eq!(
        TypeId::of::<Compose<(BlockList, FungibleVotes)>>(),
        TypeId::of::<Compose<(FungibleVotes, BlockList)>>(),
    );
    // every ordering of the triple resolves to the same type
    assert_composes_to::<(AllowList, BlockList, FungibleVotes), AllowBlockListVotes>();
    assert_composes_to::<(AllowList, FungibleVotes, BlockList), AllowBlockListVotes>();
    assert_composes_to::<(BlockList, AllowList, FungibleVotes), AllowBlockListVotes>();
    assert_composes_to::<(BlockList, FungibleVotes, AllowList), AllowBlockListVotes>();
    assert_composes_to::<(FungibleVotes, AllowList, BlockList), AllowBlockListVotes>();
    // the combined types are distinct from their members and from each other
    assert_ne!(TypeId::of::<AllowListVotes>(), TypeId::of::<AllowList>());
    assert_ne!(TypeId::of::<AllowListVotes>(), TypeId::of::<FungibleVotes>());
    assert_ne!(TypeId::of::<BlockListVotes>(), TypeId::of::<BlockList>());
    assert_ne!(TypeId::of::<AllowBlockListVotes>(), TypeId::of::<AllowBlockList>());
    assert_ne!(TypeId::of::<AllowBlockListVotes>(), TypeId::of::<AllowListVotes>());
    // additive markers stay ignored around curated combinations
    assert_composes_to::<(Burnable, AllowList, FungibleVotes), AllowListVotes>();
    assert_composes_to::<
        (AllowList, Burnable, BlockList, FungibleVotes, Burnable),
        AllowBlockListVotes,
    >();
}

// The combined contract types have to back their list extensions, so that a
// contract selecting one of them can implement `FungibleAllowList` and/or
// `FungibleBlockList` alongside. A missing impl fails here at compile time
// instead of at some downstream contract's build.
#[test]
fn combinations_back_their_list_extensions() {
    fn assert_allowlist<T: AllowListContractType>() {}
    fn assert_blocklist<T: BlockListContractType>() {}
    assert_allowlist::<AllowList>();
    assert_allowlist::<AllowBlockList>();
    assert_allowlist::<AllowListVotes>();
    assert_allowlist::<AllowBlockListVotes>();
    assert_blocklist::<BlockList>();
    assert_blocklist::<AllowBlockList>();
    assert_blocklist::<BlockListVotes>();
    assert_blocklist::<AllowBlockListVotes>();
    assert_allowlist::<TotalSupplyAllowList>();
    assert_allowlist::<TotalSupplyAllowBlockList>();
    assert_blocklist::<TotalSupplyBlockList>();
    assert_blocklist::<TotalSupplyAllowBlockList>();
}

#[test]
fn total_supply_combinations_are_order_insensitive() {
    assert_eq!(
        TypeId::of::<Compose<(AllowList, TotalSupply)>>(),
        TypeId::of::<Compose<(TotalSupply, AllowList)>>(),
    );
    assert_eq!(
        TypeId::of::<Compose<(BlockList, TotalSupply)>>(),
        TypeId::of::<Compose<(TotalSupply, BlockList)>>(),
    );
    // every ordering of the triple resolves to the same type
    assert_composes_to::<(AllowList, BlockList, TotalSupply), TotalSupplyAllowBlockList>();
    assert_composes_to::<(AllowList, TotalSupply, BlockList), TotalSupplyAllowBlockList>();
    assert_composes_to::<(BlockList, AllowList, TotalSupply), TotalSupplyAllowBlockList>();
    assert_composes_to::<(TotalSupply, AllowList, BlockList), TotalSupplyAllowBlockList>();
    assert_composes_to::<(TotalSupply, BlockList, AllowList), TotalSupplyAllowBlockList>();
    // the combined types are distinct from their members and from each other
    assert_ne!(TypeId::of::<TotalSupplyAllowList>(), TypeId::of::<AllowList>());
    assert_ne!(TypeId::of::<TotalSupplyAllowList>(), TypeId::of::<TotalSupply>());
    assert_ne!(TypeId::of::<TotalSupplyAllowBlockList>(), TypeId::of::<AllowBlockList>());
    assert_ne!(TypeId::of::<TotalSupplyAllowBlockList>(), TypeId::of::<TotalSupplyAllowList>());
}

// `RWA` and `Vault` require `TotalSupply` in the list and resolve to
// themselves once it is there, in any position. Lists missing it are rejected
// at compile time and cannot be covered here.
#[test]
fn rwa_and_vault_require_total_supply_in_the_list() {
    assert_composes_to::<(RWA, TotalSupply), RWA>();
    assert_composes_to::<(TotalSupply, RWA), RWA>();
    assert_composes_to::<(Burnable, RWA, Burnable, TotalSupply), RWA>();
    assert_composes_to::<(Vault, TotalSupply), Vault>();
    assert_composes_to::<(TotalSupply, Vault), Vault>();
    assert_composes_to::<(TotalSupply, Burnable, Vault), Vault>();
}

// Every supply-aware contract type has to back `FungibleTotalSupply`. A
// missing impl fails here at compile time instead of at some downstream
// contract's build.
#[test]
fn supply_aware_contract_types_back_total_supply() {
    fn assert_supply<T: TotalSupplyOverrides>() {}
    assert_supply::<TotalSupply>();
    assert_supply::<TotalSupplyAllowList>();
    assert_supply::<TotalSupplyBlockList>();
    assert_supply::<TotalSupplyAllowBlockList>();
    assert_supply::<RWA>();
    assert_supply::<Vault>();
    assert_supply::<Capped>();
    assert_supply::<CappedAllowList>();
    assert_supply::<CappedBlockList>();
    assert_supply::<CappedAllowBlockList>();
    assert_supply::<CappedRWA>();
    assert_supply::<CappedVault>();
}

#[contract]
struct MockContract;

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_address = e.register(MockContract, ());
    (e, contract_address)
}

#[test]
fn list_combination_applies_both_policies() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        Base::mint(&e, &alice, 100);
    });

    // One frame per authorized call: the same address cannot `require_auth`
    // twice within a single invocation.
    e.as_contract(&address, || {
        <AllowBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <AllowBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <AllowBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <AllowBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });
    e.as_contract(&address, || {
        <AllowBlockList as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 30);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(Base::allowance(&e, &alice, &spender), 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn list_combination_transfer_rejects_not_allowed_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        Base::mint(&e, &alice, 100);
        // `bob` is not allowed
        <AllowBlockList as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn list_combination_transfer_rejects_blocked_sender() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        Base::mint(&e, &alice, 100);
        // allowed, but blocked
        BlockList::block_user(&e, &alice);
        <AllowBlockList as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn list_combination_checks_blocklist_first() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        Base::mint(&e, &alice, 100);
        // `alice` is both blocked and not allowed: the blocklist error wins
        BlockList::block_user(&e, &alice);
        <AllowBlockList as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn list_combination_transfer_from_rejects_blocked_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        Base::mint(&e, &alice, 100);
        Base::approve(&e, &alice, &spender, 50, 1000);
        BlockList::block_user(&e, &bob);
        <AllowBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn list_combination_transfer_from_rejects_not_allowed_sender() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &bob);
        Base::mint(&e, &alice, 100);
        Base::approve(&e, &alice, &spender, 50, 1000);
        // `alice` is not allowed
        <AllowBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn list_combination_approve_rejects_not_allowed_owner() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        <AllowBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn list_combination_approve_rejects_blocked_owner() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        BlockList::block_user(&e, &alice);
        <AllowBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn list_combination_burn_rejects_blocked_account() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        Base::mint(&e, &alice, 100);
        BlockList::block_user(&e, &alice);
        <AllowBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn list_combination_burn_rejects_not_allowed_account() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Base::mint(&e, &alice, 100);
        <AllowBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn list_combination_burn_from_rejects_blocked_account() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        Base::mint(&e, &alice, 100);
        Base::approve(&e, &alice, &spender, 50, 1000);
        BlockList::block_user(&e, &alice);
        <AllowBlockList as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn list_combination_burn_from_rejects_not_allowed_account() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        Base::mint(&e, &alice, 100);
        Base::approve(&e, &alice, &spender, 50, 1000);
        <AllowBlockList as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });
}

#[test]
fn allow_list_votes_gates_transfers_and_moves_voting_units() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        AllowListVotes::mint(&e, &alice, 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
    e.as_contract(&address, || {
        <AllowListVotes as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <AllowListVotes as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <AllowListVotes as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <AllowListVotes as BurnableOverrides>::burn(&e, &alice, 10);
    });
    e.as_contract(&address, || {
        <AllowListVotes as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 30);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(get_voting_units(&e, &alice), 30);
        assert_eq!(get_voting_units(&e, &bob), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn allow_list_votes_rejects_not_allowed_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowListVotes::mint(&e, &alice, 100);
        // `bob` is not allowed
        <AllowListVotes as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn allow_list_votes_rejects_not_allowed_burner() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        AllowListVotes::mint(&e, &alice, 100);
        <AllowListVotes as BurnableOverrides>::burn(&e, &alice, 10);
    });
}

#[test]
fn block_list_votes_gates_transfers_and_moves_voting_units() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        BlockListVotes::mint(&e, &alice, 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
    e.as_contract(&address, || {
        <BlockListVotes as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <BlockListVotes as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <BlockListVotes as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <BlockListVotes as BurnableOverrides>::burn(&e, &alice, 10);
    });
    e.as_contract(&address, || {
        <BlockListVotes as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 30);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(Base::allowance(&e, &alice, &spender), 20);
        assert_eq!(get_voting_units(&e, &alice), 30);
        assert_eq!(get_voting_units(&e, &bob), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn block_list_votes_rejects_blocked_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        BlockListVotes::mint(&e, &alice, 100);
        BlockList::block_user(&e, &bob);
        <BlockListVotes as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
fn allow_block_list_votes_applies_both_policies_and_moves_voting_units() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        AllowBlockListVotes::mint(&e, &alice, 100);
        assert_eq!(get_voting_units(&e, &alice), 100);
    });
    e.as_contract(&address, || {
        <AllowBlockListVotes as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <AllowBlockListVotes as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <AllowBlockListVotes as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <AllowBlockListVotes as BurnableOverrides>::burn(&e, &alice, 10);
    });
    e.as_contract(&address, || {
        <AllowBlockListVotes as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 30);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(get_voting_units(&e, &alice), 30);
        assert_eq!(get_voting_units(&e, &bob), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn allow_block_list_votes_rejects_blocked_sender() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        AllowBlockListVotes::mint(&e, &alice, 100);
        BlockList::block_user(&e, &alice);
        <AllowBlockListVotes as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob),
            30,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn allow_block_list_votes_rejects_not_allowed_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowBlockListVotes::mint(&e, &alice, 100);
        // `bob` is not allowed
        <AllowBlockListVotes as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob),
            30,
        );
    });
}

#[test]
fn total_supply_allow_list_burn_decreases_supply() {
    let (e, address) = setup_env();
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &account, 100);
        AllowList::allow_user(&e, &account);
        <TotalSupplyAllowList as BurnableOverrides>::burn(&e, &account, 40);
        assert_eq!(Base::balance(&e, &account), 60);
        assert_eq!(TotalSupplyAllowList::total_supply(&e), 60);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn total_supply_allow_list_burn_respects_policy() {
    let (e, address) = setup_env();
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &account, 100);
        // `account` is not allowed, the allowlist policy has to reject the
        // burn
        <TotalSupplyAllowList as BurnableOverrides>::burn(&e, &account, 40);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn total_supply_allow_list_transfer_respects_policy() {
    let (e, address) = setup_env();
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 100);
        // neither account is allowed, the allowlist policy has to reject the
        // transfer
        <TotalSupplyAllowList as ContractOverrides>::transfer(
            &e,
            &from,
            &MuxedAddress::from(recipient),
            30,
        );
    });
}

#[test]
fn total_supply_block_list_applies_policy_and_tracks_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        TotalSupplyBlockList::mint(&e, &alice, 100);
        assert_eq!(TotalSupplyBlockList::total_supply(&e), 100);
    });
    e.as_contract(&address, || {
        <TotalSupplyBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <TotalSupplyBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <TotalSupplyBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <TotalSupplyBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 40);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(Base::allowance(&e, &alice, &spender), 30);
        assert_eq!(TotalSupplyBlockList::total_supply(&e), 90);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn total_supply_block_list_transfer_rejects_blocked_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        TotalSupplyBlockList::mint(&e, &alice, 100);
        BlockList::block_user(&e, &bob);
        <TotalSupplyBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob),
            30,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn total_supply_block_list_approve_rejects_blocked_owner() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        TotalSupplyBlockList::mint(&e, &alice, 100);
        BlockList::block_user(&e, &alice);
        <TotalSupplyBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn total_supply_block_list_transfer_from_rejects_blocked_sender() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        TotalSupplyBlockList::mint(&e, &alice, 100);
        Base::approve(&e, &alice, &spender, 50, 1000);
        BlockList::block_user(&e, &alice);
        <TotalSupplyBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
}

#[test]
fn total_supply_block_list_burn_from_decreases_supply() {
    let (e, address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 40, e.ledger().sequence() + 100);
        <TotalSupplyBlockList as BurnableOverrides>::burn_from(&e, &spender, &owner, 40);
        assert_eq!(Base::balance(&e, &owner), 60);
        assert_eq!(total_supply(&e), 60);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn total_supply_block_list_burn_respects_policy() {
    let (e, address) = setup_env();
    let account = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &account, 100);
        BlockList::block_user(&e, &account);
        // `account` is blocked, the blocklist policy has to reject the burn
        <TotalSupplyBlockList as BurnableOverrides>::burn(&e, &account, 40);
    });
}

#[test]
fn total_supply_allow_block_list_tracks_supply_under_both_policies() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        TotalSupplyAllowBlockList::mint(&e, &alice, 100);
        assert_eq!(TotalSupplyAllowBlockList::total_supply(&e), 100);
    });
    e.as_contract(&address, || {
        <TotalSupplyAllowBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <TotalSupplyAllowBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <TotalSupplyAllowBlockList as ContractOverrides>::transfer_from(
            &e, &spender, &alice, &bob, 20,
        );
    });
    e.as_contract(&address, || {
        <TotalSupplyAllowBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });
    e.as_contract(&address, || {
        <TotalSupplyAllowBlockList as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 30);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(Base::allowance(&e, &alice, &spender), 20);
        assert_eq!(TotalSupplyAllowBlockList::total_supply(&e), 80);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn total_supply_allow_block_list_burn_rejects_blocked_account() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        TotalSupplyAllowBlockList::mint(&e, &alice, 100);
        BlockList::block_user(&e, &alice);
        <TotalSupplyAllowBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn total_supply_allow_block_list_transfer_rejects_not_allowed_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        TotalSupplyAllowBlockList::mint(&e, &alice, 100);
        // `bob` is not allowed
        <TotalSupplyAllowBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob),
            30,
        );
    });
}

// ################## CAPPED COMBINATIONS ##################

// Mints through `MintOverrides`, the path `Self::ContractType::mint` takes on
// a contract, so the cap check of the capped contract type is exercised.
fn mint_through<T: MintOverrides>(e: &Env, to: &Address, amount: i128) {
    T::mint(e, to, amount);
}

#[test]
fn capped_combinations_mint_up_to_the_cap() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        mint_through::<Capped>(&e, &alice, 25);
        mint_through::<CappedAllowList>(&e, &alice, 25);
        mint_through::<CappedBlockList>(&e, &alice, 25);
        mint_through::<CappedAllowBlockList>(&e, &alice, 25);
        // every capped contract type reads the same supply counter and cap
        assert_eq!(total_supply(&e), 100);
        assert_eq!(<CappedAllowList as TotalSupplyOverrides>::total_supply(&e), 100);
        assert_eq!(Base::balance(&e, &alice), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn capped_allow_list_mint_rejects_exceeding_cap() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        mint_through::<CappedAllowList>(&e, &alice, 60);
        mint_through::<CappedAllowList>(&e, &alice, 41);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn capped_block_list_mint_rejects_exceeding_cap() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        mint_through::<CappedBlockList>(&e, &alice, 101);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn capped_allow_block_list_mint_rejects_exceeding_cap() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        // supply minted outside the capped type still counts against the cap
        mint(&e, &alice, 90);
        mint_through::<CappedAllowBlockList>(&e, &alice, 11);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #108)")]
fn capped_combination_mint_requires_a_cap() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        mint_through::<CappedAllowList>(&e, &alice, 1);
    });
}

#[test]
fn capped_allow_list_applies_policy_and_tracks_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        mint_through::<CappedAllowList>(&e, &alice, 100);
    });
    e.as_contract(&address, || {
        <CappedAllowList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <CappedAllowList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <CappedAllowList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <CappedAllowList as BurnableOverrides>::burn(&e, &alice, 10);
    });
    e.as_contract(&address, || {
        <CappedAllowList as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 30);
        assert_eq!(Base::balance(&e, &bob), 50);
        assert_eq!(total_supply(&e), 80);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn capped_allow_list_transfer_rejects_not_allowed_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        AllowList::allow_user(&e, &alice);
        mint_through::<CappedAllowList>(&e, &alice, 100);
        <CappedAllowList as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn capped_allow_list_burn_respects_policy() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        mint_through::<CappedAllowList>(&e, &alice, 100);
        // `alice` is not allowed
        <CappedAllowList as BurnableOverrides>::burn(&e, &alice, 10);
    });
}

#[test]
fn capped_block_list_applies_policy_and_tracks_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        mint_through::<CappedBlockList>(&e, &alice, 100);
    });
    e.as_contract(&address, || {
        <CappedBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob.clone()),
            30,
        );
    });
    e.as_contract(&address, || {
        <CappedBlockList as BurnableOverrides>::burn(&e, &alice, 20);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 50);
        assert_eq!(Base::balance(&e, &bob), 30);
        assert_eq!(total_supply(&e), 80);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn capped_block_list_transfer_rejects_blocked_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        mint_through::<CappedBlockList>(&e, &alice, 100);
        BlockList::block_user(&e, &bob);
        <CappedBlockList as ContractOverrides>::transfer(&e, &alice, &MuxedAddress::from(bob), 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn capped_block_list_approve_rejects_blocked_owner() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        BlockList::block_user(&e, &alice);
        <CappedBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
}

#[test]
fn capped_allow_block_list_applies_both_policies_and_tracks_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        mint_through::<CappedAllowBlockList>(&e, &alice, 100);
    });
    e.as_contract(&address, || {
        <CappedAllowBlockList as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <CappedAllowBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 20);
    });
    e.as_contract(&address, || {
        <CappedAllowBlockList as BurnableOverrides>::burn_from(&e, &spender, &alice, 10);
    });

    e.as_contract(&address, || {
        assert_eq!(Base::balance(&e, &alice), 70);
        assert_eq!(Base::balance(&e, &bob), 20);
        assert_eq!(total_supply(&e), 90);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn capped_allow_block_list_transfer_rejects_blocked_sender() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        AllowList::allow_user(&e, &alice);
        AllowList::allow_user(&e, &bob);
        mint_through::<CappedAllowBlockList>(&e, &alice, 100);
        BlockList::block_user(&e, &alice);
        <CappedAllowBlockList as ContractOverrides>::transfer(
            &e,
            &alice,
            &MuxedAddress::from(bob),
            30,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn capped_allow_block_list_burn_rejects_not_allowed_account() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        mint_through::<CappedAllowBlockList>(&e, &alice, 100);
        <CappedAllowBlockList as BurnableOverrides>::burn(&e, &alice, 10);
    });
}

// A cap lowered below the current supply blocks further minting until the
// supply is burned back under it (refer to `set_cap`).
#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn capped_mint_rejected_after_cap_lowered_below_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        mint_through::<Capped>(&e, &alice, 80);
        Capped::set_cap(&e, 50);
        mint_through::<Capped>(&e, &alice, 1);
    });
}

// Burning under a cap is plain supply-tracked burning, and frees room under
// the cap again.
#[test]
fn capped_burn_decreases_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        mint_through::<Capped>(&e, &alice, 100);
        <Capped as ContractOverrides>::approve(&e, &alice, &spender, 50, 1000);
    });
    e.as_contract(&address, || {
        <Capped as BurnableOverrides>::burn(&e, &alice, 30);
    });
    e.as_contract(&address, || {
        <Capped as BurnableOverrides>::burn_from(&e, &spender, &alice, 20);
        assert_eq!(Capped::total_supply(&e), 50);
        assert_eq!(Base::balance(&e, &alice), 50);
        // the burned amount can be minted again
        mint_through::<Capped>(&e, &alice, 50);
        assert_eq!(Capped::total_supply(&e), 100);
    });
}

// Every capped contract type reads the same supply counter.
#[test]
fn capped_combinations_report_the_tracked_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 100);
        mint_through::<Capped>(&e, &alice, 40);
        assert_eq!(Capped::total_supply(&e), 40);
        assert_eq!(CappedAllowList::total_supply(&e), 40);
        assert_eq!(CappedBlockList::total_supply(&e), 40);
        assert_eq!(CappedAllowBlockList::total_supply(&e), 40);
    });
}

#[test]
fn capped_block_list_transfer_from_and_burn_from_apply_policy_and_track_supply() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        mint_through::<CappedBlockList>(&e, &alice, 100);
        <CappedBlockList as ContractOverrides>::approve(&e, &alice, &spender, 60, 1000);
    });
    e.as_contract(&address, || {
        <CappedBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 30);
    });
    e.as_contract(&address, || {
        <CappedBlockList as BurnableOverrides>::burn_from(&e, &spender, &alice, 20);
        assert_eq!(Base::balance(&e, &alice), 50);
        assert_eq!(Base::balance(&e, &bob), 30);
        assert_eq!(Base::allowance(&e, &alice, &spender), 10);
        assert_eq!(total_supply(&e), 80);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn capped_block_list_transfer_from_rejects_blocked_receiver() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        Capped::set_cap(&e, 1000);
        mint_through::<CappedBlockList>(&e, &alice, 100);
        <CappedBlockList as ContractOverrides>::approve(&e, &alice, &spender, 60, 1000);
        BlockList::block_user(&e, &bob);
    });
    e.as_contract(&address, || {
        <CappedBlockList as ContractOverrides>::transfer_from(&e, &spender, &alice, &bob, 30);
    });
}
