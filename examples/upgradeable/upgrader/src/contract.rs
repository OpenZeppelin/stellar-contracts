/// Helper contract to perform upgrade+migrate or rollback+downgrade in a single
/// transaction.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol, Val};
use stellar_upgradeable::UpgradeableClient;

pub const MIGRATE: Symbol = symbol_short!("migrate");
pub const ROLLBACK: Symbol = symbol_short!("rollback");
pub const ADMIN: Symbol = symbol_short!("ADMIN");

#[contract]
pub struct Upgrader;

#[contractimpl]
impl Upgrader {
    /// Initialize the contract with an admin address that will have the authority to perform upgrades
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("contract already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
    }

    /// Updates the admin address
    /// 
    /// # Arguments
    /// 
    /// * `env` - The Soroban environment
    /// * `current_admin` - The current admin address (must be authenticated)
    /// * `new_admin` - The new admin address
    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) {
        // Verify the current admin is calling this function
        current_admin.require_auth();
        
        // Verify current_admin is actually the admin
        let stored_admin: Address = env.storage().instance().get(&ADMIN).unwrap_or_else(|| {
            panic!("contract not initialized");
        });
        
        if current_admin != stored_admin {
            panic!("not authorized");
        }
        
        // Update the admin
        env.storage().instance().set(&ADMIN, &new_admin);
    }
    
    /// Get the current admin address
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN).unwrap_or_else(|| {
            panic!("contract not initialized");
        })
    }

    /// Ensure the operator is authorized to perform upgrade operations
    fn check_authorization(env: &Env, operator: &Address) {
        operator.require_auth();
        
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap_or_else(|| {
            panic!("contract not initialized");
        });
        
        if *operator != admin {
            panic!("not authorized");
        }
    }

    pub fn upgrade(env: Env, contract_address: Address, operator: Address, wasm_hash: BytesN<32>) {
        // Verify the operator is authorized
        Self::check_authorization(&env, &operator);
        
        let contract_client = UpgradeableClient::new(&env, &contract_address);
        contract_client.upgrade(&wasm_hash, &operator);
    }

    pub fn upgrade_and_migrate(
        env: Env,
        contract_address: Address,
        operator: Address,
        wasm_hash: BytesN<32>,
        migration_data: soroban_sdk::Vec<Val>,
    ) {
        // Verify the operator is authorized
        Self::check_authorization(&env, &operator);
        
        let contract_client = UpgradeableClient::new(&env, &contract_address);
        contract_client.upgrade(&wasm_hash, &operator);
        // The types of the arguments to the migrate function are unknown to this
        // contract, so we need to call it with invoke_contract.
        env.invoke_contract::<()>(&contract_address, &MIGRATE, migration_data);
    }

    pub fn rollback_and_upgrade(
        env: Env,
        contract_address: Address,
        operator: Address,
        wasm_hash: BytesN<32>,
        rollback_data: soroban_sdk::Vec<Val>,
    ) {
        // Verify the operator is authorized
        Self::check_authorization(&env, &operator);
        
        let contract_client = UpgradeableClient::new(&env, &contract_address);
        // The types of the arguments to the rollback function are unknown to this
        // contract, so we need to call it with invoke_contract.
        env.invoke_contract::<()>(&contract_address, &ROLLBACK, rollback_data);
        contract_client.upgrade(&wasm_hash, &operator);
    }
}
