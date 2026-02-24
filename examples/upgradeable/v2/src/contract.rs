/// The contract in "v1" needs to be upgraded with this one. We demonstrate how
/// to implement `Upgradeable` directly and add a `migrate` function that uses
/// `upgradeable::run_migration()`. The `migrate()` function and its arguments
/// are completely customizable.
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    BytesN, Env, Symbol,
};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};

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

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl Upgradeable for ExampleContract {
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
        operator.require_auth();
        let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
        if operator != owner {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[contractimpl]
impl ExampleContract {
    pub fn migrate(e: &Env, migration_data: Data, operator: Address) {
        operator.require_auth();
        let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
        if operator != owner {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }
        upgradeable::run_migration(e, || {
            e.storage().instance().set(&DATA_KEY, &migration_data);
        });
    }
}
