/// The contract in "v1" needs to be upgraded with this one. We are
/// demonstrating the usage of the `UpgradeableMigratable` macro, because this
/// time we want to do a migration after the upgrade. That's why we derive
/// `UpgradeableMigratable`. For it to work, we implement
/// `UpgradeableMigratableInternal` with the custom migration logic.
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Env, Symbol};
use stellar_contract_utils::Upgradeable;
use stellar_macros::only_owner;

pub const DATA_KEY: &Symbol = &symbol_short!("DATA_KEY");

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

#[derive(Upgradeable)]
#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    #[only_owner]
    pub fn set_data(env: Env, data: Data) {
        env.storage().persistent().set(DATA_KEY, &data);
    }

    pub fn get_data(env: Env) -> Data {
        env.storage().persistent().get(DATA_KEY).unwrap()
    }
}
