use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::compliance::{
    modules::{
        max_balance::{emit_id_balance_preset, emit_max_balance_set, emit_preset_completed},
        storage::{
            add_i128_or_panic, get_irs_client, require_non_negative_amount, sub_i128_or_panic,
        },
        ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    TransferKind,
};

#[contracttype]
#[derive(Clone)]
pub enum MaxBalanceStorageKey {
    /// Per-token cap on the aggregate balance any single identity may hold.
    MaxBalance(Address),
    /// Per-(token, identity) aggregate balance tracked by this module.
    IdBalance(Address, Address),
    /// Per-token flag indicating that the preset migration phase is finalized.
    PresetCompleted(Address),
}

// ################## QUERY STATE ##################

/// Returns the configured per-identity maximum balance for `token`. Returns
/// `0` when no limit has been configured, which blocks all transfers and
/// mints until [`set_max_balance`] is called.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_max_balance(e: &Env, token: &Address) -> i128 {
    let key = MaxBalanceStorageKey::MaxBalance(token.clone());
    if let Some(value) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        0
    }
}

/// Returns the aggregate balance tracked for `identity` under `token`.
/// Returns `0` when no entry exists.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity (on-chain ID) address.
pub fn get_id_balance(e: &Env, token: &Address, identity: &Address) -> i128 {
    let key = MaxBalanceStorageKey::IdBalance(token.clone(), identity.clone());
    if let Some(value) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        0
    }
}

/// Returns `true` when the preset phase for `token` has been finalized,
/// blocking any further preset writes.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn is_preset_completed(e: &Env, token: &Address) -> bool {
    let key = MaxBalanceStorageKey::PresetCompleted(token.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Resolves `account` to its identity via the token's configured IRS and
/// returns the identity's aggregate balance.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `account` - The wallet address to resolve.
///
/// # Errors
///
/// * refer to [`get_irs_client`] errors.
pub fn get_id_balance_of(e: &Env, token: &Address, account: &Address) -> i128 {
    let identity = get_irs_client(e, token).stored_identity(account);
    get_id_balance(e, token, &identity)
}

// ################## CHANGE STATE ##################

/// Sets the per-identity maximum balance for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `max` - The maximum aggregate balance any identity may hold.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `max` is negative.
///
/// # Events
///
/// * topics - `["max_balance_set", token: Address]`
/// * data - `[max: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_max_balance(e: &Env, token: &Address, max: i128) {
    require_non_negative_amount(e, max);
    let key = MaxBalanceStorageKey::MaxBalance(token.clone());
    e.storage().persistent().set(&key, &max);
    emit_max_balance_set(e, token, max);
}

/// Pre-seeds the tracked aggregate balance for `identity` under `token`.
/// Useful when registering this module on a token that already has live
/// balances.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity (on-chain ID) address.
/// * `balance` - The balance to record.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `balance` is negative.
/// * [`ComplianceModuleError::PresetAlreadyCompleted`] - When the preset phase
///   has already been finalized.
///
/// # Events
///
/// * topics - `["id_balance_preset", token: Address, identity: Address]`
/// * data - `[balance: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn preset_id_balance(e: &Env, token: &Address, identity: &Address, balance: i128) {
    require_non_negative_amount(e, balance);
    if is_preset_completed(e, token) {
        panic_with_error!(e, ComplianceModuleError::PresetAlreadyCompleted);
    }
    set_id_balance(e, token, identity, balance);
    emit_id_balance_preset(e, token, identity, balance);
}

