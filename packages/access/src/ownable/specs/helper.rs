use cvlr::clog;
use soroban_sdk::{Address, Env};

use crate::ownable::OwnableStorageKey;

/// Returns `Some(Address)` if a pending owner is set, or `None` if there is no
/// pending ownership transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_pending_owner(e: &Env) -> Option<Address> {
    let pending_owner = e.storage().temporary().get::<_, Address>(&OwnableStorageKey::PendingOwner);
    if let Some(pending_owner_internal) = pending_owner.clone() {
        clog!(cvlr_soroban::Addr(&pending_owner_internal));
    }
    pending_owner
}
