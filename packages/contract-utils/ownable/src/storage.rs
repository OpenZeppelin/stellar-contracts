use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_role_transfer::{accept_transfer, transfer_role};

use crate::ownable::{
    emit_ownership_renounced, emit_ownership_transfer, emit_ownership_transfer_completed,
    OwnableError,
};

/// Storage keys for `Ownable` utility.
#[contracttype]
pub enum OwnableStorageKey {
    Owner,
    PendingOwner,
}

// ################## QUERY STATE ##################

/// Returns `Some(Address)` if ownership is set, or `None` if ownership has been
/// renounced.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_owner(e: &Env) -> Option<Address> {
    e.storage().instance().get::<_, Address>(&OwnableStorageKey::Owner)
}

/// Ensures that the caller is the current owner. Panics if not.
///
/// This is used internally by the `#[only_owner]` macro expansion to gate
/// access.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `caller` - The address attempting the restricted action.
///
/// # Errors
///
/// * [`OwnableError::NotAuthorized`] - If the caller is not the current owner.
pub fn ensure_is_owner(e: &Env, caller: &Address) {
    if let Some(owner) = get_owner(e) {
        if owner != *caller {
            panic_with_error!(e, OwnableError::NotAuthorized);
        }
    } else {
        // No owner means ownership has been renounced — no one can call restricted
        // functions
        panic_with_error!(e, OwnableError::NotAuthorized);
    }
}

// ################## CHANGE STATE ##################

/// Sets owner role.
///
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The account to grant the owner privilege.
///
/// **IMPORTANT**: this function lacks authorization checks.
/// It is expected to call this function only in the constructor!
pub fn set_owner(e: &Env, owner: &Address) {
    e.storage().instance().set(&OwnableStorageKey::Owner, &owner);
}

/// Initiates a 2-step ownership transfer to a new owner.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `caller` - The current owner initiating the transfer.
/// * `new_owner` - The proposed new owner.
/// * `live_until_ledger` - Ledger number until which the new owner can accept.
///   A value of `0` cancels any pending transfer.
///
/// # Errors
///
/// * See [`transfer_role()`] for possible errors.
///
/// # Events
///
/// * topics - `["ownership_transfer"]`
/// * data - `[old_owner: Address, new_owner: Address]`
pub fn transfer_ownership(e: &Env, caller: &Address, new_owner: &Address, live_until_ledger: u32) {
    transfer_role(
        e,
        caller,
        new_owner,
        &OwnableStorageKey::Owner,
        &OwnableStorageKey::PendingOwner,
        live_until_ledger,
    );

    emit_ownership_transfer(e, caller, new_owner, live_until_ledger);
}

/// Completes the 2-step ownership transfer process.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `caller` - The address of the pending owner accepting ownership.
///
/// # Errors
///
/// * See [`accept_transfer()`] for possible errors.
///
/// # Events
///
/// * topics - `["ownership_transfer_completed"]`
/// * data - `[old_owner: Address, new_owner: Address]`
pub fn accept_ownership(e: &Env, caller: &Address) {
    let previous_owner =
        get_owner(e).expect("One cannot renounce ownership while there is a pending transfer.");

    accept_transfer(e, caller, &OwnableStorageKey::Owner, &OwnableStorageKey::PendingOwner);

    emit_ownership_transfer_completed(e, &previous_owner, caller);
}

/// Renounces ownership of the contract.
///
/// Once renounced, no one will have privileged access via `#[only_owner]`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `caller` - The current owner.
///
/// # Errors
///
/// * [`OwnableError::TransferInProgress`] - If there is a pending ownership
///   transfer.
/// * refer to [`ensure_is_owner()`].
///
/// # Events
///
/// * topics - `["ownership_renounced"]`
/// * data - `[old_owner: Address]`
pub fn renounce_ownership(e: &Env, caller: &Address) {
    ensure_is_owner(e, caller);
    caller.require_auth();

    if e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner).is_some() {
        panic_with_error!(e, OwnableError::TransferInProgress);
    }

    e.storage().instance().remove(&OwnableStorageKey::Owner);
    emit_ownership_renounced(e, caller);
}