/// Pre-seeds aggregate balances for multiple identities in a single call.
/// `identities` and `balances` must have the same length.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identities` - The identity addresses to pre-seed.
/// * `balances` - The balances aligned positionally with `identities`.
///
/// # Errors
///
/// * [`ComplianceModuleError::BatchSizeMismatch`] - When `identities` and
///   `balances` have different lengths.
/// * refer to [`preset_id_balance`] errors.
///
/// # Events
///
/// For each entry:
/// * topics - `["id_balance_preset", token: Address, identity: Address]`
/// * data - `[balance: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
///
/// Each `(token, identity)` pair lives in its own persistent entry, so the
/// caller must size `identities` to stay within the per-transaction network
/// limits — see <https://lab.stellar.org/network-limits>.
pub fn batch_preset_id_balances(
    e: &Env,
    token: &Address,
    identities: &Vec<Address>,
    balances: &Vec<i128>,
) {
    if identities.len() != balances.len() {
        panic_with_error!(e, ComplianceModuleError::BatchSizeMismatch);
    }
    for (identity, balance) in identities.iter().zip(balances.iter()) {
        preset_id_balance(e, token, &identity, balance);
    }
}

/// Finalizes the preset phase for `token`. After this call,
/// invoking [`preset_id_balance`] and [`batch_preset_id_balances`] will
/// panic with [`ComplianceModuleError::PresetAlreadyCompleted`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
///
/// # Events
///
/// * topics - `["preset_completed", token: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn mark_preset_completed(e: &Env, token: &Address) {
    let key = MaxBalanceStorageKey::PresetCompleted(token.clone());
    e.storage().persistent().set(&key, &());
    emit_preset_completed(e, token);
}

/// Records a transfer between two wallets: debits the sender's identity and
/// credits the recipient's identity. Panics if crediting the recipient would
/// exceed the per-identity cap, unless the transfer is forced: a privileged
/// operation (forced transfer, recovery) is exempt from the cap, but the
/// aggregate balances are still updated so the books stay true. Transfers
/// where `from` and `to` resolve to the same identity are no-ops: no debit,
/// no credit, no cap check.
///
/// When `from`'s live identity entry has already been removed by account
/// recovery, the sender is resolved through the IRS recovery record rather than
/// reverting on the source lookup, so a forced/recovery transfer still updates
/// the books. Once `from` recovered to `to`, both sides resolve to the same
/// identity and the movement is a same-identity no-op.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender wallet.
/// * `to` - The recipient wallet.
/// * `amount` - The transferred amount.
/// * `kind` - Who initiated the transfer and under what authority.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MaxBalanceExceeded`] - When the recipient's new
///   aggregate balance would exceed the configured maximum and the transfer is
///   not forced.
/// * [`ComplianceModuleError::MathOverflow`] - When the recipient's credit
///   addition overflows.
/// * [`ComplianceModuleError::MathUnderflow`] - When the sender's debit
///   subtraction underflows.
/// * refer to [`get_irs_client`] errors.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_transfer(
    e: &Env,
    from: &Address,
    to: &Address,
    amount: i128,
    kind: &TransferKind,
    token: &Address,
) {
    require_non_negative_amount(e, amount);

    let irs = get_irs_client(e, token);
    let id_to = irs.stored_identity(to);

    // `from`'s live identity entry may have been removed by account recovery
    // (`recover_identity` moves the identity to the new wallet and deletes the
    // old mapping). When the live lookup fails with a contract error, resolve
    // the sender through the recovery record so a forced/recovery transfer stays
    // bookkeeping-correct instead of reverting here. A sender that was never
    // registered (failed lookup, no recovery record) re-raises the IRS error.
    let id_from = match irs.try_stored_identity(from) {
        Ok(Ok(id)) => id,
        Err(Ok(err)) => match irs.get_recovered_to(from) {
            Some(recovered_wallet) => irs.stored_identity(&recovered_wallet),
            None => panic_with_error!(e, err),
        },
        // A conversion failure or host-level invoke error is unreachable with a
        // correctly configured IRS; terminate by re-issuing the call.
        _ => irs.stored_identity(from),
    };

    if id_from == id_to {
        return;
    }

    debit_identity(e, token, &id_from, amount);
    if *kind == TransferKind::Forced {
        credit_identity_unchecked(e, token, &id_to, amount);
    } else {
        credit_identity(e, token, &id_to, amount);
    }
}

