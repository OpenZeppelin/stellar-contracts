use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_constants::{OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD};
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
/// renounced or has never been set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_owner(e: &Env) -> Option<Address> {
    e.storage()
        .instance()
        .get::<_, Address>(&OwnableStorageKey::Owner)
        .inspect(|_| e.storage().instance().extend_ttl(OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT))
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
/// * `new_owner` - The proposed new owner.
/// * `live_until_ledger` - Ledger number until which the new owner can accept.
///   A value of `0` cancels any pending transfer.
///
/// # Errors
///
/// * refer to [`transfer_role`] errors.
/// * refer to [`enforce_owner_auth`] errors.
///
///
/// # Events
///
/// * topics - `["ownership_transfer"]`
/// * data - `[old_owner: Address, new_owner: Address]`
///
/// # Notes
///
/// * Authorization for the current owner is required.
pub fn transfer_ownership(e: &Env, new_owner: &Address, live_until_ledger: u32) {
    let owner = enforce_owner_auth(e);

    transfer_role(e, new_owner, &OwnableStorageKey::PendingOwner, live_until_ledger);

    emit_ownership_transfer(e, &owner, new_owner, live_until_ledger);
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
/// * refer to [`accept_transfer`] errors.
///
/// # Events
///
/// * topics - `["ownership_transfer_completed"]`
/// * data - `[new_owner: Address]`
///
/// # Notes
///
/// * Authorization for the pending owner is required.
pub fn accept_ownership(e: &Env) {
    let new_owner = accept_transfer(e, &OwnableStorageKey::Owner, &OwnableStorageKey::PendingOwner);

    emit_ownership_transfer_completed(e, &new_owner);
}

/// Renounces ownership of the contract.
///
/// Once renounced, no one will have privileged access via `#[only_owner]`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`OwnableError::TransferInProgress`] - If there is a pending ownership
///   transfer.
/// * refer to [`enforce_owner_auth`] errors.
///
/// # Events
///
/// * topics - `["ownership_renounced"]`
/// * data - `[old_owner: Address]`
///
/// # Notes
///
/// * Authorization for the current owner is required.
pub fn renounce_ownership(e: &Env) {
    let owner = enforce_owner_auth(e);
    let key = OwnableStorageKey::PendingOwner;

    if e.storage().temporary().get::<_, Address>(&key).is_some() {
        e.storage().temporary().extend_ttl(&key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);
        panic_with_error!(e, OwnableError::TransferInProgress);
    }

    e.storage().instance().remove(&OwnableStorageKey::Owner);
    emit_ownership_renounced(e, &owner);
}

// ################## LOW-LEVEL HELPERS ##################

/// Enforces authorization from the current owner.
///
/// This is used internally by the `#[only_owner]` macro expansion to gate
/// access.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`OwnableError::NotAuthorized`] - If the authorization from the current
///   owner is missing.
pub fn enforce_owner_auth(e: &Env) -> Address {
    if let Some(owner) = get_owner(e) {
        owner.require_auth();
        owner
    } else {
        // No owner means ownership has been renounced — no one can call restricted
        // functions
        panic_with_error!(e, OwnableError::NotAuthorized);
    }
}
