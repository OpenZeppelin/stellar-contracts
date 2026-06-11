extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use stellar_tokens::rwa::compliance::{AccountSnapshot, TransferKind};

use crate::contract::{SupplyLimitContract, SupplyLimitContractClient};

/// This module ignores balance and frozen amounts, so they are left at zero.
fn snap(address: &Address) -> AccountSnapshot {
    AccountSnapshot { address: address.clone(), balance: 0, frozen: 0 }
}

fn create_client<'a>(e: &Env, admin: &Address, manager: &Address) -> SupplyLimitContractClient<'a> {
    let address = e.register(SupplyLimitContract, (admin, manager));
    SupplyLimitContractClient::new(e, &address)
}

#[test]
fn set_and_get_supply_limit_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.get_supply_limit(&token), 0);

    client.set_supply_limit(&token, &1_000_i128, &manager);
    assert_eq!(client.get_supply_limit(&token), 1_000);
    assert_eq!(client.get_supply_count(&token), 0);
}

#[test]
fn name_returns_module_identifier() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.name(), String::from_str(&e, "SupplyLimitModule"));
}

#[test]
fn set_and_get_compliance_address_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let compliance = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);

    assert_eq!(client.get_compliance_address(&token), compliance);
}

#[test]
fn set_compliance_address_requires_admin_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let compliance = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn set_supply_limit_requires_manager_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_supply_limit(&token, &100_i128, &manager);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &manager);
}

#[test]
fn on_created_allows_minting_up_to_the_limit() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_supply_limit(&token, &100_i128, &manager);

    client.on_created(&snap(&to), &70_i128, &token);
    // The remaining headroom can still be minted exactly.
    client.on_created(&snap(&to), &30_i128, &token);

    assert_eq!(client.get_supply_count(&token), 100);
}

#[test]
fn on_transfer_is_a_noop() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // Transfers do not affect the tracked supply.
    client.on_transfer(&snap(&from), &snap(&to), &9_999_i128, &TransferKind::Standard, &token);
    assert_eq!(client.get_supply_count(&token), 0);
}

#[test]
fn on_created_and_on_destroyed_track_supply() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_supply_limit(&token, &200_i128, &manager);

    client.on_created(&snap(&to), &120_i128, &token);
    assert_eq!(client.get_supply_count(&token), 120);

    client.on_destroyed(&snap(&to), &50_i128, &token);
    assert_eq!(client.get_supply_count(&token), 70);
}

#[test]
#[should_panic(expected = "Error(Contract, #394)")]
fn on_created_panics_when_exceeding_limit() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let to = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_supply_limit(&token, &50_i128, &manager);

    client.on_created(&snap(&to), &51_i128, &token);
}
