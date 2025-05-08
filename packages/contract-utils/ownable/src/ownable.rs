use soroban_sdk::{contracterror, Address, Env, Symbol};

/// Trait for ownership-based access control.
pub trait Ownable {
    /// Returns the current owner of the contract (if any).
    fn owner(e: &Env) -> Option<Address> {
        crate::get_owner(e)
    }

    /// Transfers ownership to a new address.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::NotAuthorized`] if caller is not the current owner.
    /// * [`OwnableError::InvalidNewOwner`] if new owner is the zero address.
    fn transfer_ownership(e: &Env, caller: Address, new_owner: Address);

    /// Renounces ownership. Leaves the contract without an owner.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::NotAuthorized`] if caller is not the current owner.
    fn renounce_ownership(e: &Env, caller: Address);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OwnableError {
    NotAuthorized = 200,
    InvalidNewOwner = 201,
}

// ################## EVENTS ##################

/// Emits `ownership_transferred` event.
pub fn emit_ownership_transferred(e: &Env, old_owner: &Address, new_owner: &Address) {
    let topics = (Symbol::new(e, "ownership_transferred"),);
    e.events().publish(topics, (old_owner, new_owner));
}

/// Emits `ownership_renounced` event.
pub fn emit_ownership_renounced(e: &Env, old_owner: &Address) {
    let topics = (Symbol::new(e, "ownership_renounced"),);
    e.events().publish(topics, old_owner);
}
