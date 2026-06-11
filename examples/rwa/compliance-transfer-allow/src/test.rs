extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};
use stellar_tokens::rwa::compliance::{AccountSnapshot, TransferKind};

use crate::contract::{TransferAllowContract, TransferAllowContractClient};

/// This module ignores balance and frozen amounts, so they are left at zero.
fn snap(address: &Address) -> AccountSnapshot {
    AccountSnapshot { address: address.clone(), balance: 0, frozen: 0 }
}

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> TransferAllowContractClient<'a> {
    let address = e.register(TransferAllowContract, (admin, manager));
    TransferAllowContractClient::new(e, &address)
}

#[test]
fn allow_and_disallow_user_round_trip() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert!(!client.is_user_allowed(&token, &user));

    client.allow_user(&token, &user, &manager);
    assert!(client.is_user_allowed(&token, &user));

    client.disallow_user(&token, &user, &manager);
    assert!(!client.is_user_allowed(&token, &user));
}

#[test]
fn batch_allow_and_disallow_users_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let user_a = Address::generate(&e);
    let user_b = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.batch_allow_users(&token, &vec![&e, user_a.clone(), user_b.clone()], &manager);
    assert!(client.is_user_allowed(&token, &user_a));
    assert!(client.is_user_allowed(&token, &user_b));

    client.batch_disallow_users(&token, &vec![&e, user_a.clone(), user_b.clone()], &manager);
    assert!(!client.is_user_allowed(&token, &user_a));
    assert!(!client.is_user_allowed(&token, &user_b));
}

#[test]
fn on_transfer_checks_sender_then_recipient() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let allowed = Address::generate(&e);
    let other = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.allow_user(&token, &allowed, &manager);

    // Allowlisted sender, then allowlisted recipient: both pass.
    client.on_transfer(&snap(&allowed), &snap(&other), &10_i128, &TransferKind::Standard, &token);
    client.on_transfer(&snap(&other), &snap(&allowed), &10_i128, &TransferKind::Standard, &token);
}

#[test]
#[should_panic(expected = "Error(Contract, #406)")]
fn on_transfer_panics_when_neither_party_allowlisted() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let other = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.on_transfer(&snap(&other), &snap(&other), &10_i128, &TransferKind::Standard, &token);
}

#[test]
fn on_transfer_forced_is_exempt_from_policy() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let other = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // Neither party is allowlisted: a standard transfer would panic, but a
    // forced one passes through untouched.
    client.on_transfer(&snap(&other), &snap(&other), &10_i128, &TransferKind::Forced, &token);
}

#[test]
fn on_created_always_allows() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // Mints are not restricted by this module: the hook is a no-op even
    // for a recipient that is not allowlisted.
    client.on_created(&snap(&to), &10_i128, &token);
}

#[test]
fn name_returns_module_identifier() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.name(), String::from_str(&e, "TransferAllowModule"));
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
fn allow_user_requires_manager_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.allow_user(&token, &user, &manager);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &manager);
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
