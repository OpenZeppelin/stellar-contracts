extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};

use crate::non_fungible::{
    consecutive::Consecutive, extensions::enumerable::Enumerable, royalties::NonFungibleRoyalties,
    Base, NonFungibleToken,
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
#[should_panic(expected = "Error(Contract, #200)")]
fn test_royalty_info_non_existent_token() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Try to get royalty info for non-existent token
        Base::royalty_info(&e, 999, 1000);
    });
}

#[test]
fn test_no_royalty_set() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Check royalty info
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 1000);
        assert_eq!(royalty_receiver, e.current_contract_address());
        assert_eq!(royalty_amount, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #212)")]
fn test_invalid_royalty_amount() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set invalid royalty amount
        Base::set_token_royalty(&e, token_id, &Address::generate(&e), 10001);
    });
}

#[test]
fn test_remove_token_royalty() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let default_receiver = Address::generate(&e);
    let token_receiver = Address::generate(&e);

    e.as_contract(&address, || {
        // Set default royalty
        Base::set_default_royalty(&e, &default_receiver, 1000); // 10%

        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set token-specific royalty
        Base::set_token_royalty(&e, token_id, &token_receiver, 500); // 5%

        // Verify token-specific royalty is used
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 2000);
        assert_eq!(royalty_receiver, token_receiver);
        assert_eq!(royalty_amount, 100); // 5% of 2000

        // Remove token-specific royalty
        Base::remove_token_royalty(&e, token_id);

        // Verify default royalty is now used
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 2000);
        assert_eq!(royalty_receiver, default_receiver);
        assert_eq!(royalty_amount, 200); // 10% of 2000
    });
}

#[test]
fn test_remove_token_royalty_no_default() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let token_receiver = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint a token
        let token_id = Enumerable::sequential_mint(&e, &owner);

        // Set token-specific royalty
        Base::set_token_royalty(&e, token_id, &token_receiver, 500); // 5%

        // Verify token-specific royalty is used
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 2000);
        assert_eq!(royalty_receiver, token_receiver);
        assert_eq!(royalty_amount, 100); // 5% of 2000

        // Remove token-specific royalty
        Base::remove_token_royalty(&e, token_id);

        // Verify zero royalty is now used (since no default is set)
        let (royalty_receiver, royalty_amount) = Base::royalty_info(&e, token_id, 2000);
        assert_eq!(royalty_receiver, e.current_contract_address());
        assert_eq!(royalty_amount, 0);
    });
}

// A contract that pairs the royalties extension with `Consecutive`, whose
// ownership is stored sparsely: only the last token of a batch gets a
// materialised `Owner` entry, and the rest are resolved by walking back
// through the bucket. `royalty_info` must therefore establish existence
// through `Self::owner_of` rather than `Base::owner_of`.
#[contract]
struct ConsecutiveRoyaltiesContract;

#[contractimpl(contracttrait)]
impl NonFungibleToken for ConsecutiveRoyaltiesContract {
    type ContractType = Consecutive;
}

#[contractimpl(contracttrait)]
impl NonFungibleRoyalties for ConsecutiveRoyaltiesContract {
    fn set_default_royalty(e: &Env, receiver: Address, basis_points: u32, _operator: Address) {
        Base::set_default_royalty(e, &receiver, basis_points);
    }

    fn set_token_royalty(
        e: &Env,
        token_id: u32,
        receiver: Address,
        basis_points: u32,
        _operator: Address,
    ) {
        Base::set_token_royalty(e, token_id, &receiver, basis_points);
    }

    fn remove_token_royalty(e: &Env, token_id: u32, _operator: Address) {
        Base::remove_token_royalty(e, token_id);
    }
}

#[test]
fn test_royalty_info_resolves_every_token_under_consecutive() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(ConsecutiveRoyaltiesContract, ());
    let client = ConsecutiveRoyaltiesContractClient::new(&e, &address);

    let owner = Address::generate(&e);
    let receiver = Address::generate(&e);

    e.as_contract(&address, || {
        Consecutive::batch_mint(&e, &owner, 10);
        Base::set_default_royalty(&e, &receiver, 500); // 5%
    });

    // Every token in the batch exists and resolves an owner, so every token
    // must also resolve royalty information. Before this was fixed, only
    // token 9 answered: it is the batch boundary and therefore the only id
    // with a materialised `Owner` entry.
    for token_id in 0..10u32 {
        assert_eq!(client.owner_of(&token_id), owner);

        let (royalty_receiver, royalty_amount) = client.royalty_info(&token_id, &1_000_000);
        assert_eq!(royalty_receiver, receiver);
        assert_eq!(royalty_amount, 50_000); // 5% of 1_000_000
    }
}
