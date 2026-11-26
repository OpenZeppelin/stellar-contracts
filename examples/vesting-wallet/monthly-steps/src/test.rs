extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

use crate::contract::{VestingWalletContract, VestingWalletContractClient, MONTH};

const START: u64 = 1_000;

/// A vesting contract with no cliff, funded with 900 of a SAC. Returns the
/// client and the token.
fn setup<'a>(e: &Env, duration: u64) -> (VestingWalletContractClient<'a>, Address) {
    e.mock_all_auths();
    let address = e.register(VestingWalletContract, (Address::generate(e), START, duration, 0u64));
    let token = e.register_stellar_asset_contract_v2(Address::generate(e)).address();
    StellarAssetClient::new(e, &token).mint(&address, &900);
    (VestingWalletContractClient::new(e, &address), token)
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // ExampleError::RenounceDisabled
fn renounce_ownership_panics() {
    let e = Env::default();
    let (client, _) = setup(&e, 3 * MONTH);
    client.renounce_ownership();
}

#[test]
fn exact_months_works() {
    // 3 pieces, one per full month.
    let e = Env::default();
    let (client, token) = setup(&e, 3 * MONTH);

    assert_eq!(client.vested_amount(&token, &(START + MONTH - 1)), 0);
    assert_eq!(client.vested_amount(&token, &(START + MONTH)), 300);
    assert_eq!(client.vested_amount(&token, &(START + 2 * MONTH)), 600);
    assert_eq!(client.vested_amount(&token, &(START + 3 * MONTH - 1)), 600);
    assert_eq!(client.vested_amount(&token, &(START + 3 * MONTH)), 900);
}

#[test]
fn partial_last_month_works() {
    // 2.5 months is 3 pieces; the partial last month is
    // paid at `end`, never earlier.
    let e = Env::default();
    let duration = 5 * MONTH / 2;
    let (client, token) = setup(&e, duration);

    assert_eq!(client.vested_amount(&token, &(START + MONTH - 1)), 0);
    assert_eq!(client.vested_amount(&token, &(START + MONTH)), 300);
    assert_eq!(client.vested_amount(&token, &(START + 2 * MONTH)), 600);
    assert_eq!(client.vested_amount(&token, &(START + duration - 1)), 600);
    assert_eq!(client.vested_amount(&token, &(START + duration)), 900);
}

#[test]
fn shorter_than_month_works() {
    // Below one month there is a single piece, paid at
    // `end`, and no division by zero.
    let e = Env::default();
    let duration = MONTH / 2;
    let (client, token) = setup(&e, duration);

    assert_eq!(client.vested_amount(&token, &START), 0);
    assert_eq!(client.vested_amount(&token, &(START + duration - 1)), 0);
    assert_eq!(client.vested_amount(&token, &(START + duration)), 900);
}

#[test]
fn release_follows_monthly_steps_works() {
    let e = Env::default();
    let (client, token) = setup(&e, 3 * MONTH);
    let receiver = client.get_owner().unwrap();
    let token_client = TokenClient::new(&e, &token);

    e.ledger().set_timestamp(START + MONTH - 1);
    assert_eq!(client.release(&token), 0);

    e.ledger().set_timestamp(START + MONTH);
    assert_eq!(client.release(&token), 300);

    // Halfway through the second month: no new piece yet.
    e.ledger().set_timestamp(START + MONTH + MONTH / 2);
    assert_eq!(client.release(&token), 0);

    e.ledger().set_timestamp(START + 3 * MONTH);
    assert_eq!(client.release(&token), 600);
    assert_eq!(client.released(&token), 900);
    assert_eq!(token_client.balance(&receiver), 900);
}
