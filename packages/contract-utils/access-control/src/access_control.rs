use soroban_sdk::{Address, Env, Symbol};

pub trait AccessControl {
    /// Returns true if the account has the specified role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `account` - The account to check.
    /// * `role` - The role to check for.
    fn has_role(e: &Env, account: &Address, role: &Symbol) -> bool {
        crate::has_role(e, account, role)
    }

    /// Returns the total number of accounts that have the specified role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to get the count for.
    fn get_role_member_count(e: &Env, role: &Symbol) -> u32 {
        crate::get_role_member_count(e, role)
    }

    /// Returns the account at the specified index for a given role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `role` - The role to query.
    /// * `index` - The index of the account to retrieve.
    fn get_role_member(e: &Env, role: &Symbol, index: u32) -> Address {
        crate::get_role_member(e, role, index)
    }

    /// Returns the admin account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn get_admin(e: &Env) -> Address {
        crate::get_admin(e)
    }

    /// Grants a role to an account.
    /// Creates the role if it does not exist.
    /// Returns early if the account already has the role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be admin.
    /// * `account` - The account to grant the role to.
    /// * `role` - The role to grant.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::grant_role()`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    /// The caller must be the admin.
    fn grant_role(e: &Env, caller: &Address, account: &Address, role: &Symbol);

    /// Revokes a role from an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be admin.
    /// * `account` - The account to revoke the role from.
    /// * `role` - The role to revoke.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::revoke_role()`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    /// The caller must be the admin.
    fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &Symbol);

    // TODO: add `renounce()`

    /// Initiates admin role transfer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be admin.
    /// * `new_admin` - The account to transfer admin privileges to.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::transfer_admin_role()`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: You MUST implement proper authorization in your contract.
    /// The caller must be the admin.
    fn transfer_admin_role(e: &Env, caller: &Address, new_admin: &Address);

    /// Completes the 2-step admin transfer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller, must be the pending admin.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::accept_admin_transfer()`] when implementing this function.
    fn accept_admin_transfer(e: &Env, caller: &Address);
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccessControlError {
    Unauthorized = 120,
    RoleNotFound = 121,
    AccountNotFound = 122,
    NoPendingAdminTransfer = 123,
}