/// Records a mint to `to`: credits the recipient's identity. Panics if the
/// new aggregate balance would exceed the per-identity cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The recipient wallet.
/// * `amount` - The minted amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MaxBalanceExceeded`] - When the recipient's new
///   aggregate balance would exceed the configured maximum.
/// * [`ComplianceModuleError::MathOverflow`] - When the credit addition
///   overflows.
/// * refer to [`get_irs_client`] errors.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_created(e: &Env, to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let identity = get_irs_client(e, token).stored_identity(to);
    credit_identity(e, token, &identity, amount);
}

/// Records a burn from `from`: debits the sender's identity. The cap is not
/// re-checked because burning can only lower the aggregate balance.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The wallet whose tokens were burned.
/// * `amount` - The burned amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MathUnderflow`] - When the debit subtraction
///   underflows.
/// * refer to [`get_irs_client`] errors.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_destroyed(e: &Env, from: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let identity = get_irs_client(e, token).stored_identity(from);
    debit_identity(e, token, &identity, amount);
}

// ################## LOW-LEVEL HELPERS ##################

/// Writes the aggregate balance entry for `(token, identity)` directly to
/// persistent storage, replacing any existing value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity address.
/// * `balance` - The new balance to record.
///
/// # Security Warning
///
/// This helper performs no authorization checks and skips the per-identity
/// cap check. Callers must enforce both invariants themselves.
pub fn set_id_balance(e: &Env, token: &Address, identity: &Address, balance: i128) {
    let key = MaxBalanceStorageKey::IdBalance(token.clone(), identity.clone());
    e.storage().persistent().set(&key, &balance);
}

/// Credits `amount` to `identity`'s tracked aggregate balance under `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity (on-chain ID) address.
/// * `amount` - The amount to credit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::MaxBalanceExceeded`] - When the resulting
///   aggregate balance would exceed the configured maximum for `token`.
/// * [`ComplianceModuleError::MathOverflow`] - When adding `amount` to the
///   current balance overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn credit_identity(e: &Env, token: &Address, identity: &Address, amount: i128) {
    let current = get_id_balance(e, token, identity);
    let next = add_i128_or_panic(e, current, amount);
    if next > get_max_balance(e, token) {
        panic_with_error!(e, ComplianceModuleError::MaxBalanceExceeded);
    }
    set_id_balance(e, token, identity, next);
}

/// Credits `amount` to `identity`'s tracked aggregate balance under `token`
/// without enforcing the per-identity cap. Used for forced transfers, where
/// the cap is an investor-facing policy the admin is consciously overriding
/// but the books must still record the movement.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity (on-chain ID) address.
/// * `amount` - The amount to credit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathOverflow`] - When adding `amount` to the
///   current balance overflows.
///
/// # Security Warning
///
/// This helper performs no authorization checks and skips the per-identity
/// cap check.
pub fn credit_identity_unchecked(e: &Env, token: &Address, identity: &Address, amount: i128) {
    let current = get_id_balance(e, token, identity);
    set_id_balance(e, token, identity, add_i128_or_panic(e, current, amount));
}

/// Debits `amount` from `identity`'s tracked aggregate balance under `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity (on-chain ID) address.
/// * `amount` - The amount to debit. Must be non-negative; the caller is
///   responsible for validating it before calling.
///
/// # Errors
///
/// * [`ComplianceModuleError::MathUnderflow`] - When subtracting `amount` from
///   the current balance would yield a negative value or underflows `i128`.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn debit_identity(e: &Env, token: &Address, identity: &Address, amount: i128) {
    let current = get_id_balance(e, token, identity);
    let next = sub_i128_or_panic(e, current, amount);
    if next < 0 {
        panic_with_error!(e, ComplianceModuleError::MathUnderflow);
    }
    set_id_balance(e, token, identity, next);
}
