#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};
use stellar_diamond_proxy_core::storage::SharedStorageImpl;
use stellar_facet_macro::facet;

#[contract]
pub struct SharedStorageFacetIncrement;

#[facet]
#[contractimpl]
impl SharedStorageFacetIncrement {
    // Increment the counter
    pub fn increment(env: Env) -> u32 {
        let counter = Self::get_value(env.clone());
        let new_value = counter.saturating_add(1);
        let _ = env
            .shared_storage()
            .instance()
            .set(&Symbol::new(&env, "counter"), &new_value);
        new_value
    }

    // Decrement the counter
    pub fn decrement(env: Env) -> u32 {
        let counter = Self::get_value(env.clone());

        let new_value = counter.saturating_sub(1);
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
