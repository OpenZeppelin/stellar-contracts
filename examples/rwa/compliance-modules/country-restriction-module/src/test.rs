extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{CountryRestrictionModule, CountryRestrictionModuleClient};

fn create_client(e: &Env) -> (Address, CountryRestrictionModuleClient<'_>) {
    let admin = Address::generate(e);
    let address = e.register(CountryRestrictionModule, ());
    let client = CountryRestrictionModuleClient::new(e, &address);
    (admin, client)
}

#[test]
fn country_module_initialization_() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);

    assert_eq!(client.name(), String::from_str(&e, "Country Restriction Module"));
}

#[test]
fn test_transfer_permissions() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    // Currently allows all transfers (placeholder implementation)
    assert!(client.can_transfer(&from, &to, &amount));
}

#[test]
fn test_creation_permissions() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    // Currently allows all minting (placeholder implementation)
    assert!(client.can_create(&to, &amount));
}

#[test]
fn test_hook_methods() {
    let e = Env::default();
    e.mock_all_auths();

    let (_admin, client) = create_client(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let amount = 1000i128;

    // These methods should not panic (they're tracking methods)
    client.on_transfer(&from, &to, &amount);
    client.on_created(&to, &amount);
    client.on_destroyed(&from, &amount);
}
