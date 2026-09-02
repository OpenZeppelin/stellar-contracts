extern crate std;

use soroban_sdk::{
    auth::Context,
    contract, contractimpl, contracttype, map,
    testutils::{Address as _, Events},
    vec,
    xdr::{ScErrorCode, ScErrorType, ToXdr},
    Address, Bytes, BytesN, Env, Error, Event, IntoVal, Map, TryFromVal, Val, Vec,
};
use stellar_accounts::{
    policies::Policy,
    smart_account::{ContextRule, Signer},
};

use crate::contract::{
    AccountDeployed, AccountFactoryContract, AccountFactoryContractClient, SALT_PREIMAGE_VERSION,
};

// The account wasm built from `examples/multisig-smart-account/account`. The
// factory deploys real account code so that the tests can read the deployed
// configuration back off the account.
mod account {
    soroban_sdk::contractimport!(file = "testdata/multisig_account_example.wasm");
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

#[contracttype]
enum MockPolicyStorageKey {
    Installed(Address, u32),
}

/// A policy that records the parameters it was installed with, so a test can
/// check that the map given to the factory is the map the account installed.
#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type AccountParams = Val;

    fn enforce(
        _e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(e: &Env, install_params: Val, rule: ContextRule, smart_account: Address) {
        // Like the shipped policies, the account must authorize its own
        // install. The account is the direct invoker during construction, so
        // this passes without any authorization entry.
        smart_account.require_auth();
        e.storage()
            .persistent()
            .set(&MockPolicyStorageKey::Installed(smart_account, rule.id), &install_params);
    }

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

#[contractimpl]
impl MockPolicyContract {
    pub fn install_params(e: &Env, smart_account: Address, rule_id: u32) -> Val {
        e.storage()
            .persistent()
            .get(&MockPolicyStorageKey::Installed(smart_account, rule_id))
            .unwrap()
    }
}

/// A contract that tries to create an account directly in somebody else's
/// namespace, exactly as the factory does but from outside of it.
#[contract]
struct SquatterContract;

#[contractimpl]
impl SquatterContract {
    pub fn squat(
        e: &Env,
        deployer: Address,
        chain_salt: BytesN<32>,
        wasm_hash: BytesN<32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> Address {
        e.deployer().with_address(deployer, chain_salt).deploy_v2(wasm_hash, (signers, policies))
    }
}

fn setup(e: &Env) -> (Address, Address, Address) {
    let wasm_hash = e.deployer().upload_contract_wasm(account::WASM);
    let factory = e.register(AccountFactoryContract, (wasm_hash,));
    let verifier = e.register(MockVerifierContract, ());
    let policy = e.register(MockPolicyContract, ());
    (factory, verifier, policy)
}

fn external(e: &Env, verifier: &Address, key: u8) -> Signer {
    Signer::External(verifier.clone(), Bytes::from_array(e, &[key; 32]))
}

fn no_policies(e: &Env) -> Map<Address, Val> {
    Map::new(e)
}

/// Signers of rule 0 of a deployed account, as the account holds them.
fn deployed_signers(e: &Env, account: &Address) -> Vec<Signer> {
    let rule = account::Client::new(e, account).get_context_rule(&0);
    Vec::<Signer>::try_from_val(e, &rule.signers.to_val()).unwrap()
}

/// Policies of rule 0 of a deployed account, as the account holds them.
fn deployed_policies(e: &Env, account: &Address) -> Vec<Address> {
    account::Client::new(e, account).get_context_rule(&0).policies
}

/// Test fixture that recomputes the documented salt derivation. It pins the
/// preimage layout; it is not a client helper, clients call `predict`.
fn mirror_chain_salt(
    e: &Env,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
    salt: u32,
) -> BytesN<32> {
    let preimage: Vec<Val> =
        (SALT_PREIMAGE_VERSION, signers.clone(), policies.clone(), salt).into_val(e);
    e.crypto().sha256(&preimage.to_xdr(e)).to_bytes()
}

fn address_in_namespace(e: &Env, deployer: &Address, chain_salt: BytesN<32>) -> Address {
    e.deployer().with_address(deployer.clone(), chain_salt).deployed_address()
}

#[test]
fn pinned_account_wasm_hash_returns_the_constructor_argument() {
    let e = Env::default();
    let wasm_hash = e.deployer().upload_contract_wasm(account::WASM);
    let factory = e.register(AccountFactoryContract, (wasm_hash.clone(),));
    let client = AccountFactoryContractClient::new(&e, &factory);

    assert_eq!(client.pinned_account_wasm_hash(), wasm_hash);
}

#[test]
fn predict_and_deploy_account_agree() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let predicted = client.predict(&signers, &no_policies(&e), &0);
    let deployed = client.deploy_account(&signers, &no_policies(&e), &0);

    assert_eq!(deployed, predicted);
    assert_eq!(deployed_signers(&e, &deployed), signers);
}

#[test]
fn deployed_account_holds_exactly_the_requested_configuration() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);
    let policies = map![&e, (policy.clone(), 2u32.into_val(&e))];

