#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env, String, Vec};
use stellar_event_assertion::EventAssertion;

use super::storage::{find_bit_in_bucket, find_bit_in_item, BUCKETS, IDS_IN_BUCKET};
use crate::{
    extensions::consecutive::{
        storage::{NFTConsecutiveStorageKey, IDS_IN_ITEM},
        Consecutive,
    },
    sequential, Base,
};

#[contract]
pub struct MockContract;

#[test]
fn consecutive_find_bit_in_item() {
    assert_eq!(find_bit_in_item(Some(0b00000000000000000000000000000001), 0), Some(31));
    assert_eq!(find_bit_in_item(Some(0b10000000000000000000000000000001), 0), Some(0));
    assert_eq!(find_bit_in_item(Some(0), 0), None);

    assert_eq!(find_bit_in_item(Some(0b11000000000000000000000000000001), 10), Some(31));
    assert_eq!(find_bit_in_item(Some(0b10000000000000000000000000000001), 31), Some(31));
    assert_eq!(find_bit_in_item(Some(0b10000000001000000000000000000001), 9), Some(10));
    assert_eq!(find_bit_in_item(Some(0b10000000001000000000000000000001), 10), Some(10));
    assert_eq!(find_bit_in_item(Some(0b10000000001000010000000000000000), 11), Some(15));
    assert_eq!(find_bit_in_item(Some(0b10000000000000000000000000000000), 10), None);

    assert_eq!(find_bit_in_item(Some(0b00000000000000000000000000000001), 32), None);
    assert_eq!(find_bit_in_item(None, 0), None);
}

#[test]
fn consecutive_find_bit_in_bucket() {
    let e = Env::default();
    let ids = IDS_IN_ITEM as u32;

    assert_eq!(
        find_bit_in_bucket(
            vec![&e, 0b00001000000000000000000000000000, 0b10001000000000000000000000000001],
            5
        ),
        Some(32)
    );
    assert_eq!(find_bit_in_bucket(vec![&e, 1 << 31], 0), Some(0));

    // One item, LSB set
    assert_eq!(find_bit_in_bucket(vec![&e, 1], 0), Some(31));

    // One item, bit in middle, start before it
    assert_eq!(find_bit_in_bucket(vec![&e, 1 << 16], 10), Some(15));

    // One item, start after the only set bit
    assert_eq!(find_bit_in_bucket(vec![&e, 1 << 31], 1), None);

    // Empty bucket
    assert_eq!(find_bit_in_bucket(vec![&e,], 0), None);

    // Start beyond bucket bit capacity
    assert_eq!(find_bit_in_bucket(vec![&e, 0, 0], 2 * ids), None);

    // Two items, bit in second item
    let bucket = vec![&e, 0, 1 << 30];
    assert_eq!(find_bit_in_bucket(bucket.clone(), ids), Some(ids + 1));

    // Two items, bit in second item, start skips it
    assert_eq!(find_bit_in_bucket(bucket, ids + 2), None);

    // Bit exactly at start position
    let bucket = vec![&e, 0, 1 << 29];
    assert_eq!(find_bit_in_bucket(bucket, ids + 2), Some(ids + 2));

    // All zeros
    assert_eq!(find_bit_in_bucket(vec![&e, 0, 0], 0), None);
}

