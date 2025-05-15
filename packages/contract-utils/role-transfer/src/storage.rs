use soroban_sdk::{panic_with_error, Address, Env, IntoVal, Val};

use crate::RoleTransferError;

/// Initiates the role transfer (owner/admin).
/// If `live_until_ledger == 0`, cancels the pending transfer.
///
/// Does not emit any events.
///
/// Returns None if the transfer was initiated, Some(Address) where the Address
/// is the new candidate for the pending role in the storage if the pending
/// transfer is revoked.
pub fn transfer_role<T, U>(
    e: &Env,
    current: &Address,
    new: &Address,
    active_key: &T,
    pending_key: &U,
    live_until_ledger: u32,
) -> Option<Address>
where
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
        e.storage().temporary().remove(pending_key);

        return Some(pending);
    }

    let current_ledger = e.ledger().sequence();
    if live_until_ledger < current_ledger {
        panic_with_error!(e, RoleTransferError::InvalidLiveUntilLedger);
    }

    let live_for = live_until_ledger - current_ledger;
    e.storage().temporary().set(pending_key, new);
    e.storage().temporary().extend_ttl(pending_key, live_for, live_for);

    None
}

/// Completes the role transfer if `caller` is the pending new owner/admin.
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
