extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    Address, Env, Event,
};

use crate::fungible::{extensions::burnable::Burn, Approve, Base, Mint};

#[contract]
struct MockContract;

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        Base::mint(&e, &account, 100);
        Base::burn(&e, &account, 50);
        assert_eq!(Base::balance(&e, &account), 50);

        let events = e.events().all();
        assert_eq!(events.events().len(), 2);
        assert_eq!(
            events.events().first().unwrap(),
            &Mint { to: account.clone(), amount: 100 }.to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(1).unwrap(),
            &Burn { from: account.clone(), amount: 50 }.to_xdr(&e, &address)
        );
    });
}

#[test]
fn burn_with_allowance_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        Base::mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 30, 1000);
        Base::burn_from(&e, &spender, &owner, 30);
        assert_eq!(Base::balance(&e, &owner), 70);
        assert_eq!(Base::balance(&e, &spender), 0);

        let events = e.events().all();
        assert_eq!(events.events().len(), 3);
        assert_eq!(
            events.events().first().unwrap(),
            &Mint { to: owner.clone(), amount: 100 }.to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(1).unwrap(),
            &Approve {
                owner: owner.clone(),
                spender: spender.clone(),
                amount: 30,
                live_until_ledger: 1000,
            }
            .to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(2).unwrap(),
            &Burn { from: owner.clone(), amount: 30 }.to_xdr(&e, &address)
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn burn_with_insufficient_balance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        Base::mint(&e, &account, 100);
        assert_eq!(Base::balance(&e, &account), 100);
        Base::burn(&e, &account, 101);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn burn_with_no_allowance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        Base::mint(&e, &owner, 100);
        assert_eq!(Base::balance(&e, &owner), 100);
        Base::burn_from(&e, &spender, &owner, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn burn_with_insufficient_allowance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        Base::mint(&e, &owner, 100);
        Base::approve(&e, &owner, &spender, 50, 100);
        assert_eq!(Base::allowance(&e, &owner, &spender), 50);
        assert_eq!(Base::balance(&e, &owner), 100);
        Base::burn_from(&e, &spender, &owner, 60);
    });
}
