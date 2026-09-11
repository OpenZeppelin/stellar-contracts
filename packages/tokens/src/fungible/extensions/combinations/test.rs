extern crate std;

use core::any::TypeId;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, MuxedAddress};
use stellar_governance::votes::get_voting_units;

use crate::{
    fungible::{
        extensions::{
            allowlist::{AllowList, AllowListContractType},
            blocklist::{BlockList, BlockListContractType},
            burnable::Burnable,
            combinations::{Composable, Compose},
            total_supply::{mint, total_supply, TotalSupply},
            votes::FungibleVotes,
        },
        overrides::{BurnableOverrides, TotalSupplyOverrides},
        Base, ContractOverrides,
    },
    rwa::RWA,
    vault::Vault,
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
    assert_composes_to::<(AllowList, TotalSupply, Burnable), TotalSupplyAllowList>();
    assert_composes_to::<(Burnable, TotalSupply, BlockList), TotalSupplyBlockList>();
}

#[test]
fn additive_only_lists_resolve_to_base() {
    assert_composes_to::<Burnable, Base>();
    assert_composes_to::<(Burnable,), Base>();
    assert_composes_to::<(Burnable, Burnable), Base>();
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
    assert_supply::<FungibleVotes>();
    assert_supply::<AllowListVotes>();
    assert_supply::<BlockListVotes>();
    assert_supply::<AllowBlockListVotes>();
    assert_supply::<RWA>();
    assert_supply::<Vault>();
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

#[test]
fn votes_combinations_serve_total_supply_from_checkpoints() {
    let (e, address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&address, || {
        AllowList::allow_user(&e, &alice);
        AllowBlockListVotes::mint(&e, &alice, 100);
        assert_eq!(<AllowListVotes as TotalSupplyOverrides>::total_supply(&e), 100);
        assert_eq!(<BlockListVotes as TotalSupplyOverrides>::total_supply(&e), 100);
        assert_eq!(<AllowBlockListVotes as TotalSupplyOverrides>::total_supply(&e), 100);
    });
    e.as_contract(&address, || {
        <AllowBlockListVotes as BurnableOverrides>::burn(&e, &alice, 40);
    });
    e.as_contract(&address, || {
        assert_eq!(<AllowBlockListVotes as TotalSupplyOverrides>::total_supply(&e), 60);
    });
}
