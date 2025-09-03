use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{
    ownable::{
        emit_ownership_renounced, emit_ownership_transfer, emit_ownership_transfer_completed,
        OwnableError,
    },
    role_transfer::{accept_transfer, transfer_role},
    Ownable,
};

/// Storage keys for `Ownable` utility.
#[contracttype]
pub enum OwnableStorageKey {
    Owner,
    PendingOwner,
}

pub struct Owner;

impl Ownable for Owner {
    type Impl = Self;

    fn get_owner(e: &Env) -> Option<Address> {
        e.storage().instance().get(&OwnableStorageKey::Owner)
    }

    fn transfer_ownership(e: &Env, new_owner: &Address, live_until_ledger: u32) {
        let owner = Self::enforce_owner_auth(e);

        transfer_role(e, new_owner, &OwnableStorageKey::PendingOwner, live_until_ledger);

        emit_ownership_transfer(e, &owner, new_owner, live_until_ledger);
    }

    fn accept_ownership(e: &Env) {
        let new_owner =
            accept_transfer(e, &OwnableStorageKey::Owner, &OwnableStorageKey::PendingOwner);
        emit_ownership_transfer_completed(e, &new_owner);
    }

    fn renounce_ownership(e: &Env) {
        let owner = Self::enforce_owner_auth(e);
        let key = OwnableStorageKey::PendingOwner;

        if e.storage().temporary().get::<_, Address>(&key).is_some() {
            panic_with_error!(e, OwnableError::TransferInProgress);
        }

        e.storage().instance().remove(&OwnableStorageKey::Owner);
        emit_ownership_renounced(e, &owner);
    }

    fn set_owner(e: &Env, owner: &Address) {
        // Check if owner is already set
        if e.storage().instance().has(&OwnableStorageKey::Owner) {
            panic_with_error!(e, OwnableError::OwnerAlreadySet);
        }
        e.storage().instance().set(&OwnableStorageKey::Owner, &owner);
    }
}