#[test]
fn consecutive_set_ownership_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let _ = sequential::increment_token_id(&e, 1001);
        Consecutive::set_ownership_in_bucket(&e, 0);
        let bucket = e
            .storage()
            .instance()
            .get::<_, Vec<u32>>(&NFTConsecutiveStorageKey::OwnershipBucket(0))
            .unwrap();
        assert_eq!(bucket.get(0).unwrap(), 0b10000000000000000000000000000000);

        Consecutive::set_ownership_in_bucket(&e, 1);
        let bucket = e
            .storage()
            .instance()
            .get::<_, Vec<u32>>(&NFTConsecutiveStorageKey::OwnershipBucket(0))
            .unwrap();
        assert_eq!(bucket.get(0).unwrap(), 0b11000000000000000000000000000000);

        Consecutive::set_ownership_in_bucket(&e, 31);
        let bucket = e
            .storage()
            .instance()
            .get::<_, Vec<u32>>(&NFTConsecutiveStorageKey::OwnershipBucket(0))
            .unwrap();
        assert_eq!(bucket.get(0).unwrap(), 0b11000000000000000000000000000001);

        Consecutive::set_ownership_in_bucket(&e, 32);
        let bucket = e
            .storage()
            .instance()
            .get::<_, Vec<u32>>(&NFTConsecutiveStorageKey::OwnershipBucket(0))
            .unwrap();
        assert_eq!(bucket.get(1).unwrap(), 0b10000000000000000000000000000000);

        Consecutive::set_ownership_in_bucket(&e, 45);
        let bucket = e
            .storage()
            .instance()
            .get::<_, Vec<u32>>(&NFTConsecutiveStorageKey::OwnershipBucket(0))
            .unwrap();
        assert_eq!(bucket.get(0).unwrap(), 0b11000000000000000000000000000001);
        assert_eq!(bucket.get(1).unwrap(), 0b10000000000001000000000000000000);
        assert_eq!(bucket.get(2).unwrap(), 0);

        Consecutive::set_ownership_in_bucket(&e, 1000);
        let bucket = e
            .storage()
            .instance()
            .get::<_, Vec<u32>>(&NFTConsecutiveStorageKey::OwnershipBucket(0))
            .unwrap();
        assert_eq!(bucket.get(31).unwrap(), 0b00000000100000000000000000000000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_set_ownership_panics_for_max_sequential_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let _ = sequential::increment_token_id(&e, 100);
        Consecutive::set_ownership_in_bucket(&e, sequential::next_token_id(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #306)")]
fn consecutive_set_ownership_panics_for_max_allowed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let max = (IDS_IN_BUCKET * BUCKETS) as u32;
        // increment sequential more than max
        let _ = sequential::increment_token_id(&e, max + 100);
        Consecutive::set_ownership_in_bucket(&e, max);
    });
}

#[test]
fn consecutive_owner_of_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    e.as_contract(&address, || {
        let ids_in_bucket = IDS_IN_BUCKET as u32;
        let max = 3 * ids_in_bucket + 1;
        let _ = sequential::increment_token_id(&e, max);

        Consecutive::set_ownership_in_bucket(&e, 0);
        e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(0), &user1);
        assert_eq!(Consecutive::owner_of(&e, 0), user1);

        Consecutive::set_ownership_in_bucket(&e, 1_000);
        e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(1_000), &user2);
        assert_eq!(Consecutive::owner_of(&e, 0), user1);
        assert_eq!(Consecutive::owner_of(&e, 10), user2);
        assert_eq!(Consecutive::owner_of(&e, 1_000), user2);

        // skip one bucket
        Consecutive::set_ownership_in_bucket(&e, max - 1);
        e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(max - 1), &user3);
        assert_eq!(Consecutive::owner_of(&e, max - 1000), user3);
        assert_eq!(Consecutive::owner_of(&e, max - ids_in_bucket), user3);
        assert_eq!(Consecutive::owner_of(&e, max - 2 * ids_in_bucket), user3);
        assert_eq!(Consecutive::owner_of(&e, 1_001), user3);
    });
}

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

        assert_eq!(sequential::next_token_id(&e), amount);
        assert_eq!(Base::balance(&e, &owner), amount);

        let _owner = e
            .storage()
            .persistent()
            .get::<_, Address>(&NFTConsecutiveStorageKey::Owner(99))
            .unwrap();
        assert_eq!(_owner, owner);
        assert_eq!(Consecutive::owner_of(&e, 0), owner);
        assert_eq!(Consecutive::owner_of(&e, 50), owner);
        assert_eq!(Consecutive::owner_of(&e, 99), owner);

        // new mint
        let last_id = Consecutive::batch_mint(&e, &owner, amount);
        assert_eq!(last_id, 2 * amount - 1);
        assert_eq!(Base::balance(&e, &owner), 2 * amount);
        assert_eq!(Consecutive::owner_of(&e, 2 * amount - 1), owner);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #312)")]
