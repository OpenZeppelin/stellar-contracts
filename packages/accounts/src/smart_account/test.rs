#![cfg(test)]
extern crate std;

use soroban_sdk::{
    auth::Context, contract, contractimpl, symbol_short, testutils::Address as _, Address, Bytes,
    Env, Vec,
};

use super::storage::Signer;

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl MockPolicyContract {
    pub fn can_enforce(
        e: &Env,
        _context: Context,
        _context_rule_signers: Vec<Signer>,
        _authenticated_signers: Vec<Signer>,
        _smart_account: Address,
    ) -> bool {
        e.storage().persistent().get(&symbol_short!("enforce")).unwrap_or(true)
    }

    fn enforce(
        _e: &Env,
        _context: Context,
        _context_rule_signers: Vec<Signer>,
        _authenticated_signers: Vec<Signer>,
        _smart_account: Address,
    ) {
    }
}

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(e: &Env, _hash: Bytes, _key_data: Bytes, _sig_data: Bytes) -> bool {
        e.storage().persistent().get(&symbol_short!("verify")).unwrap_or(true)
    }
}
