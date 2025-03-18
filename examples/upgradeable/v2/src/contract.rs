use soroban_sdk::{contract, contracttype, symbol_short, Address, Env, Symbol};

use stellar_upgradeable::{Migration, Upgrade};
use stellar_upgradeable_macros::Upgradeable;

pub const DATA_KEY: Symbol = symbol_short!("DATA_KEY");
pub const OWNER: Symbol = symbol_short!("OWNER");

#[contracttype]
pub struct Data {
    pub num1: u32,
    pub num2: u32,
}

#[derive(Upgradeable)]
#[migrateable]
#[contract]
pub struct ExampleContract;

impl Upgrade for ExampleContract {
    fn upgrade_auth(e: &Env) {
        e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
    }
}

impl Migration for ExampleContract {
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
