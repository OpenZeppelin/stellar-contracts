extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env, String};
use stellar_governance::votes::{delegate, get_delegate, get_votes, get_voting_units};

use crate::non_fungible::{extensions::votes::NonFungibleVotes, Base};

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
fn mint_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);

        assert_eq!(Base::balance(&e, &alice), 1);
        assert_eq!(get_voting_units(&e, &alice), 1);

        NonFungibleVotes::mint(&e, &alice, 2);

        assert_eq!(Base::balance(&e, &alice), 2);
        assert_eq!(get_voting_units(&e, &alice), 2);
    });
}

#[test]
fn mint_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        NonFungibleVotes::mint(&e, &alice, 1);

        assert_eq!(Base::balance(&e, &alice), 1);
        assert_eq!(get_voting_units(&e, &alice), 1);
        assert_eq!(get_votes(&e, &bob), 1);
    });
}

#[test]
fn sequential_mint_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        let token_id = NonFungibleVotes::sequential_mint(&e, &alice);

        assert_eq!(token_id, 0);
        assert_eq!(Base::balance(&e, &alice), 1);
        assert_eq!(get_voting_units(&e, &alice), 1);
    });
}

#[test]
fn burn_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);
        NonFungibleVotes::mint(&e, &alice, 2);
        assert_eq!(get_voting_units(&e, &alice), 2);
    });

    e.as_contract(&contract_address, || {
        NonFungibleVotes::burn(&e, &alice, 1);
        assert_eq!(Base::balance(&e, &alice), 1);
        assert_eq!(get_voting_units(&e, &alice), 1);
    });
}

#[test]
fn burn_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        delegate(&e, &alice, &bob);
        NonFungibleVotes::mint(&e, &alice, 1);
        NonFungibleVotes::mint(&e, &alice, 2);
        assert_eq!(get_votes(&e, &bob), 2);
    });

    e.as_contract(&contract_address, || {
        NonFungibleVotes::burn(&e, &alice, 1);
        assert_eq!(get_votes(&e, &bob), 1);
    });
}

#[test]
fn burn_from_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &owner, 1);
        Base::approve(&e, &owner, &spender, 1, 1000);
    });

    e.as_contract(&contract_address, || {
        NonFungibleVotes::burn_from(&e, &spender, &owner, 1);

        assert_eq!(Base::balance(&e, &owner), 0);
        assert_eq!(get_voting_units(&e, &owner), 0);
    });
}

#[test]
fn transfer_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);
        assert_eq!(get_voting_units(&e, &alice), 1);
    });

    e.as_contract(&contract_address, || {
        NonFungibleVotes::transfer(&e, &alice, &bob, 1);

        assert_eq!(Base::balance(&e, &alice), 0);
        assert_eq!(Base::balance(&e, &bob), 1);
        assert_eq!(get_voting_units(&e, &alice), 0);
        assert_eq!(get_voting_units(&e, &bob), 1);
    });
}

#[test]
fn transfer_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let delegate_a = Address::generate(&e);
    let delegate_b = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);
        NonFungibleVotes::mint(&e, &alice, 2);
        delegate(&e, &alice, &delegate_a);
        assert_eq!(get_votes(&e, &delegate_a), 2);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &bob, &delegate_b);
    });

    e.as_contract(&contract_address, || {
        NonFungibleVotes::transfer(&e, &alice, &bob, 1);

        assert_eq!(get_votes(&e, &delegate_a), 1);
        assert_eq!(get_votes(&e, &delegate_b), 1);
    });
}

#[test]
fn transfer_from_updates_voting_units() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &owner, 1);
        Base::approve(&e, &owner, &spender, 1, 1000);
    });

    e.as_contract(&contract_address, || {
        NonFungibleVotes::transfer_from(&e, &spender, &owner, &recipient, 1);

        assert_eq!(Base::balance(&e, &owner), 0);
        assert_eq!(Base::balance(&e, &recipient), 1);
        assert_eq!(get_voting_units(&e, &owner), 0);
        assert_eq!(get_voting_units(&e, &recipient), 1);
    });
}

#[test]
fn transfer_from_with_delegation_updates_votes() {
    let (e, contract_address) = setup_env();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let delegate_owner = Address::generate(&e);
    let delegate_recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &owner, 1);
        delegate(&e, &owner, &delegate_owner);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &recipient, &delegate_recipient);
    });

    e.as_contract(&contract_address, || {
        Base::approve(&e, &owner, &spender, 1, 1000);
        NonFungibleVotes::transfer_from(&e, &spender, &owner, &recipient, 1);

        assert_eq!(get_votes(&e, &delegate_owner), 0);
        assert_eq!(get_votes(&e, &delegate_recipient), 1);
    });
}

#[test]
fn self_delegation() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);
        delegate(&e, &alice, &alice);

        assert_eq!(get_delegate(&e, &alice), Some(alice.clone()));
        assert_eq!(get_votes(&e, &alice), 1);
    });
}

#[test]
fn multiple_holders_with_same_delegate() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let charlie = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);
        NonFungibleVotes::mint(&e, &alice, 2);
        NonFungibleVotes::mint(&e, &bob, 3);

        delegate(&e, &alice, &charlie);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &bob, &charlie);

        assert_eq!(get_votes(&e, &charlie), 3);
    });
}

#[test]
fn transfer_between_delegated_accounts() {
    let (e, contract_address) = setup_env();
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    e.as_contract(&contract_address, || {
        NonFungibleVotes::mint(&e, &alice, 1);
        NonFungibleVotes::mint(&e, &alice, 2);
        NonFungibleVotes::mint(&e, &bob, 3);
        delegate(&e, &alice, &alice);
    });

    e.as_contract(&contract_address, || {
        delegate(&e, &bob, &bob);
    });

    e.as_contract(&contract_address, || {
        assert_eq!(get_votes(&e, &alice), 2);
        assert_eq!(get_votes(&e, &bob), 1);

        NonFungibleVotes::transfer(&e, &alice, &bob, 1);

        assert_eq!(get_votes(&e, &alice), 1);
        assert_eq!(get_votes(&e, &bob), 2);
    });
}