    // No `mock_all_auths`: factory-driven construction, including the policy
    // install that requires the account's authorization, needs no auth entry.
    let deployed = client.deploy_account(&vec![&e, b.clone(), a.clone()], &policies, &0);

    let signers = deployed_signers(&e, &deployed);
    assert_eq!(signers.len(), 2);
    assert!(signers.contains(&a));
    assert!(signers.contains(&b));
    assert_eq!(deployed_policies(&e, &deployed), vec![&e, policy.clone()]);

    let installed: u32 =
        MockPolicyContractClient::new(&e, &policy).install_params(&deployed, &0).into_val(&e);
    assert_eq!(installed, 2);
}

#[test]
fn chain_salt_is_sha256_of_the_canonical_xdr_preimage() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let supplied = vec![&e, external(&e, &verifier, 1), external(&e, &verifier, 2)];
    let policies = map![&e, (policy, 2u32.into_val(&e))];

    let deployed = client.deploy_account(&supplied, &policies, &41);

    // The canonical order is not guessed: it is the order the account holds,
    // which is the list the factory constructed with and hashed.
    let canonical = deployed_signers(&e, &deployed);
    assert_eq!(canonical.len(), 2);
    let from_canonical =
        address_in_namespace(&e, &factory, mirror_chain_salt(&e, &canonical, &policies, 41));
    assert_eq!(from_canonical, deployed);

    // Reversing a two-element canonical list is always non-canonical, so this
    // shows the hash covers the canonical list specifically.
    let reversed = vec![&e, canonical.get(1).unwrap(), canonical.get(0).unwrap()];
    let from_reversed =
        address_in_namespace(&e, &factory, mirror_chain_salt(&e, &reversed, &policies, 41));
    assert_ne!(from_reversed, deployed);
}

#[test]
fn signer_order_and_duplicates_do_not_change_the_address() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);

    let ab = client.predict(&vec![&e, a.clone(), b.clone()], &no_policies(&e), &0);
    let ba = client.predict(&vec![&e, b.clone(), a.clone()], &no_policies(&e), &0);
    assert_eq!(ab, ba);

    let aa = client.predict(&vec![&e, a.clone(), a.clone()], &no_policies(&e), &0);
    let a_only = client.predict(&vec![&e, a.clone()], &no_policies(&e), &0);
    assert_eq!(aa, a_only);

    // The canonical list is also what the account is constructed with.
    let deployed = client.deploy_account(&vec![&e, a.clone(), a.clone()], &no_policies(&e), &0);
    assert_eq!(deployed, a_only);
    assert_eq!(deployed_signers(&e, &deployed), vec![&e, a]);
}

#[test]
fn policy_map_order_does_not_change_the_address() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let other_policy = e.register(MockPolicyContract, ());
    let signers = vec![&e, external(&e, &verifier, 1)];

    let one_way =
        map![&e, (policy.clone(), 1u32.into_val(&e)), (other_policy.clone(), 2u32.into_val(&e))];
    let other_way = map![&e, (other_policy, 2u32.into_val(&e)), (policy, 1u32.into_val(&e))];

    assert_eq!(client.predict(&signers, &one_way, &0), client.predict(&signers, &other_way, &0));
}

#[test]
fn every_part_of_the_tuple_changes_the_address() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);
    let delegated = Signer::Delegated(Address::generate(&e));

    let addresses = [
        client.predict(&vec![&e, a.clone()], &no_policies(&e), &0),
        client.predict(&vec![&e, b.clone()], &no_policies(&e), &0),
        client.predict(&vec![&e, a.clone(), b.clone()], &no_policies(&e), &0),
        client.predict(&vec![&e, a.clone()], &map![&e, (policy.clone(), 1u32.into_val(&e))], &0),
        // Same policy address, different parameters: a 1-of-N and a 2-of-N
        // must never compete for one address.
        client.predict(&vec![&e, a.clone()], &map![&e, (policy.clone(), 2u32.into_val(&e))], &0),
        client.predict(&vec![&e, a.clone()], &no_policies(&e), &1),
        client.predict(&vec![&e, delegated], &no_policies(&e), &0),
    ];

    for (i, address) in addresses.iter().enumerate() {
        for other in &addresses[i + 1..] {
            assert_ne!(address, other);
        }
    }
}

#[test]
fn extra_salt_gives_one_configuration_several_accounts() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let first = client.deploy_account(&signers, &no_policies(&e), &0);
    let second = client.deploy_account(&signers, &no_policies(&e), &1);

    assert_ne!(first, second);
    assert_eq!(deployed_signers(&e, &first), signers);
    assert_eq!(deployed_signers(&e, &second), signers);
}

#[test]
fn deploying_the_same_tuple_twice_traps() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let deployed = client.deploy_account(&signers, &no_policies(&e), &0);

    // The host refuses to create a contract at an occupied address. The
    // refusal is a host error, not a contract error, so it reaches the caller
    // escalated to an untyped `Error(Context, InvalidAction)`.
    let again = client.try_deploy_account(&signers, &no_policies(&e), &0);
    assert_eq!(
        again,
        Err(Ok(Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction)))
    );
    // The failure changes nothing: the address is still the same tuple's.
    assert_eq!(client.predict(&signers, &no_policies(&e), &0), deployed);
}

