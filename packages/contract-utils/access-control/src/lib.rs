use soroban_sdk::{contracttype, panic_with_error, Address, Env, Symbol};
use stellar_constants::{
    OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT, TOKEN_TTL_THRESHOLD,
};

/// String identifier for the default admin role
pub const DEFAULT_ADMIN_ROLE_STR: &str = "admin";

#[contracttype]
pub struct RoleAccountKey {
    pub role: Symbol,
    pub account: Address,
}

/// Storage keys for the data associated with the access control
#[contracttype]
pub enum AccessControlStorageKey {
    RoleAccounts(RoleAccountKey),
    RoleAccountsIndex(u32),
}

/// Errors that can be returned by the access control module
#[contracttype]
pub enum AccessControlError {
    Unauthorized,
    RoleNotFound,
    AccountNotFound,
}

pub struct AccessControl;

impl AccessControl {
    // ################## QUERY STATE ##################

    /// Returns true if the account has the specified role
    pub fn has_role(e: &Env, account: &Address, role: &Symbol) -> bool {
        let key = AccessControlStorageKey::RoleAccountsIndex(account.clone());
        e.storage().persistent().has(&key)
    }

    /// Returns the admin role for a given role
    pub fn get_role_admin(e: &Env, role: &Symbol) -> Symbol {
        let key = AccessControlStorageKey::RoleAdmins(role.clone());
        e.storage().persistent().get(&key).unwrap_or_else(|| Self::default_admin_role(e))
    }

    // ################## CHANGE STATE ##################

    /// Grants a role to an account
    pub fn grant_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
        // Check if caller has admin role for this role
        let admin_role = Self::get_role_admin(e, role);
        if !Self::has_role(e, caller, &admin_role) {
            panic_with_error!(e, AccessControlError::Unauthorized);
        }

        // Add to role enumeration
        Self::add_to_role_enumeration(e, account, role);
    }

    /// Revokes a role from an account
    pub fn revoke_role(e: &Env, caller: &Address, account: &Address, role: &Symbol) {
        // Check if caller has admin role for this role
        let admin_role = Self::get_role_admin(e, role);
        if !Self::has_role(e, caller, &admin_role) {
            panic_with_error!(e, AccessControlError::Unauthorized);
        }

        // Remove from role enumeration
        Self::remove_from_role_enumeration(e, account, role);
    }

    /// Sets the admin role for a given role
    pub fn set_role_admin(e: &Env, caller: &Address, role: &Symbol, admin_role: &Symbol) {
        // Check if caller has admin role for this role
        let current_admin_role = Self::get_role_admin(e, role);
        if !Self::has_role(e, caller, &current_admin_role) {
            panic_with_error!(e, AccessControlError::Unauthorized);
        }

        let key = AccessControlStorageKey::RoleAdmins(role.clone());
        e.storage().persistent().set(&key, admin_role);
    }

    // ################## LOW-LEVEL HELPERS ##################

    /// Adds an account to role enumeration
    fn add_to_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
        // Get the current count of accounts for this role
        let key =
            AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index: 0 });
        let count = e.storage().persistent().get(&key).unwrap_or(0);

        // Add to role enumeration
        e.storage().persistent().set(
            &AccessControlStorageKey::RoleAccounts(RoleAccountKey {
                role: role.clone(),
                index: count,
            }),
            account,
        );
        e.storage()
            .persistent()
            .set(&AccessControlStorageKey::RoleAccountsIndex(account.clone()), &count);

        // Increment count
        e.storage().persistent().set(
            &AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index: 0 }),
            &(count + 1),
        );
    }

    /// Removes an account from role enumeration
    fn remove_from_role_enumeration(e: &Env, account: &Address, role: &Symbol) {
        let key = AccessControlStorageKey::RoleAccountsIndex(account.clone());
        let Some(index) = e.storage().persistent().get::<_, u32>(&key) else {
            panic_with_error!(e, AccessControlError::AccountNotFound);
        };

        // Get the count of accounts for this role
        let count_key =
            AccessControlStorageKey::RoleAccounts(RoleAccountKey { role: role.clone(), index: 0 });
        let count = e.storage().persistent().get(&count_key).unwrap_or(0);

        // If not the last account, swap with last
        if index != count - 1 {
            let last_account_key = AccessControlStorageKey::RoleAccounts(RoleAccountKey {
                role: role.clone(),
                index: count - 1,
            });
            let last_account = e.storage().persistent().get(&last_account_key).unwrap();

            // Move last account to current index
            e.storage().persistent().set(
                &AccessControlStorageKey::RoleAccounts(RoleAccountKey {
                    role: role.clone(),
                    index,
                }),
                &last_account,
            );

            // Update last account's index
            e.storage()
                .persistent()
                .set(&AccessControlStorageKey::RoleAccountsIndex(last_account), &index);
        }

        // Remove the last account
        e.storage().persistent().remove(&AccessControlStorageKey::RoleAccounts(RoleAccountKey {
            role: role.clone(),
            index: count - 1,
        }));

        // Remove the account's index
        e.storage().persistent().remove(&key);

        // Decrement count
        e.storage().persistent().set(&count_key, &(count - 1));
    }

    /// Returns the symbol for default admin role symbol
    fn default_admin_role(e: &Env) -> Symbol {
        Symbol::new(e, DEFAULT_ADMIN_ROLE_STR)
    }
}

/// Ensures that the caller has the specified role
pub fn ensure_has_role(e: &Env, caller: &Address, role: &Symbol) {
    if !AccessControl::has_role(e, caller, role) {
        panic_with_error!(e, AccessControlError::Unauthorized);
    }
}
