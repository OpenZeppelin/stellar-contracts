use soroban_sdk::{contracttype, vec, Address, Env, Vec};

use super::{IDBalancePreSet, MaxBalanceSet};
use crate::rwa::compliance::{
    modules::{
        storage::{
            add_i128_or_panic, get_irs_client, hooks_verified, require_non_negative_amount,
            sub_i128_or_panic, verify_required_hooks,
        },
        MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    ComplianceHook,
};

#[contracttype]
#[derive(Clone)]
pub enum MaxBalanceStorageKey {
    /// Per-token maximum allowed identity balance.
    MaxBalance(Address),
    /// Balance keyed by (token, identity) — not by wallet.
    IDBalance(Address, Address),
}

// ################## RAW STORAGE ##################

/// Returns the per-identity balance cap for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_max_balance(e: &Env, token: &Address) -> i128 {
    let key = MaxBalanceStorageKey::MaxBalance(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the per-identity balance cap for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `value` - The maximum balance per identity.
pub fn set_max_balance(e: &Env, token: &Address, value: i128) {
    let key = MaxBalanceStorageKey::MaxBalance(token.clone());
    e.storage().persistent().set(&key, &value);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the tracked balance for `identity` on `token`, or `0` if not
/// set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
pub fn get_id_balance(e: &Env, token: &Address, identity: &Address) -> i128 {
    let key = MaxBalanceStorageKey::IDBalance(token.clone(), identity.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the tracked balance for `identity` on `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
/// * `balance` - The new balance value.
pub fn set_id_balance(e: &Env, token: &Address, identity: &Address, balance: i128) {
    let key = MaxBalanceStorageKey::IDBalance(token.clone(), identity.clone());
    e.storage().persistent().set(&key, &balance);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

// ################## HELPERS ##################

fn can_increase_identity_balance(
    e: &Env,
    token: &Address,
    identity: &Address,
    amount: i128,
) -> bool {
    if amount < 0 {
        return false;
    }

    let max = get_max_balance(e, token);
    if max == 0 {
        return true;
    }

    let current = get_id_balance(e, token, identity);
    add_i128_or_panic(e, current, amount) <= max
}

// ################## ACTIONS ##################

/// Validates, stores, and emits [`MaxBalanceSet`] for the given cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `max` - The maximum balance per identity.
pub fn configure_max_balance(e: &Env, token: &Address, max: i128) {
    require_non_negative_amount(e, max);
    set_max_balance(e, token, max);
    MaxBalanceSet { token: token.clone(), max_balance: max }.publish(e);
}

/// Pre-seeds the tracked balance for an identity and emits
/// [`IDBalancePreSet`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
/// * `balance` - The pre-seeded balance value.
pub fn pre_set_identity_balance(e: &Env, token: &Address, identity: &Address, balance: i128) {
    require_non_negative_amount(e, balance);
    set_id_balance(e, token, identity, balance);
    IDBalancePreSet { token: token.clone(), identity: identity.clone(), balance }.publish(e);
}

/// Pre-seeds tracked balances for multiple identities. Emits
/// [`IDBalancePreSet`] for each.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identities` - Identity addresses.
/// * `balances` - Corresponding balance values.
pub fn batch_pre_set_identity_balances(
    e: &Env,
    token: &Address,
    identities: &Vec<Address>,
    balances: &Vec<i128>,
) {
    assert!(
        identities.len() == balances.len(),
        "MaxBalanceModule: identities and balances length mismatch"
    );
    for i in 0..identities.len() {
        let id = identities.get(i).unwrap();
        let bal = balances.get(i).unwrap();
        require_non_negative_amount(e, bal);
        set_id_balance(e, token, &id, bal);
        IDBalancePreSet { token: token.clone(), identity: id, balance: bal }.publish(e);
    }
}

// ################## HOOK WIRING ##################

/// Returns the set of compliance hooks this module requires.
pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
    vec![
        e,
        ComplianceHook::CanTransfer,
        ComplianceHook::CanCreate,
        ComplianceHook::Transferred,
        ComplianceHook::Created,
        ComplianceHook::Destroyed,
    ]
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on all required hooks.
pub fn verify_hook_wiring(e: &Env) {
    verify_required_hooks(e, required_hooks(e));
}

// ################## COMPLIANCE HOOKS ##################

/// Updates identity balances after a transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `to` - The recipient address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
pub fn on_transfer(e: &Env, from: &Address, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let irs = get_irs_client(e, token);
    let from_id = irs.stored_identity(from);
    let to_id = irs.stored_identity(to);

    if from_id == to_id {
        return;
    }

    let from_balance = get_id_balance(e, token, &from_id);
    assert!(
        can_increase_identity_balance(e, token, &to_id, amount),
        "MaxBalanceModule: recipient identity balance exceeds max"
    );

    let to_balance = get_id_balance(e, token, &to_id);
    let new_to_balance = add_i128_or_panic(e, to_balance, amount);
    set_id_balance(e, token, &from_id, sub_i128_or_panic(e, from_balance, amount));
    set_id_balance(e, token, &to_id, new_to_balance);
}

/// Updates identity balance after a mint.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient address.
/// * `amount` - The minted amount.
/// * `token` - The token address.
pub fn on_created(e: &Env, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let irs = get_irs_client(e, token);
    let to_id = irs.stored_identity(to);

    assert!(
        can_increase_identity_balance(e, token, &to_id, amount),
        "MaxBalanceModule: recipient identity balance exceeds max after mint"
    );

    let current = get_id_balance(e, token, &to_id);
    let new_balance = add_i128_or_panic(e, current, amount);
    set_id_balance(e, token, &to_id, new_balance);
}

/// Updates identity balance after a burn.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The burner address.
/// * `amount` - The burned amount.
/// * `token` - The token address.
pub fn on_destroyed(e: &Env, from: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let irs = get_irs_client(e, token);
    let from_id = irs.stored_identity(from);

    let current = get_id_balance(e, token, &from_id);
    set_id_balance(e, token, &from_id, sub_i128_or_panic(e, current, amount));
}

/// Checks whether a transfer would exceed the recipient identity's
/// balance cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `to` - The recipient address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
pub fn can_transfer(e: &Env, from: &Address, to: &Address, amount: i128, token: &Address) -> bool {
    assert!(
        hooks_verified(e),
        "MaxBalanceModule: not armed — call verify_hook_wiring() after wiring hooks [CanTransfer, \
         CanCreate, Transferred, Created, Destroyed]"
    );
    if amount < 0 {
        return false;
    }
    let irs = get_irs_client(e, token);
    let from_id = irs.stored_identity(from);
    let to_id = irs.stored_identity(to);

    if from_id == to_id {
        return true;
    }

    can_increase_identity_balance(e, token, &to_id, amount)
}

/// Checks whether a mint would exceed the recipient identity's balance
/// cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient address.
/// * `amount` - The mint amount.
/// * `token` - The token address.
pub fn can_create(e: &Env, to: &Address, amount: i128, token: &Address) -> bool {
    assert!(
        hooks_verified(e),
        "MaxBalanceModule: not armed — call verify_hook_wiring() after wiring hooks [CanTransfer, \
         CanCreate, Transferred, Created, Destroyed]"
    );
    if amount < 0 {
        return false;
    }
    let irs = get_irs_client(e, token);
    let to_id = irs.stored_identity(to);
    can_increase_identity_balance(e, token, &to_id, amount)
}