#[test]
fn predict_validates_nothing() {
    let e = Env::default();
    let (factory, _, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let empty: Vec<Signer> = Vec::new(&e);

    // An address for a configuration the account contract refuses. Nothing
    // else can be created there either, since the namespace is the factory's.
    let predicted = client.predict(&empty, &no_policies(&e), &0);
    assert_eq!(
        predicted,
        address_in_namespace(&e, &factory, mirror_chain_salt(&e, &empty, &no_policies(&e), 0))
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #3004)")]
fn account_constructor_errors_propagate_from_deploy_account() {
    let e = Env::default();
    let (factory, _, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);

    // `SmartAccountError::NoSignersAndPolicies`
    client.deploy_account(&Vec::new(&e), &no_policies(&e), &0);
}

#[test]
fn only_the_factory_can_deploy_in_its_namespace() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let squatter = e.register(SquatterContract, ());
    let squatter_client = SquatterContractClient::new(&e, &squatter);
    let wasm_hash = client.pinned_account_wasm_hash();

    let victim_signers = vec![&e, external(&e, &verifier, 1)];
    let attacker_signers = vec![&e, external(&e, &verifier, 9)];
    let victim = client.predict(&victim_signers, &no_policies(&e), &0);
    let chain_salt = mirror_chain_salt(&e, &victim_signers, &no_policies(&e), 0);
    assert_eq!(address_in_namespace(&e, &factory, chain_salt.clone()), victim);

    // The attacker's signers at the victim's chain salt in the factory's
    // namespace. Creating there requires the factory's authorization, and a
    // contract authorizes only by executing, so this cannot succeed from
    // outside the factory.
    let squat = squatter_client.try_squat(
        &factory,
        &chain_salt,
        &wasm_hash,
        &attacker_signers,
        &no_policies(&e),
    );
    assert!(squat.is_err());

    // Control: the identical operation in the squatter's own namespace works,
    // so it was the namespace that was refused, not the operation.
    let own = squatter_client.squat(
        &squatter,
        &chain_salt,
        &wasm_hash,
        &attacker_signers,
        &no_policies(&e),
    );
    assert_eq!(own, address_in_namespace(&e, &squatter, chain_salt));
    assert_ne!(own, victim);

    // The legitimate deployment still lands on the predicted address.
    assert_eq!(client.deploy_account(&victim_signers, &no_policies(&e), &0), victim);
}

#[test]
fn front_running_creates_the_victims_account() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let victim_signers = vec![&e, external(&e, &verifier, 1)];
    let attacker_signers = vec![&e, external(&e, &verifier, 9)];

    let victim = client.predict(&victim_signers, &no_policies(&e), &0);

    // Whoever submits the victim's tuple first creates the victim's account.
    // There is no argument through which a different configuration can land
    // at this address.
    let created = client.deploy_account(&victim_signers, &no_policies(&e), &0);
    assert_eq!(created, victim);
    assert_eq!(deployed_signers(&e, &created), victim_signers);

    let attackers_own = client.deploy_account(&attacker_signers, &no_policies(&e), &0);
    assert_ne!(attackers_own, victim);
}

#[test]
fn every_deploy_uses_the_pinned_wasm() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    // A factory pinned to a hash with no code behind it. `predict` still
    // answers, since the wasm is not part of the address, but nothing can be
    // deployed and there is no argument through which to supply other code.
    let bogus_hash = BytesN::from_array(&e, &[7u8; 32]);
    let other_factory = e.register(AccountFactoryContract, (bogus_hash.clone(),));
    let other_client = AccountFactoryContractClient::new(&e, &other_factory);
    assert_eq!(other_client.pinned_account_wasm_hash(), bogus_hash);

    let other_predicted = other_client.predict(&signers, &no_policies(&e), &0);
    assert_eq!(
        other_predicted,
        address_in_namespace(
            &e,
            &other_factory,
            mirror_chain_salt(&e, &signers, &no_policies(&e), 0)
        )
    );
    assert!(other_client.try_deploy_account(&signers, &no_policies(&e), &0).is_err());

    // A different pin is a different factory, hence a different namespace:
    // the same tuple has a different address on each.
    assert_ne!(client.predict(&signers, &no_policies(&e), &0), other_predicted);
}

#[test]
fn deploy_account_emits_account_deployed() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);
    let policies = map![&e, (policy, 2u32.into_val(&e))];

    let deployed = client.deploy_account(&vec![&e, b, a], &policies, &5);

    // The account constructor emits its own events first; the factory's event
    // is the last one and carries the canonical signer list.
    let events = e.events().all();
    assert_eq!(
        events.events().last().unwrap(),
        &AccountDeployed {
            account: deployed.clone(),
            signers: deployed_signers(&e, &deployed),
            policies,
            salt: 5,
        }
        .to_xdr(&e, &factory)
    );
}
