use soroban_sdk::{auth::Context, Env, Vec, Address};
use crate::smart_account::storage::{ContextRuleType, SmartAccountStorageKey, ContextRule};
use crate::smart_account::{Signer, Meta};
use crate::smart_account::specs::policy1::Policy1;
use crate::smart_account::specs::policy2::Policy2;
use crate::policies::Policy;

pub fn get_next_id(e: Env) -> u32 {
    e.storage().instance().get(&SmartAccountStorageKey::NextId).unwrap_or(0u32)
}

pub fn get_count(e: Env) -> u32 {
    e.storage().instance().get(&SmartAccountStorageKey::Count).unwrap_or(0u32)
}

pub fn get_ids_of_rule_type(e: Env, rule_type: ContextRuleType) -> Vec<u32> {
    e.storage().persistent().get::<_, Vec<u32>>(&SmartAccountStorageKey::Ids(rule_type)).unwrap_or_else(|| Vec::new(&e))
}

pub fn get_policies_of_id(e: Env, id: u32) -> Vec<Address> {
    e.storage().persistent().get::<_, Vec<Address>>(&SmartAccountStorageKey::Policies(id)).unwrap_or_else(|| Vec::new(&e))
}

pub fn get_signers_of_id(e: Env, id: u32) -> Vec<Signer> {
    e.storage().persistent().get::<_, Vec<Signer>>(&SmartAccountStorageKey::Signers(id)).unwrap_or_else(|| Vec::new(&e))
}

pub fn get_meta_of_id(e: Env, id: u32) -> Meta {
    e.storage().persistent().get::<_, Meta>(&SmartAccountStorageKey::Meta(id)).unwrap_or_else(|| panic!())
}

// 

use crate::smart_account::MAX_SIGNERS;
use crate::smart_account::MAX_POLICIES;

// a version of the validate_signers_and_policies function that
// returns false instead of panicking and returns true otherwise.
pub fn validate_signers_and_policies_non_panicking(
    e: &Env,
    signers: &Vec<Signer>,
    policies: &Vec<Address>,
) -> bool {
    if signers.len() > MAX_SIGNERS {
        return false
    }

    if policies.len() > MAX_POLICIES {
        return false;
    }

    // Check minimum requirements - must have at least one signer or one policy
    if signers.is_empty() && policies.is_empty() {
        return false;
    }

    true
}
