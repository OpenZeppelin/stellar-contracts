use soroban_sdk::{panic_with_error, Address, Env, IntoVal, Val};

use crate::RoleTransferError;

/// Initiates the role transfer. If `live_until_ledger == 0`, cancels the
/// pending transfer.
///
/// Does not emit any events.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `current` - The current role holder initiating the transfer.
/// * `new` - The proposed new role holder.
/// * `active_key` - Storage key for the current role holder.
/// * `pending_key` - Storage key for the pending role holder.
/// * `live_until_ledger` - Ledger number until which the new role holder can
///   accept. A value of `0` cancels the pending transfer.
///
/// # Errors
///
/// * [`RoleTransferError::AccountNotFound`] - If the current role holder is not
///   found in storage.
/// * [`RoleTransferError::Unauthorized`] - If the caller is not the current
///   role holder.
/// * [`RoleTransferError::NoPendingTransfer`] - If trying to cancel a transfer
///   that doesn't exist.
/// * [`RoleTransferError::InvalidLiveUntilLedger`] - If the specified ledger is
///   in the past.
/// * [`RoleTransferError::InvalidPendingAccount`] - If the specified pending
///   account is not the same as the provided `new` address.
pub fn transfer_role<T, U>(
    e: &Env,
    current: &Address,
    new: &Address,
    active_key: &T,
    pending_key: &U,
    live_until_ledger: u32,
) where
    T: IntoVal<Env, Val>,
    U: IntoVal<Env, Val>,
{
    current.require_auth();

    let current_in_storage = e
        .storage()
        .instance()
        .get::<T, Address>(active_key)
        .unwrap_or_else(|| panic_with_error!(e, RoleTransferError::AccountNotFound));

    if current != &current_in_storage {
        panic_with_error!(e, RoleTransferError::Unauthorized);
    }

    if live_until_ledger == 0 {
        let Some(pending) = e.storage().temporary().get::<U, Address>(pending_key) else {
            panic_with_error!(e, RoleTransferError::NoPendingTransfer);
        };
        if pending != *new {
            panic_with_error!(e, RoleTransferError::InvalidPendingAccount);
        }
        e.storage().temporary().remove(pending_key);

        return;
    }

    let current_ledger = e.ledger().sequence();
    if live_until_ledger < current_ledger {
        panic_with_error!(e, RoleTransferError::InvalidLiveUntilLedger);
    }

    let live_for = live_until_ledger - current_ledger;
    e.storage().temporary().set(pending_key, new);
    e.storage().temporary().extend_ttl(pending_key, live_for, live_for);
}

/// Completes the role transfer if `caller` is the pending new role holder.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `caller` - The address of the pending role holder accepting the transfer.
/// * `active_key` - Storage key for the current role holder.
/// * `pending_key` - Storage key for the pending role holder.
///
/// # Errors
///
/// * [`RoleTransferError::NoPendingTransfer`] - If there is no pending transfer
///   to accept.
/// * [`RoleTransferError::Unauthorized`] - If the caller is not the pending
///   role holder.
pub fn accept_transfer<T, U>(e: &Env, caller: &Address, active_key: &T, pending_key: &U)
where
    T: IntoVal<Env, Val>,
    U: IntoVal<Env, Val>,
{
    caller.require_auth();

    let pending = e
        .storage()
        .temporary()
        .get::<U, Address>(pending_key)
        .unwrap_or_else(|| panic_with_error!(e, RoleTransferError::NoPendingTransfer));

    if caller != &pending {
        panic_with_error!(e, RoleTransferError::Unauthorized);
    }

    e.storage().temporary().remove(pending_key);
    e.storage().instance().set(active_key, caller);
}
