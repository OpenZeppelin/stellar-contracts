use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::fungible::{emit_approve, emit_transfer, FungibleTokenError};

// Same values as in Stellar Asset Contract (SAC) implementation:
// https://github.com/stellar/rs-soroban-env/blob/main/soroban-env-host/src/builtin_contracts/stellar_asset_contract/storage_types.rs
pub const DAY_IN_LEDGERS: u32 = 17280;

pub const INSTANCE_EXTEND_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const BALANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_TTL_THRESHOLD: u32 = BALANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Storage key that maps to [`AllowanceData`]
#[contracttype]
pub struct AllowanceKey {
    pub owner: Address,
    pub spender: Address,
}

/// Storage container for the amount of tokens for which an allowance is granted
/// and the ledger number at which this allowance expires.
#[contracttype]
pub struct AllowanceData {
    pub amount: i128,
    pub live_until_ledger: u32,
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    TotalSupply,
    Balance(Address),
    Allowance(AllowanceKey),
}

// ################## QUERY STATE ##################

/// Returns the total amount of tokens in circulation. If no supply is recorded,
/// it defaults to `0`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn total_supply(e: &Env) -> i128 {
    e.storage().instance().extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);
    e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0)
}

/// Returns the amount of tokens held by `account`. Defaults to `0` if no
/// balance is stored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address for which the balance is being queried.
pub fn balance(e: &Env, account: &Address) -> i128 {
    let key = StorageKey::Balance(account.clone());
    if let Some(balance) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);
        balance
    } else {
        0
    }
}

/// Returns the amount of tokens a `spender` is allowed to spend on behalf of an
/// `owner` and the ledger number at which this allowance expires. Both values
/// default to `0`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
///
/// # Notes
///
/// Attention is required when `live_until_ledger` is less than the current
/// ledger number, as this indicates the entry has expired. In such cases, the
/// allowance should be treated as `0`.
pub fn allowance_data(e: &Env, owner: &Address, spender: &Address) -> AllowanceData {
    let key = AllowanceKey { owner: owner.clone(), spender: spender.clone() };
    let default = AllowanceData { amount: 0, live_until_ledger: 0 };
    e.storage().temporary().get(&StorageKey::Allowance(key)).unwrap_or(default)
}

/// Returns the amount of tokens a `spender` is allowed to spend on behalf of an
/// `owner`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
///
/// # Notes
///
/// An allowance entry where `live_until_ledger` is less than the current
/// ledger number is treated as an allowance with amount `0`.
pub fn allowance(e: &Env, owner: &Address, spender: &Address) -> i128 {
    let allowance = allowance_data(e, owner, spender);

    if allowance.amount > 0 && allowance.live_until_ledger < e.ledger().sequence() {
        return 0;
    }

    allowance.amount
}

// ################## CHANGE STATE ##################

/// Sets the amount of tokens a `spender` is allowed to spend on behalf of an
/// `owner`. Overrides any existing allowance set between `spender` and `owner`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
///
/// # Errors
///
/// * [`FungibleTokenError::InvalidLiveUntilLedger`] - Occurs when attempting to
///   set `live_until_ledger` that is less than the current ledger number and
///   greater than `0`.
///
/// # Events
///
/// * topics - `["approve", from: Address, spender: Address]`
/// * data - `[amount: i128, live_until_ledger: u32]`
///
/// # Notes
///
/// Authorization for `owner` is required.
pub fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
    owner.require_auth();
    set_allowance(e, owner, spender, amount, live_until_ledger, true);
}

/// Sets the amount of tokens a `spender` is allowed to spend on behalf of an
/// `owner`. Overrides any existing allowance set between `spender` and `owner`.
/// Variant of [`approve()`] that doesn't handle authorization, but controls
/// event emission. That can be useful in operations like spending allowance
/// during [`transfer_from()`].
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
/// * `emit` - A flag to enable or disable event emission.
///
/// # Errors
///
/// * [`FungibleTokenError::InvalidLiveUntilLedger`] - Occurs when attempting to
///   set `live_until_ledger` that is less than the current ledger number and
///   greater than `0`.
/// * [`FungibleTokenError::LessThanZero`] - Occurs when `amount < 0`.
///
/// # Events
///
/// Emits an event if `emit` is `true`.
/// * topics - `["approve", from: Address, spender: Address]`
/// * data - `[amount: i128, live_until_ledger: u32]`
///
/// # Notes
///
/// No authorization is required.
pub fn set_allowance(
    e: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
    live_until_ledger: u32,
    emit: bool,
) {
    if amount < 0 {
        panic_with_error!(e, FungibleTokenError::LessThanZero)
    }

    let allowance = AllowanceData { amount, live_until_ledger };

    if amount > 0 && live_until_ledger < e.ledger().sequence() {
        panic_with_error!(e, FungibleTokenError::InvalidLiveUntilLedger);
    }

    let key =
        StorageKey::Allowance(AllowanceKey { owner: owner.clone(), spender: spender.clone() });
    e.storage().temporary().set(&key, &allowance);

    if amount > 0 {
        // NOTE: can't underflow because of the check above.
        let live_for = live_until_ledger - e.ledger().sequence();

        e.storage().temporary().extend_ttl(&key, live_for, live_for)
    }

    if emit {
        emit_approve(e, owner, spender, amount, live_until_ledger);
    }
}

