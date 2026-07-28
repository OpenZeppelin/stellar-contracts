extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, String,
};
use stellar_tokens::rwa::compliance::{
    modules::initial_lockup_period::LockedTokens, AccountSnapshot, TransferKind,
};

use crate::contract::{InitialLockupPeriodContract, InitialLockupPeriodContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> InitialLockupPeriodContractClient<'a> {
    let address = e.register(InitialLockupPeriodContract, (admin, manager));
    InitialLockupPeriodContractClient::new(e, &address)
}

/// Snapshot carrying the wallet's token balance. The module reads `balance`
/// on the transfer and burn hooks; `frozen` is left at zero.
fn snap(address: &Address, balance: i128) -> AccountSnapshot {
    AccountSnapshot { address: address.clone(), balance, frozen: 0 }
}

#[test]
fn set_and_get_lockup_period_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.get_lockup_period(&token), 0);

    client.set_lockup_period(&token, &17_280_u32, &manager);
    assert_eq!(client.get_lockup_period(&token), 17_280);
}

#[test]
fn on_created_locks_minted_tokens() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_lockup_period(&token, &100_u32, &manager);

    client.on_created(&snap(&wallet, 0), &80_i128, &token);

    let details = client.get_locked_details(&token, &wallet);
    assert_eq!(details.total_locked, 80);
    assert_eq!(details.locks, vec![&e, LockedTokens { amount: 80, release_ledger: 100 }]);
    assert_eq!(client.get_locked_amount(&token, &wallet), 80);
}

#[test]
fn transfers_are_limited_to_unlocked_tokens() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_lockup_period(&token, &100_u32, &manager);
    client.on_created(&snap(&from, 0), &80_i128, &token);

    // After the release time, the locked tokens become spendable.
    e.ledger().with_mut(|li| li.sequence_number = 100);
    client.on_transfer(&snap(&from, 80), &snap(&to, 0), &30_i128, &TransferKind::Standard, &token);
    // The 80-token lock had released by ledger 100; spending 30 consumed 30 of
    // it, leaving a 50-token (already-released) entry on the books.
    assert_eq!(client.get_locked_details(&token, &from).total_locked, 50);
    // Tokens received through transfers are never locked.
    assert_eq!(client.get_locked_details(&token, &to).total_locked, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #399)")]
fn on_transfer_panics_when_tokens_still_locked() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_lockup_period(&token, &100_u32, &manager);
    client.on_created(&snap(&from, 0), &80_i128, &token);

    client.on_transfer(&snap(&from, 80), &snap(&to, 0), &1_i128, &TransferKind::Standard, &token);
}

#[test]
fn on_destroyed_consumes_expired_locks() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_compliance_address(&token, &compliance, &admin);
    client.set_lockup_period(&token, &100_u32, &manager);
    client.on_created(&snap(&wallet, 0), &100_i128, &token);

    e.ledger().with_mut(|li| li.sequence_number = 100);
    client.on_destroyed(&snap(&wallet, 100), &60_i128, &token);

    assert_eq!(client.get_locked_details(&token, &wallet).total_locked, 40);
}

#[test]
fn preset_locks_seeds_wallet() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let locks = vec![&e, LockedTokens { amount: 40, release_ledger: 500 }];
    client.preset_locks(&token, &wallet, &locks, &manager);

    assert_eq!(client.get_locked_details(&token, &wallet).total_locked, 40);
    assert_eq!(client.get_locked_amount(&token, &wallet), 40);
}

#[test]
#[should_panic(expected = "Error(Contract, #395)")]
fn preset_locks_panics_after_completed() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let wallet = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.mark_preset_completed(&token, &manager);
    client.preset_locks(&token, &wallet, &vec![&e], &manager);
}

#[test]
fn mark_preset_completed_flips_flag() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert!(!client.is_preset_completed(&token));
    client.mark_preset_completed(&token, &manager);
    assert!(client.is_preset_completed(&token));
}

#[test]
fn name_returns_module_identifier() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    assert_eq!(client.name(), String::from_str(&e, "InitialLockupPeriodModule"));
}

#[test]
fn set_lockup_period_requires_manager_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let token = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.set_lockup_period(&token, &100_u32, &manager);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &manager);
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
