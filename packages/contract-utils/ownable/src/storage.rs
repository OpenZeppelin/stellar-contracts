use soroban_sdk::{contracttype, panic_with_error, symbol_short, Address, Env, Symbol};
use stellar_role_transfer::{accept_transfer, transfer_role};

use crate::ownable::{emit_ownership_renounced, emit_ownership_transferred, OwnableError};

#[contracttype]
pub enum OwnableStorageKey {
    Owner,
    PendingOwner,
}

/// Returns the current owner (if any).
pub fn get_owner(e: &Env) -> Option<Address> {
    e.storage().instance().get::<_, Address>(&OwnableStorageKey::Owner)
}

/// Ensures that the caller is the current owner. Panics if not.
///
/// This is used inside the `#[only_owner]` macro expansion.
pub fn ensure_is_owner(e: &Env, caller: &Address) {
    if let Some(owner) = get_owner(e) {
        if owner != *caller {
            panic_with_error!(e, OwnableError::NotAuthorized);
        }
    } else {
        // No owner means ownership has been renounced — no one can call restricted functions
        panic_with_error!(e, OwnableError::NotAuthorized);
    }
}

pub fn transfer_ownership(e: &Env, caller: &Address, new_owner: &Address, live_until_ledger: u32) {
    match transfer_role(
        e,
        admin,
        new_admin,
        &OwnableStorageKey::Owner,
        &OwnableStorageKey::PendingOwner,
        live_until_ledger,
    ) {
        Some(pending) => emit_owner_transfer(e, admin, &pending, live_until_ledger),
        None => emit_owner_transfer(e, admin, new_admin, live_until_ledger),
    }
}

pub fn accept_ownership(e: &Env, caller: &Address) {
    let previous_owner = get_owner(e);

    accept_transfer(e, caller, &OwnableStorageKey::Owner, &OwnableStorageKey::PendingOwner);

    emit_admin_transfer_completed(e, &previous_admin, caller);
}

/// Renounces ownership, leaving the contract without an owner.
pub fn renounce_ownership(e: &Env, caller: &Address) {
    ensure_is_owner(e, caller);
    caller.require_auth();

    e.storage().instance().remove(&OwnableStorageKey::Owner);
    emit_ownership_renounced(e, caller);
}
