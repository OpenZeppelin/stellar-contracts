use soroban_sdk::{contracterror, Address, Env, Symbol};

/// A trait for managing contract ownership using a 2-step transfer pattern.
///
/// Provides functions to query ownership, initiate a transfer, or renounce
/// ownership.
pub trait Ownable {
    /// Returns `Some(Address)` if ownership is set, or `None` if ownership has
    /// been renounced.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_owner(e: &Env) -> Option<Address> {
        crate::get_owner(e)
    }

    /// Initiates a 2-step ownership transfer to a new address.
    ///
    /// Requires authorization from the current owner. The new owner must later
    /// call `accept_ownership()` to complete the transfer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `caller` - The current owner initiating the transfer.
    /// * `new_owner` - The proposed new owner.
    /// * `live_until_ledger` - Ledger number until which the new owner can
    ///   accept. A value of `0` cancels any pending transfer.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::NotAuthorized`] - If `caller` is not the current
    ///   owner.
    fn transfer_ownership(e: &Env, caller: Address, new_owner: Address, live_until_ledger: u32);

    /// Renounces ownership of the contract.
    ///
    /// Permanently removes the owner, disabling all functions gated by
    /// `#[only_owner]`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `caller` - The current owner.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::NotAuthorized`] - If `caller` is not the current
    ///   owner.
    fn renounce_ownership(e: &Env, caller: Address);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OwnableError {
    NotAuthorized = 130,
    TransferInProgress = 131,
}

// ################## EVENTS ##################

/// Emits an event when an ownership transfer is initiated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_owner` - The current owner initiating the transfer.
/// * `new_owner` - The proposed new owner.
/// * `live_until_ledger` - The ledger number at which the pending transfer will
///   expire. If this value is `0`, it means the pending transfer is cancelled.
///
/// # Events
///
/// * topics - `["ownership_transfer"]`
/// * data - `[old_owner: Address, new_owner: Address]`
pub fn emit_ownership_transfer(
    e: &Env,
    old_owner: &Address,
    new_owner: &Address,
    live_until_ledger: u32,
) {
    let topics = (Symbol::new(e, "ownership_transfer"),);
    e.events().publish(topics, (old_owner, new_owner, live_until_ledger));
}

/// Emits an event when an ownership transfer is completed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `new_owner` - The new owner who accepted the transfer.
///
/// # Events
///
/// * topics - `["ownership_transfer_completed"]`
/// * data - `[new_owner: Address]`
pub fn emit_ownership_transfer_completed(e: &Env, new_owner: &Address) {
    let topics = (Symbol::new(e, "ownership_transfer_completed"),);
    e.events().publish(topics, new_owner);
}

/// Emits an event when ownership is renounced.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_owner` - The address of the owner who renounced ownership.
///
/// # Events
///
/// * topics - `["ownership_renounced"]`
/// * data - `[old_owner: Address]`
pub fn emit_ownership_renounced(e: &Env, old_owner: &Address) {
    let topics = (Symbol::new(e, "ownership_renounced"),);
    e.events().publish(topics, old_owner);
}
