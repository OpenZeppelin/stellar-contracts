use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{
    ownable::{
        OwnableError,
    },
    role_transfer::{accept_transfer, transfer_role},
};

#[cfg(not(feature = "certora"))]
use crate::{
    ownable::{
        emit_ownership_renounced, emit_ownership_transfer, emit_ownership_transfer_completed
    }
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
    e.storage().instance().get::<_, Address>(&OwnableStorageKey::Owner)
}

// ################## CHANGE STATE ##################

/// Sets owner role.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The account to grant the owner privilege.
///
/// # Errors
///
/// * [`OwnableError::OwnerAlreadySet`] - If the owner is already set.
///
/// **IMPORTANT**: this function lacks authorization checks.
/// It is expected to call this function only in the constructor!
pub fn set_owner(e: &Env, owner: &Address) {
    // Check if owner is already set
    if e.storage().instance().has(&OwnableStorageKey::Owner) {
        panic_with_error!(e, OwnableError::OwnerAlreadySet);
    }
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

    #[cfg(not(feature = "certora"))]
    emit_ownership_transfer(e, &owner, new_owner, live_until_ledger);
}

/// Completes the 2-step ownership transfer process.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
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

    #[cfg(not(feature = "certora"))]
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
    #[cfg(not(feature = "certora"))]
    let key = OwnableStorageKey::PendingOwner;
    #[cfg(not(feature = "certora"))]
    if e.storage().temporary().get::<_, Address>(&key).is_some() {
        panic_with_error!(e, OwnableError::TransferInProgress);
    }
    #[cfg(feature = "certora")]
    if e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner).is_some() {
        panic_with_error!(e, OwnableError::TransferInProgress);
    }

    e.storage().instance().remove(&OwnableStorageKey::Owner);
    #[cfg(not(feature = "certora"))]
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
/// * [`OwnableError::OwnerNotSet`] - If the owner is not set.
pub fn enforce_owner_auth(e: &Env) -> Address {
    let Some(owner) = get_owner(e) else {
        panic_with_error!(e, OwnableError::OwnerNotSet);
    };
    owner.require_auth();
    owner
}
