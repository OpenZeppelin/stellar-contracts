extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

use crate::contract::{VestingWalletContract, VestingWalletContractClient};

const START: u64 = 1_000;
const DURATION: u64 = 1_000;
const CLIFF_DURATION: u64 = 100;

fn create_client<'a>(e: &Env, receiver: &Address) -> VestingWalletContractClient<'a> {
    let address = e.register(VestingWalletContract, (receiver, START, DURATION, CLIFF_DURATION));
    VestingWalletContractClient::new(e, &address)
}

#[test]
fn constructor_sets_owner_and_schedule_works() {
    let e = Env::default();
    let receiver = Address::generate(&e);
    let client = create_client(&e, &receiver);

    assert_eq!(client.get_owner(), Some(receiver));
    assert_eq!(client.start(), START);
    assert_eq!(client.duration(), DURATION);
    assert_eq!(client.cliff(), START + CLIFF_DURATION);
    assert_eq!(client.end(), START + DURATION);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // ExampleError::RenounceDisabled
fn renounce_ownership_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let client = create_client(&e, &Address::generate(&e));
    client.renounce_ownership();
}

#[test]
fn release_end_to_end_works() {
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let client = create_client(&e, &receiver);
    let token = e.register_stellar_asset_contract_v2(Address::generate(&e)).address();
    let token_client = TokenClient::new(&e, &token);

    // Funding is a plain transfer (here a mint) to the vesting contract.
    StellarAssetClient::new(&e, &token).mint(&client.address, &1_000);

    e.ledger().set_timestamp(START + CLIFF_DURATION - 1);
    assert_eq!(client.release(&token), 0);

    e.ledger().set_timestamp(START + DURATION / 2);
    assert_eq!(client.release(&token), 500);
    assert_eq!(token_client.balance(&receiver), 500);

    e.ledger().set_timestamp(START + DURATION);
    assert_eq!(client.release(&token), 500);
    assert_eq!(token_client.balance(&receiver), 1_000);
    assert_eq!(token_client.balance(&client.address), 0);
}