fn consecutive_batch_mint_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 0;

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, amount);
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
        assert_eq!(Consecutive::owner_of(&e, 49), owner);
        let _owner = e
            .storage()
            .persistent()
            .get::<_, Address>(&NFTConsecutiveStorageKey::Owner(49))
            .unwrap();
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

        let _owner =
            e.storage().persistent().get::<_, Address>(&NFTConsecutiveStorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        let _owner = e
            .storage()
            .persistent()
            .get::<_, Address>(&NFTConsecutiveStorageKey::Owner(token_id - 1))
            .unwrap();
        assert_eq!(_owner, owner);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_consecutive_mint(&owner, 0, 99);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });

    e.as_contract(&address, || {
        Consecutive::burn(&e, &owner, 0);
        assert_eq!(Base::balance(&e, &owner), amount - 2);

        let _owner =
            e.storage().persistent().get::<_, Address>(&NFTConsecutiveStorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        assert_eq!(Consecutive::owner_of(&e, 1), owner);
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
        let burned = e
            .storage()
            .persistent()
            .get::<_, bool>(&NFTConsecutiveStorageKey::BurnedToken(token_id))
            .unwrap();
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
fn consecutive_set_owner_for_previous_token_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &user1, 5); // 0,1,2,3,4

        // existing id
        Consecutive::set_owner_for_previous_token(&e, &user2, 3);
        assert_eq!(Consecutive::owner_of(&e, 2), user2);

        // when 0 -> does nothing
        Consecutive::set_owner_for_previous_token(&e, &user3, 0);
        assert_eq!(Consecutive::owner_of(&e, 0), user2);

        // when more than max -> does nothing
        Consecutive::set_owner_for_previous_token(&e, &user2, 5);
        assert_eq!(Consecutive::owner_of(&e, 4), user1);

        // when already has owner -> does nothing
        e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(3), &user3);
        Consecutive::set_owner_for_previous_token(&e, &user2, 4);
        let owner = e
            .storage()
            .persistent()
            .get::<_, Address>(&NFTConsecutiveStorageKey::Owner(3))
            .unwrap();
        assert_eq!(owner, user3);

        // when is burned -> does nothing
        e.storage().persistent().set(&NFTConsecutiveStorageKey::BurnedToken(1), &true);
        Consecutive::set_owner_for_previous_token(&e, &user2, 2);
        let owner = e.storage().persistent().get::<_, Address>(&NFTConsecutiveStorageKey::Owner(0));
        assert_eq!(owner, None);
    });
}

#[test]
fn consecutive_token_uri_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let base_uri = String::from_str(&e, "https://smth.com/");
        let collection_name = String::from_str(&e, "My NFT collection");
        let collection_symbol = String::from_str(&e, "NFT");
        Base::set_metadata(
            &e,
            base_uri.clone(),
            collection_name.clone(),
            collection_symbol.clone(),
        );

        let _ = sequential::increment_token_id(&e, 10);
        let uri = Consecutive::token_uri(&e, 9);

        assert_eq!(uri, String::from_str(&e, "https://smth.com/9"));
        assert_eq!(collection_name, Base::name(&e));
        assert_eq!(collection_symbol, Base::symbol(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_token_uri_panics_for_more_than_max_id_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let _ = sequential::increment_token_id(&e, 100);
        Consecutive::token_uri(&e, sequential::next_token_id(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_token_uri_panics_for_more_than_total_ids_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let max = (IDS_IN_BUCKET * BUCKETS) as u32;
        // increment sequential more than max
        let _ = sequential::increment_token_id(&e, max + 100);
        Consecutive::token_uri(&e, max);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn consecutive_token_uri_panics_for_burned_id_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, 1);
        Consecutive::burn(&e, &owner, 0);
        Consecutive::token_uri(&e, 0);
    });
}
