use soroban_sdk::{panic_with_error, symbol_short, Address, Env, Symbol};

use crate::ownable::{emit_ownership_renounced, emit_ownership_transferred, OwnableError};

pub const OWNER: Symbol = symbol_short!("OWNER");

/// Returns the current owner (if any).
pub fn get_owner(e: &Env) -> Option<Address> {
    e.storage().instance().get::<Symbol, Address>(&OWNER)
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

/// Transfers ownership to a new owner.
///
/// # Notes
/// Authorization of `caller` must be done **before** calling this.
///
/// # Panics
/// * If `new_owner` is the zero address (for safety).
/// * If caller is not the current owner.
pub fn transfer_ownership(e: &Env, caller: &Address, new_owner: &Address) {
    ensure_is_owner(e, caller);
    caller.require_auth();

    if caller == new_owner {
        return; // No-op if transferring to self
    }

    let old_owner = caller.clone();
    e.storage().instance().set(&OWNER, new_owner);
    emit_ownership_transferred(e, &old_owner, new_owner);
}

/// Renounces ownership, leaving the contract without an owner.
pub fn renounce_ownership(e: &Env, caller: &Address) {
    ensure_is_owner(e, caller);
    caller.require_auth();

    e.storage().instance().remove(&OWNER);
    emit_ownership_renounced(e, caller);
}