/// Deducts the amount of tokens a `spender` is allowed to spend on behalf of an
/// `owner`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens to be deducted from `spender`s allowance.
///
/// # Errors
///
/// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
///   transfer more tokens than `spender` current allowance.
///
/// # Notes
///
/// No authorization is required.
pub fn spend_allowance(e: &Env, owner: &Address, spender: &Address, amount: i128) {
    let allowance = allowance_data(e, owner, spender);

    if allowance.amount < amount {
        panic_with_error!(e, FungibleTokenError::InsufficientAllowance);
    }

    if amount > 0 {
        set_allowance(
            e,
            owner,
            spender,
            allowance.amount - amount,
            allowance.live_until_ledger,
            false,
        );
    }
}

/// Transfers `amount` of tokens from `from` to `to`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Errors
///
/// * [`FungibleTokenError::InsufficientBalance`] - When attempting to transfer
///   more tokens than `from` current balance.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// Authorization for `from` is required.
pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    from.require_auth();
    do_transfer(e, from, to, amount);
}

/// Transfers `amount` of tokens from `from` to `to` using the
/// allowance mechanism. `amount` is then deducted from `spender`s allowance.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `spender` - The address authorizing the transfer, and having its allowance
///   consumed during the transfer.
/// * `from` - The address holding the tokens which will be transferred.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Errors
///
/// * [`FungibleTokenError::InsufficientBalance`] - When attempting to transfer
///   more tokens than `from` current balance.
/// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
///   transfer more tokens than `spender` current allowance.
///
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// Authorization for `spender` is required.
pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
    spender.require_auth();
    spend_allowance(e, from, spender, amount);
    do_transfer(e, from, to, amount);
}

/// Equivalent to [`transfer()`] but doesn't handle authorization.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Errors
///
/// * [`FungibleTokenError::InsufficientBalance`] - When attempting to transfer
///   more tokens than `from` current balance.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// No authorization is required.
pub fn do_transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    update(e, Some(from), Some(to), amount);

    emit_transfer(e, from, to, amount);
}

/// Transfers `amount` of tokens from `from` to `to` or alternatively
/// mints (or burns) tokens if `from` (or `to`) is `None`. Updates the total
/// supply accordingly.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Errors
///
/// * [`FungibleTokenError::InsufficientBalance`] - When attempting to transfer
///   more tokens than `from` current balance.
/// * [`FungibleTokenError::LessThanOrEqualToZero`] - When `amount <= 0`.
/// * [`FungibleTokenError::MathOverflow`] - When `total_supply` overflows.
///
/// # Notes
///
/// No authorization is required.
pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: i128) {
    if amount <= 0 {
        panic_with_error!(e, FungibleTokenError::LessThanOrEqualToZero)
    }

    if let Some(account) = from {
        let mut from_balance = balance(e, account);
        if from_balance < amount {
            panic_with_error!(e, FungibleTokenError::InsufficientBalance);
        }
        // NOTE: can't underflow because of the check above.
        from_balance -= amount;
        e.storage().persistent().set(&StorageKey::Balance(account.clone()), &from_balance);
    } else {
        let mut total_supply = total_supply(e);
        total_supply = match total_supply.checked_add(amount) {
            Some(num) => num,
            _ => panic_with_error!(e, FungibleTokenError::MathOverflow),
        };
        e.storage().instance().set(&StorageKey::TotalSupply, &total_supply);
    }

    if let Some(account) = to {
        // NOTE: can't overflow because balance + amoount is at most total_supply.
        let to_balance = balance(e, account) + amount;
        e.storage().persistent().set(&StorageKey::Balance(account.clone()), &to_balance);
    } else {
        // NOTE: can't overflow because amount <= total_supply or amount <= from_balance
        // <= total_supply.
        let total_supply = total_supply(e) - amount;
        e.storage().instance().set(&StorageKey::TotalSupply, &total_supply);
    }
}
