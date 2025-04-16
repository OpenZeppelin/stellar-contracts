#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::{
    consecutive::{storage::StorageKey, Consecutive},
    sequential::next_token_id,
    Base,
};

#[contract]
pub struct MockContract;

#[test]
fn consecutive_batch_mint_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_consecutive_mint(&owner, 0, 99);

        assert_eq!(next_token_id(&e), amount);
        assert_eq!(Base::balance(&e, &owner), amount);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(0)).unwrap();
        assert_eq!(_owner, owner);
        assert_eq!(Consecutive::owner_of(&e, 50), owner);

        // new mint
        let last_id = Consecutive::batch_mint(&e, &owner, amount);
        assert_eq!(last_id, 2 * amount - 1);
        assert_eq!(Base::balance(&e, &owner), 2 * amount);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_owner_of_on_nonexistent_token_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &user, 5);
        // token 5 is out of range
        Consecutive::owner_of(&e, 5);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_owner_of_panics_on_burnt_token_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &user, 10);
        Consecutive::burn(&e, &user, 2);
        Consecutive::owner_of(&e, 2);
    });
}

#[test]
fn consecutive_transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);
        assert_eq!(Base::balance(&e, &owner), amount);

        Consecutive::transfer(&e, &owner, &recipient, 50);
        assert_eq!(Consecutive::owner_of(&e, 50), recipient);
        assert_eq!(Base::balance(&e, &recipient), 1);

        assert_eq!(Consecutive::owner_of(&e, 51), owner);
        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(51)).unwrap();
        assert_eq!(_owner, owner);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_consecutive_mint(&owner, 0, 99);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, 50);
    });
}

#[test]
fn consecutive_transfer_edge_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_consecutive_mint(&owner, 0, 99);

        assert_eq!(Consecutive::owner_of(&e, 0), owner);
        Consecutive::transfer(&e, &owner, &recipient, 0);
        assert_eq!(Consecutive::owner_of(&e, 0), recipient);
        assert_eq!(Consecutive::owner_of(&e, 1), owner);
    });

    e.as_contract(&address, || {
        Consecutive::transfer(&e, &owner, &recipient, 99);
        assert_eq!(Consecutive::owner_of(&e, 99), recipient);
        assert_eq!(Base::balance(&e, &recipient), 2);
    });
}

#[test]
fn consecutive_transfer_from_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let spender = Address::generate(&e);
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100;
    let token_id = 50;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);
        assert_eq!(Base::balance(&e, &owner), amount);

        Consecutive::approve(&e, &owner, &spender, token_id, 100);
        Consecutive::transfer_from(&e, &spender, &owner, &recipient, token_id);
        assert_eq!(Consecutive::owner_of(&e, token_id), recipient);
        assert_eq!(Base::balance(&e, &recipient), 1);

        assert_eq!(Consecutive::owner_of(&e, token_id + 1), owner);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_consecutive_mint(&owner, 0, 99);
        event_assert.assert_non_fungible_approve(&owner, &spender, token_id, 100);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, token_id);
    });
}

#[test]
fn consecutive_burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100;
    let token_id = 50;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);
        assert_eq!(Base::balance(&e, &owner), amount);

        Consecutive::burn(&e, &owner, token_id);
        assert_eq!(Base::balance(&e, &owner), amount - 1);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        let _owner =
            e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id + 1)).unwrap();
        assert_eq!(_owner, owner);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_consecutive_mint(&owner, 0, 99);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });
}

#[test]
fn consecutive_burn_from_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let amount = 100;
    let token_id = 42;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);
        Consecutive::approve(&e, &owner, &spender, token_id, 100);
        Consecutive::burn_from(&e, &spender, &owner, token_id);

        assert_eq!(Base::balance(&e, &owner), amount - 1);
        let burned =
            e.storage().persistent().get::<_, bool>(&StorageKey::BurnedToken(token_id)).unwrap();
        assert!(burned);
        assert_eq!(Consecutive::owner_of(&e, token_id + 1), owner);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_consecutive_mint(&owner, 0, 99);
        event_assert.assert_non_fungible_approve(&owner, &spender, token_id, 100);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });
}

#[test]
fn consecutive_set_owner_for_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &user1, 5); // 0,1,2,3,4

        // existing id
        Consecutive::set_owner_for(&e, &user2, 2);
        assert_eq!(Consecutive::owner_of(&e, 2), user2);

        // when more than max -> does nothing
        Consecutive::set_owner_for(&e, &user2, 5);
        let owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(5));
        assert_eq!(owner, None);

        // when already has owner -> does nothing
        e.storage().persistent().set(&StorageKey::Owner(3), &user3);
        Consecutive::set_owner_for(&e, &user2, 3);
        assert_eq!(Consecutive::owner_of(&e, 3), user3);

        // when is burned -> does nothing
        Consecutive::burn(&e, &user1, 0);
        Consecutive::set_owner_for(&e, &user2, 0);
        let owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(0));
        assert_eq!(owner, None);
    });
}
