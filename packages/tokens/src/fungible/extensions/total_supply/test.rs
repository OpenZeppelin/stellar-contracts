extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, MuxedAddress, String};

use crate::fungible::{
    extensions::total_supply::{
        decrease_total_supply, increase_total_supply, mint, total_supply, TotalSupply,
        TotalSupplyOverrides,
    },
    overrides::BurnableOverrides,
    Base, ContractOverrides,
};

#[contract]
struct MockContract;

#[test]
fn initial_supply_is_zero() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        assert_eq!(total_supply(&e), 0);
        assert_eq!(<TotalSupply as TotalSupplyOverrides>::total_supply(&e), 0);
    });
}

#[test]
fn mint_increases_supply_and_balance() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        assert_eq!(Base::balance(&e, &account), 100);
        assert_eq!(total_supply(&e), 100);

        mint(&e, &account, 50);
        assert_eq!(Base::balance(&e, &account), 150);
        assert_eq!(total_supply(&e), 150);
    });
}

#[test]
fn burn_decreases_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        <TotalSupply as BurnableOverrides>::burn(&e, &account, 40);
        assert_eq!(Base::balance(&e, &account), 60);
        assert_eq!(total_supply(&e), 60);
    });
}

#[test]
fn burn_from_decreases_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 40, e.ledger().sequence() + 100);
        <TotalSupply as BurnableOverrides>::burn_from(&e, &spender, &owner, 40);
        assert_eq!(Base::balance(&e, &owner), 60);
        assert_eq!(total_supply(&e), 60);
    });
}

#[test]
fn contract_type_delegates_token_operations() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &from, 100);
        TotalSupply::transfer(&e, &from, &MuxedAddress::from(recipient.clone()), 30);
        assert_eq!(TotalSupply::balance(&e, &from), 70);
        assert_eq!(TotalSupply::balance(&e, &recipient), 30);
        // transfers leave the supply untouched
        assert_eq!(total_supply(&e), 100);
    });

    // separate invocation, so that `from` can authorize again
    e.as_contract(&address, || {
        TotalSupply::approve(&e, &from, &recipient, 20, e.ledger().sequence() + 100);
        assert_eq!(TotalSupply::allowance(&e, &from, &recipient), 20);
        TotalSupply::transfer_from(&e, &recipient, &from, &recipient, 20);
        assert_eq!(TotalSupply::balance(&e, &recipient), 50);
        assert_eq!(total_supply(&e), 100);

        Base::set_metadata(&e, 7, String::from_str(&e, "My Token"), String::from_str(&e, "TKN"));
        assert_eq!(TotalSupply::decimals(&e), 7);
        assert_eq!(TotalSupply::name(&e), String::from_str(&e, "My Token"));
        assert_eq!(TotalSupply::symbol(&e), String::from_str(&e, "TKN"));
    });
}

#[test]
fn increase_and_decrease_total_supply_work() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        increase_total_supply(&e, 100);
        assert_eq!(total_supply(&e), 100);
        decrease_total_supply(&e, 60);
        assert_eq!(total_supply(&e), 40);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn increase_total_supply_rejects_negative_amount() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        increase_total_supply(&e, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn decrease_total_supply_rejects_negative_amount() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        decrease_total_supply(&e, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #104)")]
fn increase_total_supply_overflow_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        increase_total_supply(&e, i128::MAX);
        increase_total_supply(&e, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #104)")]
fn decrease_total_supply_underflow_panics() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        decrease_total_supply(&e, 1);
    });
}
