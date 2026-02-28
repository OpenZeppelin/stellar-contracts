use soroban_sdk::{contract, contractimpl, map, vec, Address, Bytes, Env, TryFromVal, Val, Vec};
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

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(_e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        true
    }

    pub fn canonicalize_key(e: &Env, key_data: Val) -> Bytes {
        Bytes::try_from_val(e, &key_data).unwrap()
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(e, key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap()))
    }
}

#[test]
fn create_account() {
    let e = Env::default();
    let verifier = e.register(MockVerifierContract, ());
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

#[test]
fn batch_add_signer_adds_all_signers() {
    let e = Env::default();
    e.mock_all_auths();

    let verifier = e.register(MockVerifierContract, ());
    let initial_signer = Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, b"4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29"),
    );
    let account = e.register(
        MultisigContract,
        (vec![&e, initial_signer.clone()], soroban_sdk::Map::<Address, Val>::new(&e)),
    );

    let new_signer_1 = Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, b"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"),
    );
    let new_signer_2 = Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, b"1111111111111111111111111111111111111111111111111111111111111111"),
    );

    let client = crate::contract::MultisigContractClient::new(&e, &account);
    client.batch_add_signer(&0, &vec![&e, new_signer_1.clone(), new_signer_2.clone()]);

    let rule = client.get_context_rule(&0);
    assert_eq!(rule.signers.len(), 3);
    assert!(rule.signers.contains(&initial_signer));
    assert!(rule.signers.contains(&new_signer_1));
    assert!(rule.signers.contains(&new_signer_2));
}
