/// The contract in "v1" needs to be upgraded with this one. Again, we are
/// demonstrating the usage of the `Upgradeable` macro, but this time we want to
/// do a migration after the upgrade. That's why we derive `Migratable` as well.
/// For it to work, we implement `MigratableInternal` with the custom migration
/// and rollback logic.
use soroban_sdk::{
    contract, contracterror, contracttype, panic_with_error, symbol_short, Address, Env, Symbol,
};
use stellar_upgradeable::{MigratableInternal, UpgradeableInternal};
use stellar_upgradeable_macros::{Migratable, Upgradeable};

pub const DATA_KEY: Symbol = symbol_short!("DATA_KEY");
pub const OWNER: Symbol = symbol_short!("OWNER");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

#[contracttype]
pub struct Data {
    pub num1: u32,
    pub num2: u32,
}

#[derive(Upgradeable, Migratable)]
#[contract]
pub struct ExampleContract;

impl UpgradeableInternal for ExampleContract {
    fn _upgrade_auth(e: &Env, operator: &Address) {
        operator.require_auth();
        let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
        if *operator != owner {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }
    }
}

impl MigratableInternal for ExampleContract {
    type MigrationData = Data;
    type RollbackData = ();

    fn _migrate(e: &Env, data: &Self::MigrationData) {
        e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
        e.storage().instance().set(&DATA_KEY, data);
    }

    fn _rollback(e: &Env, _data: &Self::RollbackData) {
        e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
        e.storage().instance().remove(&DATA_KEY);
    }
}
