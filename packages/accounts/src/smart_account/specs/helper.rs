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

// manual dispatching between different policies

mod ghost_vars {
    use super::Address;
    use crate::smart_account::specs::ghosts::GhostVar;
    
    pub(super) static mut POLICY1_ADDRESS: GhostVar<Address> = GhostVar::UnInit;
    pub(super) static mut POLICY2_ADDRESS: GhostVar<Address> = GhostVar::UnInit;
}

pub fn can_enforce_dispatch(
    e: &Env,
    context: &Context,
    matched_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
    policy_addr: Address,
) -> bool {
    unsafe {
        let policy1_addr = ghost_vars::POLICY1_ADDRESS.get();
        let policy2_addr = ghost_vars::POLICY2_ADDRESS.get();
        if policy_addr == policy1_addr {
            return Policy1::can_enforce(e, context.clone(), matched_signers.clone(), context_rule.clone(), smart_account.clone());
        } 
        else if policy_addr == policy2_addr {
            return Policy2::can_enforce(e, context.clone(), matched_signers.clone(), context_rule.clone(), smart_account.clone());
        }
        else {
            panic!("Policy not found");
        }
    }
}