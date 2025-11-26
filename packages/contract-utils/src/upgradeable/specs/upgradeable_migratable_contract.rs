use soroban_sdk::contract;
use stellar_macros::{Upgradeable, UpgradeableMigratable};

use crate::{
    self as stellar_contract_utils,
    upgradeable::{UpgradeableMigratable, UpgradeableMigratableInternal},
};

// Not sure what the desired behavior should be. Fill as needed.
// NOTE: Unclear what `MigrationData` should be so change as needed.
// NOTE: fill out `_migrate` and `_require_auth` before using!

#[derive(UpgradeableMigratable)]
#[contract]
pub struct UpgradeableMigratableContract;

impl UpgradeableMigratableInternal for UpgradeableMigratableContract {
    // Made a mock Migration data.
    type MigrationData = u32;

    fn _migrate(e: &soroban_sdk::Env, migration_data: &Self::MigrationData) {
        todo!()
    }

    fn _require_auth(e: &soroban_sdk::Env, operator: &soroban_sdk::Address) {
        todo!()
    }
}
