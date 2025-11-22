use soroban_sdk::{Env, Address};
use crate::access_control::AccessControlStorageKey;
use cvlr::clog;

/// Returns `Some(Address)` if a pending owner is set, or `None` if there is no pending ownership transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_pending_admin(e: &Env) -> Option<Address> {
    let pending_admin = e.storage().temporary().get::<_, Address>(&AccessControlStorageKey::PendingAdmin);
    if let Some(pending_admin_internal) = pending_admin.clone() {
        clog!(cvlr_soroban::Addr(&pending_admin_internal));
    }
    pending_admin
}

