#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};
use stellar_diamond_proxy_core::storage::SharedStorageImpl;
use stellar_facet_macro::facet;

#[contract]
pub struct SharedStorageFacetIncrementBy;

#[facet]
#[contractimpl]
impl SharedStorageFacetIncrementBy {
    // Increment the counter
    pub fn increment_by(env: Env, by: u32) -> u32 {
        let counter = Self::get_value(env.clone());
        let new_value = counter.saturating_add(by);
        let _ = env
            .shared_storage()
            .instance()
            .set(&Symbol::new(&env, "counter"), &new_value);
        new_value
    }

    // Decrement the counter
    pub fn decrement_by(env: Env, by: u32) -> u32 {
        let counter = Self::get_value(env.clone());

        let new_value = counter.saturating_sub(by);
        let _ = env
            .shared_storage()
            .instance()
            .set(&Symbol::new(&env, "counter"), &new_value);
        new_value
    }

    // Get the current counter value
    pub fn get_value(env: Env) -> u32 {
        env.shared_storage()
            .instance()
            .get::<_, u32>(&Symbol::new(&env, "counter"))
            .unwrap_or(0)
    }
}
