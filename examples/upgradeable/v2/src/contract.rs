/// The contract in "v1" needs to be upgraded with this one. We are
/// demonstrating the usage of the `UpgradeableMigratable` macro, because this
/// time we want to do a migration after the upgrade. That's why we derive
/// `UpgradeableMigratable`. For it to work, we implement
/// `UpgradeableMigratableInternal` with the custom migration logic.
use soroban_sdk::{
    contract, contracterror, contracttype, panic_with_error, symbol_short, Address, Env, Symbol,
};
use stellar_contract_utils::upgradeable::UpgradeableMigratableInternal;
use stellar_macros::UpgradeableMigratable;

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

#[derive(UpgradeableMigratable)]
#[contract]
pub struct ExampleContract;

impl UpgradeableMigratableInternal for ExampleContract {
    type MigrationData = Data;

    fn _require_auth(e: &Env, operator: &Address) {
        operator.require_auth();
        let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
        if *operator != owner {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }
    }

    fn _migrate(e: &Env, data: &Self::MigrationData) {
        e.storage().instance().set(&DATA_KEY, data);
    }
}
