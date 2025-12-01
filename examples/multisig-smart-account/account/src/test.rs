use soroban_sdk::{
    contract, contractimpl, map, testutils::Address as _, vec, Address, Bytes, Env, Val, Vec,
};
use stellar_accounts::{
    policies::{simple_threshold::SimpleThresholdAccountParams, Policy},
    smart_account::{ContextRule, Signer},
};

use crate::contract::MultisigContract;

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type AccountParams = Val;

    fn can_enforce(
        _e: &Env,
        _context: soroban_sdk::auth::Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) -> bool {
        true
    }

    fn enforce(
        _e: &Env,
        _context: soroban_sdk::auth::Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(
        _e: &Env,
        _install_params: Self::AccountParams,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

#[test]
fn create_account() {
    let e = Env::default();
    let verifier = Address::generate(&e);
    let policy = e.register(MockPolicyContract, ());
    let signers = vec![
        &e,
        Signer::External(
            verifier.clone(),
            Bytes::from_array(
                &e,
                b"4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29",
            ),
        ),
        Signer::External(
            verifier.clone(),
            Bytes::from_array(
                &e,
                b"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
            ),
        ),
    ];
    let policies = map![&e, (policy, SimpleThresholdAccountParams { threshold: 2 })];
    e.register(MultisigContract, (signers, policies));
}
