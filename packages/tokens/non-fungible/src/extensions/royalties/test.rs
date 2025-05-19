#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{
    extensions::{enumerable::Enumerable, royalties::MAX_ROYALTY_BASIS_POINTS},
    Base,
};

#[contract]
struct MockContract;

#[test]
fn test_set_default_royalty() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let receiver = Address::generate(&e);

    let token_id =
        e.as_contract(&address, || Enumerable::sequential_mint(&e, &Address::generate(&e)));

    e.as_contract(&address, || {
        // Set default royalty
        Base::set_default_royalty(&e, &receiver, 1000); // 10%

        // Check royalty info for a non-existent token (should use default)
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 1000);
        assert_eq!(royalty_receiver, receiver);
        assert_eq!(royalty_amount, 100); // 10% of 1000
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #313)")]
fn test_set_default_royalty_too_high() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let receiver = Address::generate(&e);

    e.as_contract(&address, || {
        // Try to set royalty higher than maximum
        Base::set_default_royalty(&e, &receiver, MAX_ROYALTY_BASIS_POINTS + 1);
    });
}

#[test]
fn test_set_token_royalty() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set token-specific royalty
        let receiver = Address::generate(&e);
        Base::set_token_royalty(&e, token_id, &receiver, 500); // 5%

        // Check royalty info
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 2000);
        assert_eq!(royalty_receiver, receiver);
        assert_eq!(royalty_amount, 100); // 5% of 2000
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #313)")]
fn test_set_token_royalty_too_high() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let receiver = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Try to set royalty higher than maximum
        Base::set_token_royalty(&e, token_id, &receiver, MAX_ROYALTY_BASIS_POINTS + 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #314)")]
fn test_set_token_royalty_already_set() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let receiver = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set token royalty
        Base::set_token_royalty(&e, token_id, &receiver, 500);

        // Try to set it again (should fail)
        Base::set_token_royalty(&e, token_id, &receiver, 300);
    });
}

#[test]
fn test_token_royalty_overrides_default() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let default_receiver = Address::generate(&e);
    let token_receiver = Address::generate(&e);

    // First set default royalty and mint first token
    e.as_contract(&address, || {
        // Set default royalty
        Base::set_default_royalty(&e, &default_receiver, 1000); // 10%

        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set token-specific royalty
        Base::set_token_royalty(&e, token_id, &token_receiver, 500); // 5%

        // Check that token royalty overrides default
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 2000);
        assert_eq!(royalty_receiver, token_receiver);
        assert_eq!(royalty_amount, 100); // 5% of 2000

        // Mint another token without specific royalty
        let token_id2 = Enumerable::sequential_mint(&e, &owner);

        // Check that default royalty applies
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id2, 2000);
        assert_eq!(royalty_receiver, default_receiver);
        assert_eq!(royalty_amount, 200); // 10% of 2000
    });
}

#[test]
fn test_zero_royalty() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let receiver = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set zero royalty
        Base::set_token_royalty(&e, token_id, &receiver, 0);

        // Check royalty info
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 1000);
        assert_eq!(royalty_receiver, receiver);
        assert_eq!(royalty_amount, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn test_royalty_info_non_existent_token() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Try to get royalty info for non-existent token
        Base::royalty_info(&e, 999, 1000);
    });
}
