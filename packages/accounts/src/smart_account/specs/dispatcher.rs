use soroban_sdk::{auth::Context, Env, Vec, Address, BytesN, crypto::Hash, Bytes};
use crate::smart_account::storage::{ContextRuleType, SmartAccountStorageKey, ContextRule};
use crate::smart_account::{Signer, Meta};
use crate::smart_account::specs::verifier1::Verifier1;
use crate::smart_account::specs::verifier2::Verifier2;
use crate::smart_account::specs::policy1::Policy1;
use crate::smart_account::specs::policy2::Policy2;
use crate::verifiers::Verifier;
use crate::policies::Policy;

// manual dispatching between different policies

mod ghost_vars {
    use super::Address;
    use crate::smart_account::specs::ghosts::GhostVar;
    
    pub(super) static mut POLICY1_ADDRESS: GhostVar<Address> = GhostVar::UnInit;
    pub(super) static mut POLICY2_ADDRESS: GhostVar<Address> = GhostVar::UnInit;
    pub(super) static mut VERIFIER1_ADDRESS: GhostVar<Address> = GhostVar::UnInit;
    pub(super) static mut VERIFIER2_ADDRESS: GhostVar<Address> = GhostVar::UnInit;
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

pub fn verify_dispatch(
    e: &Env,
    hash: Bytes,
    key_data: BytesN<32>,
    sig_data: BytesN<64>,
    verifier_addr: Address,
) -> bool {
    unsafe {
        let verifier1_addr = ghost_vars::VERIFIER1_ADDRESS.get();
        let verifier2_addr = ghost_vars::VERIFIER2_ADDRESS.get();
        if verifier_addr == verifier1_addr {
            return Verifier1::verify(e, hash, key_data.clone(), sig_data.clone());
        }
        else if verifier_addr == verifier2_addr {
            return Verifier2::verify(e, hash, key_data.clone(), sig_data.clone());
        }
        else {
            panic!("Verifier not found");
        }
    }
}