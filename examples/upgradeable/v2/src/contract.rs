use soroban_sdk::{contract, contracttype, symbol_short, Address, Env, Symbol};
use stellar_upgradeable::{MigratableInternal, UpgradeableInternal};
use stellar_upgradeable_macros::Upgradeable;

pub const DATA_KEY: Symbol = symbol_short!("DATA_KEY");
pub const OWNER: Symbol = symbol_short!("OWNER");

#[contracttype]
pub struct Data {
    pub num1: u32,
    pub num2: u32,
}

#[derive(Upgradeable)]
#[migratable]
#[contract]
pub struct ExampleContract;

impl UpgradeableInternal for ExampleContract {
    type UpgradeData = ();

    fn _upgrade(e: &Env, _data: &Self::UpgradeData) {
        e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
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
