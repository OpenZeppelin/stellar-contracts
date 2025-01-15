use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::fungible::{emit_approve, emit_transfer, FungibleTokenError};

#[contracttype]
struct AllowanceDataKey {
    owner: Address,
    spender: Address,
}

#[contracttype]
pub struct AllowanceValue {
    pub value: i128,
    pub live_until_ledger: u32,
}

#[contracttype]
enum StorageDataKey {
    TotalSupply,
    Balance(Address),
    Allowance(AllowanceDataKey),
}

/// Returns the total number of tokens in circulation. If no supply is recorded,
/// it defaults to `0`.
///
/// # Arguments
/// * `e` - Access to the Soroban environment.
pub fn total_supply(e: &Env) -> i128 {
    e.storage().instance().get(&StorageDataKey::TotalSupply).unwrap_or(0)
}

/// Returns the number of tokens held by an `account`. Defaults to `0` if no
/// balance is stored.
///
/// # Arguments
/// * `e` - Access to the Soroban environment.
/// * `account` - The address for which the balance is being queried.
pub fn balance(e: &Env, account: &Address) -> i128 {
    // TODO: extend persistent?
    e.storage().persistent().get(&StorageDataKey::Balance(account.clone())).unwrap_or(0)
}

/// Returns the number of tokens a `spender` is allowed to spend on behalf of an
/// `owner` through [`transfer_from()`] and the ledger number at which this
/// allowance expires. Both values default to `0`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
///
/// # Notes
///
/// * An allowance entry where `live_until_ledger` is lower than the current
///   ledger number is treated as an allowance with value `0`.
pub fn allowance(e: &Env, owner: &Address, spender: &Address) -> AllowanceValue {
    let key = AllowanceDataKey { owner: owner.clone(), spender: spender.clone() };
    let default = AllowanceValue { value: 0, live_until_ledger: 0 };
    e.storage().temporary().get(&StorageDataKey::Allowance(key)).unwrap_or(default)
}

/// Creates a `value` amount of tokens and assigns them to an `account`. Updates
/// the total supply accordingly.
///
/// # Arguments
/// * `e` - Access to the Soroban environment.
/// * `account` - The address receiving the new tokens.
/// * `value` - The amount of tokens to mint.
///
/// # Events
/// TODO
pub fn mint(e: &Env, account: &Address, value: i128) {
    update(e, None, Some(account), value)
    // TODO: emit_mint
}

/// Destroys a `value` amount of tokens from an `account`. Updates the total supply accordingly.
///
/// # Arguments
/// * `e` - Access to the Soroban environment.
/// * `account` - The address whose tokens are destroyed.
/// * `value` - The amount of tokens to burn.
///
/// # Events
/// TODO
pub fn burn(e: &Env, account: &Address, value: i128) {
    update(e, Some(account), None, value)
    // TODO: emit_burn
}

pub fn approve(e: &Env, owner: &Address, spender: &Address, value: i128, live_until_ledger: u32) {
    owner.require_auth();
    set_allowance(e, owner, spender, value, live_until_ledger, true);
}

pub fn transfer(e: &Env, owner: &Address, to: &Address, value: i128) {
    owner.require_auth();
    do_transfer(e, owner, to, value);
}

pub fn transfer_from(e: &Env, spender: &Address, owner: &Address, to: &Address, value: i128) {
    spender.require_auth();
    spend_allowance(e, owner, spender, value);
    do_transfer(e, owner, to, value);
}

pub fn do_transfer(e: &Env, owner: &Address, to: &Address, value: i128) {
    update(e, Some(owner), Some(to), value);

    emit_transfer(e, owner, to, value);
}

pub fn set_allowance(
    e: &Env,
    owner: &Address,
    spender: &Address,
    value: i128,
    live_until_ledger: u32,
    emit: bool,
) {
    let allowance = AllowanceValue { value, live_until_ledger };

    if value > 0 && live_until_ledger < e.ledger().sequence() {
        panic_with_error!(e, FungibleTokenError::InvalidLiveUntilLedger);
    }

    let key = StorageDataKey::Allowance(AllowanceDataKey {
        owner: owner.clone(),
        spender: spender.clone(),
    });
    e.storage().temporary().set(&key, &allowance);

    if value > 0 {
        // NOTE: can't underflow because of the check above.
        let live_for = live_until_ledger - e.ledger().sequence();

        e.storage().temporary().extend_ttl(&key, live_for, live_for)
    }

    if emit {
        emit_approve(e, owner, spender, value, live_until_ledger);
    }
}

pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, value: i128) {
    if let Some(account) = from {
        let mut from_balance = balance(e, account);
        if from_balance < value {
            panic_with_error!(e, FungibleTokenError::InsufficientBalance);
        }
        // NOTE: can't underflow because of the check above.
        from_balance -= value;
        e.storage().persistent().set(&StorageDataKey::Balance(account.clone()), &from_balance);
    } else {
        // TODO: check for total_supply for compatibilty with Token Interface
        let mut total_supply = total_supply(e);
        total_supply = total_supply.checked_add(value).expect("total_supply overflow");
        e.storage().instance().set(&StorageDataKey::TotalSupply, &total_supply);
    }

    if let Some(account) = to {
        let mut to_balance = balance(e, account);
        to_balance = to_balance.checked_add(value).expect("to_balance overflow");
        e.storage().persistent().set(&StorageDataKey::Balance(account.clone()), &to_balance);
    } else {
        // TODO: check for total_supply for compatibilty with Token Interface
        let mut total_supply = total_supply(e);
        total_supply = total_supply.checked_sub(value).expect("total_supply underflow");
        e.storage().instance().set(&StorageDataKey::TotalSupply, &total_supply);
    }
}

pub fn spend_allowance(e: &Env, owner: &Address, spender: &Address, value: i128) {
    let allowance = allowance(e, owner, spender);

    if allowance.value < value {
        panic_with_error!(e, FungibleTokenError::InsufficientAllowance);
    }

    if value > 0 {
        set_allowance(
            e,
            owner,
            spender,
            allowance.value - value,
            allowance.live_until_ledger,
            false,
        );
    }
}
