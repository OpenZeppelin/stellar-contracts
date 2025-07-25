use soroban_sdk::{panic_with_error, Address, Env, IntoVal, Val};

use crate::role_transfer::RoleTransferError;

/// Initiates the role transfer. If `live_until_ledger == 0`, cancels the
/// pending transfer.
///
/// Does not emit any events.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `new` - The proposed new role holder.
/// * `pending_key` - Storage key for the pending role holder.
/// * `live_until_ledger` - Ledger number until which the new role holder can
///   accept. A value of `0` cancels the pending transfer. If the specified
///   ledger is in the past or exceeds the maximum allowed TTL extension for a
///   temporary storage entry, the function will panic.
///
/// # Errors
///
/// * [`RoleTransferError::NoPendingTransfer`] - If trying to cancel a transfer
///   that doesn't exist.
/// * [`RoleTransferError::InvalidLiveUntilLedger`] - If the specified ledger is
///   in the past, or exceeds the maximum allowed TTL extension for a temporary
///   storage entry.
/// * [`RoleTransferError::InvalidPendingAccount`] - If the specified pending
///   account is not the same as the provided `new` address.
///
/// # Notes
///
/// * This function does not enforce authorization. Ensure that authorization is
///   handled at a higher level.
/// * The period during which the transfer can be accepted is implicitly
///   timebound by the maximum allowed storage TTL value which is a network
///   parameter, i.e. one cannot set `live_until_ledger` for a longer period.
/// * There is also a default minimum TTL and if the computed period is shorter
///   than it, the entry will outlive `live_until_ledger`.
pub fn transfer_role<T>(e: &Env, new: &Address, pending_key: &T, live_until_ledger: u32)
where
    T: IntoVal<Env, Val>,
{
    if live_until_ledger == 0 {
        let Some(pending) = e.storage().temporary().get::<T, Address>(pending_key) else {
            panic_with_error!(e, RoleTransferError::NoPendingTransfer);
        };
        if pending != *new {
            panic_with_error!(e, RoleTransferError::InvalidPendingAccount);
        }
        e.storage().temporary().remove(pending_key);

        return;
    }

    let current_ledger = e.ledger().sequence();
    if live_until_ledger > e.ledger().max_live_until_ledger() || live_until_ledger < current_ledger
    {
        panic_with_error!(e, RoleTransferError::InvalidLiveUntilLedger);
    }

    let live_for = live_until_ledger - current_ledger;
    e.storage().temporary().set(pending_key, new);
    e.storage().temporary().extend_ttl(pending_key, live_for, live_for);
}

/// Completes the role transfer if authorization is provided by the pending role
/// holder. Pending role holder is retrieved from the storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `active_key` - Storage key for the current role holder.
/// * `pending_key` - Storage key for the pending role holder.
///
/// # Errors
///
/// * [`RoleTransferError::NoPendingTransfer`] - If there is no pending transfer
///   to accept.
pub fn accept_transfer<T, U>(e: &Env, active_key: &T, pending_key: &U) -> Address
where
    T: IntoVal<Env, Val>,
    U: IntoVal<Env, Val>,
{
    let pending = e
        .storage()
        .temporary()
        .get::<U, Address>(pending_key)
        .unwrap_or_else(|| panic_with_error!(e, RoleTransferError::NoPendingTransfer));

    pending.require_auth();

    e.storage().temporary().remove(pending_key);
    e.storage().instance().set(active_key, &pending);

    pending
}
