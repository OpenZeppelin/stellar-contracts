extern crate std;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, testutils::Address as _, Address, Env,
    String,
};
use stellar_tokens::rwa::RWAError;

use crate::contract::{RWATokenContract, RWATokenContractClient};

#[contract]
pub struct MockIdentityVerifier;

#[contractimpl]
impl MockIdentityVerifier {
    pub fn verify_identity(e: &Env, _account: Address) {
        let result = e.storage().persistent().get(&symbol_short!("id_ok")).unwrap_or(true);
        if !result {
            panic_with_error!(e, RWAError::IdentityVerificationFailed)
        }
    }
}

// Mock Compliance Contract
#[contract]
struct MockCompliance;

#[contractimpl]
impl MockCompliance {
    pub fn can_transfer(e: Env, _from: Address, _to: Address, _amount: i128) -> bool {
        e.storage().persistent().get(&symbol_short!("tx_ok")).unwrap_or(true)
    }

    pub fn can_create(e: Env, _to: Address, _amount: i128) -> bool {
        e.storage().persistent().get(&symbol_short!("mint_ok")).unwrap_or(true)
    }

    pub fn transferred(_e: Env, _from: Address, _to: Address, _amount: i128) {}

    pub fn created(_e: Env, _to: Address, _amount: i128) {}

    pub fn destroyed(_e: Env, _from: Address, _amount: i128) {}
}

fn create_client(e: &Env) -> (Address, Address, Address, RWATokenContractClient<'_>) {
    let admin = Address::generate(e);
    let compliance_officer = Address::generate(e);
    let recovery_agent = Address::generate(e);

    // Deploy mock contracts
    let identity_verifier = e.register(MockIdentityVerifier, ());
    let compliance_contract = e.register(MockCompliance, ());

    let contract_id = e.register(
        RWATokenContract,
        (
            &admin,
            &compliance_officer,
            &recovery_agent,
            &String::from_str(e, "RWA Token"),
            &String::from_str(e, "RWA"),
            &18u32,
        ),
    );
    let client = RWATokenContractClient::new(e, &contract_id);

    // Set the mock contracts on the RWA token
    client.set_identity_verifier(&identity_verifier, &admin);
    client.set_compliance(&compliance_contract, &admin);

    (admin, compliance_officer, recovery_agent, client)
}

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, _compliance_officer, _recovery_agent, client) = create_client(&e);

    // Test token metadata
    assert_eq!(client.name(), String::from_str(&e, "RWA Token"));
    assert_eq!(client.symbol(), String::from_str(&e, "RWA"));
    assert_eq!(client.decimals(), 18u32);

    // Test initial state
    assert!(!client.paused());
}

#[test]
fn test_pausable_functionality() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, _compliance_officer, _recovery_agent, client) = create_client(&e);

    // Initially not paused
    assert!(!client.paused());

    // Pause contract
    client.pause(&admin);
    assert!(client.paused());

    // Unpause contract
    client.unpause(&admin);
    assert!(!client.paused());
}

#[test]
fn test_minting_and_burning() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, _compliance_officer, _recovery_agent, client) = create_client(&e);
    let user = Address::generate(&e);
    let amount = 1000i128;

    // Initial balance should be 0
    assert_eq!(client.balance(&user), 0i128);

    // Mint tokens
    client.mint(&user, &amount, &admin);
    assert_eq!(client.balance(&user), amount);

    // Burn tokens
    client.burn(&user, &(amount / 2), &admin);
    assert_eq!(client.balance(&user), amount / 2);
}

#[test]
fn test_freezing_functionality() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, compliance_officer, _recovery_agent, client) = create_client(&e);
    let user = Address::generate(&e);
    let amount = 1000i128;

    // Mint some tokens first
    client.mint(&user, &amount, &admin);

    // Initially not frozen
    assert!(!client.is_frozen(&user));
    assert_eq!(client.get_frozen_tokens(&user), 0i128);

    // Freeze address
    client.set_address_frozen(&user, &true, &compliance_officer);
    assert!(client.is_frozen(&user));

    // Unfreeze address
    client.set_address_frozen(&user, &false, &compliance_officer);
    assert!(!client.is_frozen(&user));

    // Test partial token freezing
    let freeze_amount = 500i128;
    client.freeze_partial_tokens(&user, &freeze_amount, &compliance_officer);
    assert_eq!(client.get_frozen_tokens(&user), freeze_amount);

    // Unfreeze partial tokens
    client.unfreeze_partial_tokens(&user, &(freeze_amount / 2), &compliance_officer);
    assert_eq!(client.get_frozen_tokens(&user), freeze_amount / 2);
}

#[test]
fn test_forced_transfer() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, _compliance_officer, _recovery_agent, client) = create_client(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    // Mint tokens to sender
    client.mint(&from, &amount, &admin);
    assert_eq!(client.balance(&from), amount);
    assert_eq!(client.balance(&to), 0i128);

    // Perform forced transfer
    client.forced_transfer(&from, &to, &amount, &admin);
    assert_eq!(client.balance(&from), 0i128);
    assert_eq!(client.balance(&to), amount);
}
