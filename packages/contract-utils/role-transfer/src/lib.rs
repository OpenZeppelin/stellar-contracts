//! This module only acts as a utility crate for `Access Control` and `Ownable`.
//! It is not intended to be used directly.

use soroban_sdk::{panic_with_error, Address, Env, Val};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
enum RoleTransferError {
    Unauthorized = 140,
    NoPendingTransfer = 141,
    InvalidLiveUntilLedger = 142,
}

/// Initiates the role transfer (owner/admin).
/// If `live_until_ledger == 0`, cancels the pending transfer.
pub fn transfer_role(
    e: &Env,
    current: &Address,
    new: &Address,
    active_key: &Val,
    pending_key: &Val,
    live_until_ledger: u32,
) {
    current.require_auth();

    let stored_current = e.storage().instance().get::<Val, Address>(active_key).unwrap();

    if current != &stored_current {
        panic_with_error!(e, RoleTransferError::Unauthorized);
    }

    if live_until_ledger == 0 {
        let Some(pending) = e.storage().temporary().get::<_, Address>(pending_key) else {
            panic_with_error!(e, RoleTransferError::NoPendingTransfer);
        };
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

/// Completes the role transfer if `caller` is the pending new owner/admin.
pub fn accept_transfer(e: &Env, caller: &Address, active_key: &Val, pending_key: &Val) {
    caller.require_auth();

    let pending = e
        .storage()
        .temporary()
        .get::<Val, Address>(pending_key)
        .unwrap_or_else(|| panic_with_error!(e, RoleTransferError::NoPendingTransfer));

    if caller != &pending {
        panic_with_error!(e, RoleTransferError::Unauthorized);
    }

    e.storage().temporary().remove(pending_key);
    e.storage().instance().set(active_key, caller);
}

mod test;
