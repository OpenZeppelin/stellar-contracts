use soroban_sdk::{Env, Vec};

use crate::smart_account::storage::{ContextRuleType, SmartAccountStorageKey};

pub fn get_next_id(e: Env) -> u32 {
    e.storage().instance().get(&SmartAccountStorageKey::NextId).unwrap_or(0u32)
}

pub fn get_count(e: Env) -> u32 {
    e.storage().instance().get(&SmartAccountStorageKey::Count).unwrap_or(0u32)
}

pub fn get_ids_of_rule_type(e: Env, rule_type: ContextRuleType) -> Vec<u32> {
    e.storage().persistent().get::<_, Vec<u32>>(&SmartAccountStorageKey::Ids(rule_type)).unwrap_or_else(|| Vec::new(&e))
}