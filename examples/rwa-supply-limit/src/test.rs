extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{SupplyLimitContract, SupplyLimitContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> SupplyLimitContractClient<'a> {
    let address = e.register(SupplyLimitContract, (admin,));
    SupplyLimitContractClient::new(e, &address)
}

#[test]
fn set_and_get_supply_limit_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    assert_eq!(client.get_supply_limit(&token), 0);

    client.set_supply_limit(&token, &1_000_i128);
    assert_eq!(client.get_supply_limit(&token), 1_000);
    assert_eq!(client.get_supply_count(&token), 0);
}

#[test]
fn name_and_compliance_address_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let client = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "SupplyLimitModule"));

    client.set_compliance_address(&compliance);
    assert_eq!(client.get_compliance_address(), compliance);
}

#[test]
fn set_supply_limit_uses_admin_auth_before_compliance_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_supply_limit(&token, &100_i128);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_supply_limit_uses_compliance_auth_after_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    client.set_supply_limit(&token, &100_i128);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &compliance);
}

#[test]
fn can_create_reflects_running_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    client.set_supply_limit(&token, &100_i128);

    assert!(client.can_create(&to, &100_i128, &token));
    assert!(!client.can_create(&to, &101_i128, &token));

    client.on_created(&to, &70_i128, &token);
    assert!(client.can_create(&to, &30_i128, &token));
    assert!(!client.can_create(&to, &31_i128, &token));
}

#[test]
fn can_transfer_is_always_true() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    assert!(client.can_transfer(&from, &to, &9_999_i128, &token));
}

#[test]
fn on_created_and_on_destroyed_track_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    client.set_supply_limit(&token, &200_i128);

    client.on_created(&to, &120_i128, &token);
    assert_eq!(client.get_supply_count(&token), 120);

    client.on_destroyed(&to, &50_i128, &token);
    assert_eq!(client.get_supply_count(&token), 70);
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn on_created_panics_when_exceeding_limit() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    client.set_supply_limit(&token, &50_i128);

    client.on_created(&to, &51_i128, &token);
}
