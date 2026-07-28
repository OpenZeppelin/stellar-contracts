extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, MuxedAddress};

use crate::fungible::{
    allowlist::AllowList,
    blocklist::BlockList,
    extensions::combinations::Compose,
    overrides::BurnableOverrides,
    total_supply::{mint, total_supply, TotalSupply},
    Base, ContractOverrides,
};

type AllowListWithSupply = Compose<(AllowList, TotalSupply)>;
// deliberately the swapped order, asserting that the list is
// order-insensitive
type BlockListWithSupply = Compose<(TotalSupply, BlockList)>;

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
