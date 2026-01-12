use soroban_sdk::{contract, contracttype};
use stellar_macros::{Upgradeable, UpgradeableMigratable};
use soroban_sdk::panic_with_error;

use crate::{
    self as stellar_contract_utils,
    upgradeable::{UpgradeableMigratable, UpgradeableMigratableInternal},
};

#[contracttype]
pub enum StorageKey {
    MigrationDataKey,
    OwnerDataKey,   
}

// Not sure what the desired behavior should be. Fill as needed.
// NOTE: Unclear what `MigrationData` should be so change as needed.
// NOTE: fill out `_migrate` and `_require_auth` before using!

#[derive(UpgradeableMigratable)]
#[contract]
pub struct UpgradeableMigratableContract;

impl UpgradeableMigratableInternal for UpgradeableMigratableContract {
    type MigrationData = u32;

    fn _migrate(e: &soroban_sdk::Env, migration_data: &Self::MigrationData) {
        UpgradeableMigratableContract::set_migration_data(e, *migration_data);
    }

    fn _require_auth(e: &soroban_sdk::Env, operator: &soroban_sdk::Address) {
        operator.require_auth(); 
        let owner = UpgradeableMigratableContract::get_owner(e);
        if *operator != owner {
            panic!();
        }
    }

}

impl UpgradeableMigratableContract {
    pub fn set_owner(e: &soroban_sdk::Env, new_owner: &soroban_sdk::Address) {
        let owner = UpgradeableMigratableContract::get_owner(e);
        owner.require_auth();
        e.storage().instance().set(&StorageKey::OwnerDataKey, new_owner);
    }

    pub fn get_owner(e: &soroban_sdk::Env) -> soroban_sdk::Address {
        e.storage().instance().get::<_, soroban_sdk::Address>(&StorageKey::OwnerDataKey).unwrap()
    }
    
    pub fn set_migration_data(e: &soroban_sdk::Env, migration_data: u32) {
        let owner = UpgradeableMigratableContract::get_owner(e);
        owner.require_auth();
        e.storage().instance().set(&StorageKey::MigrationDataKey, &migration_data);
    }

    pub fn get_migration_data(e: &soroban_sdk::Env) -> u32 {
        e.storage().instance().get::<_, u32>(&StorageKey::MigrationDataKey).unwrap()
    }
}
