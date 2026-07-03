extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    Address, Env, Event,
};

use crate::non_fungible::{extensions::burnable::Burn, Approve, ApproveForAll, Base, Mint};

#[contract]
struct MockContract;

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn(&e, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let events = e.events().all();
        assert_eq!(events.events().len(), 2);
        assert_eq!(
            events.events().first().unwrap(),
            &Mint { to: owner.clone(), token_id }.to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(1).unwrap(),
            &Burn { from: owner.clone(), token_id }.to_xdr(&e, &address)
        );
    });
}

#[test]
fn burn_from_with_approve_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::approve(&e, &owner, &spender, token_id, 1000);
        Base::burn_from(&e, &spender, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let events = e.events().all();
        assert_eq!(events.events().len(), 3);
        assert_eq!(
            events.events().first().unwrap(),
            &Mint { to: owner.clone(), token_id }.to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(1).unwrap(),
            &Approve {
                approver: owner.clone(),
                token_id,
                approved: spender.clone(),
                live_until_ledger: 1000,
            }
            .to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(2).unwrap(),
            &Burn { from: owner.clone(), token_id }.to_xdr(&e, &address)
        );
    });
}

#[test]
fn burn_from_with_operator_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::approve_for_all(&e, &owner, &operator, 1000);

        Base::burn_from(&e, &operator, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let events = e.events().all();
        assert_eq!(events.events().len(), 3);
        assert_eq!(
            events.events().first().unwrap(),
            &Mint { to: owner.clone(), token_id }.to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(1).unwrap(),
            &ApproveForAll {
                owner: owner.clone(),
                operator: operator.clone(),
                live_until_ledger: 1000
            }
            .to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(2).unwrap(),
            &Burn { from: owner.clone(), token_id }.to_xdr(&e, &address)
        );
    });
}

#[test]
fn burn_from_with_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn_from(&e, &owner, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let events = e.events().all();
        assert_eq!(events.events().len(), 2);
        assert_eq!(
            events.events().first().unwrap(),
            &Mint { to: owner.clone(), token_id }.to_xdr(&e, &address)
        );
        assert_eq!(
            events.events().get(1).unwrap(),
            &Burn { from: owner.clone(), token_id }.to_xdr(&e, &address)
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn burn_with_not_owner_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn(&e, &spender, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")]
fn burn_from_with_insufficient_approval_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn_from(&e, &spender, &owner, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")]
fn burn_with_non_existent_token_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let non_existent_token_id = 2;

    e.as_contract(&address, || {
        let _token_id = Base::sequential_mint(&e, &owner);

        Base::burn(&e, &owner, non_existent_token_id);
    });
}
