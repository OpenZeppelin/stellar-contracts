extern crate std;

use ed25519_dalek::{Signer as Ed25519Signer, SigningKey};
use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractimpl,
    crypto::Hash,
    map,
    testutils::Ledger,
    vec,
    xdr::{
        Hash as XdrHash, HashIdPreimage, HashIdPreimageSorobanAuthorization, InvokeContractArgs,
        Limits, ScVal, SorobanAddressCredentials, SorobanAuthorizationEntry,
        SorobanAuthorizedFunction, SorobanAuthorizedInvocation, SorobanCredentials, VecM, WriteXdr,
    },
    Address, Bytes, BytesN, Env, IntoVal, Map, String, TryFromVal, Val, Vec,
};

use crate::{
    smart_account::{
        add_context_rule, do_check_auth, AuthDigestPreimage, AuthPayload, ContextRule,
        ContextRuleType, Signer, SmartAccount, SmartAccountError,
    },
    verifiers::{ed25519, Verifier},
};

const ACCOUNT_NONCE: i64 = 1;
const DELEGATE_NONCE: i64 = 2;
const EXPIRATION_LEDGER: u32 = 100;
const SECRET_KEY: [u8; 32] = [
    157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197, 105,
    123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
];

#[contract]
struct SmartAccountContract;

#[contractimpl]
impl CustomAccountInterface for SmartAccountContract {
    type Error = SmartAccountError;
    type Signature = AuthPayload;

    fn __check_auth(
        e: Env,
        signature_payload: Hash<32>,
        signatures: AuthPayload,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        do_check_auth(&e, &signature_payload, &signatures, &auth_contexts)
    }
}

#[contractimpl(contracttrait)]
impl SmartAccount for SmartAccountContract {}

#[contract]
struct Probe;

#[contractimpl]
impl Probe {
    pub fn ping(_e: &Env, caller: Address) {
        caller.require_auth();
    }
}

#[contract]
struct ApprovingAccount;

#[contractimpl]
impl CustomAccountInterface for ApprovingAccount {
    type Error = SmartAccountError;
    type Signature = Val;

    fn __check_auth(
        _e: Env,
        _signature_payload: Hash<32>,
        _signature: Val,
        _auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[contract]
struct Ed25519VerifierContract;

#[contractimpl]
impl Verifier for Ed25519VerifierContract {
    type KeyData = BytesN<32>;
    type SigData = BytesN<64>;

    fn verify(e: &Env, hash: Bytes, key_data: BytesN<32>, sig_data: BytesN<64>) -> bool {
        ed25519::verify(e, &hash, &key_data, &sig_data)
    }

    fn canonicalize_key(e: &Env, key_data: BytesN<32>) -> Bytes {
        ed25519::canonicalize_key(e, &key_data)
    }

    fn batch_canonicalize_key(e: &Env, key_data: Vec<BytesN<32>>) -> Vec<Bytes> {
        ed25519::batch_canonicalize_key(e, &key_data)
    }
}

fn to_scval<T: IntoVal<Env, Val>>(e: &Env, value: T) -> ScVal {
    ScVal::try_from_val(e, &value.into_val(e)).unwrap()
}

fn contract_fn(
    contract: &Address,
    name: &str,
    args: std::vec::Vec<ScVal>,
) -> SorobanAuthorizedInvocation {
    SorobanAuthorizedInvocation {
        function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
            contract_address: contract.into(),
            function_name: name.try_into().unwrap(),
            args: args.try_into().unwrap(),
        }),
        sub_invocations: VecM::default(),
    }
}

fn signature_payload(e: &Env, nonce: i64, invocation: &SorobanAuthorizedInvocation) -> BytesN<32> {
    let preimage = HashIdPreimage::SorobanAuthorization(HashIdPreimageSorobanAuthorization {
        network_id: XdrHash(e.ledger().get().network_id),
        nonce,
        signature_expiration_ledger: EXPIRATION_LEDGER,
        invocation: invocation.clone(),
    });
    let xdr = preimage.to_xdr(Limits::none()).unwrap();
    e.crypto().sha256(&Bytes::from_slice(e, &xdr)).to_bytes()
}

fn auth_entry(
    address: &Address,
    nonce: i64,
    signature: ScVal,
    root_invocation: SorobanAuthorizedInvocation,
) -> SorobanAuthorizationEntry {
    SorobanAuthorizationEntry {
        credentials: SorobanCredentials::Address(SorobanAddressCredentials {
            address: address.into(),
            nonce,
            signature_expiration_ledger: EXPIRATION_LEDGER,
            signature,
        }),
        root_invocation,
    }
}

fn register_account(e: &Env, signer: &Signer) -> (Address, Address, u32) {
    let account = e.register(SmartAccountContract, ());
    let probe = e.register(Probe, ());
    let rule_id = e.as_contract(&account, || {
        add_context_rule(
            e,
            &ContextRuleType::CallContract(probe.clone()),
            &String::from_str(e, "probe"),
            None,
            &vec![e, signer.clone()],
            &Map::new(e),
        )
        .id
    });
    (account, probe, rule_id)
}

fn ping_invocation(e: &Env, probe: &Address, account: &Address) -> SorobanAuthorizedInvocation {
    contract_fn(probe, "ping", std::vec![to_scval(e, account.clone())])
}

fn account_preimage(
    e: &Env,
    account: &Address,
    rule_id: u32,
    root_invocation: &SorobanAuthorizedInvocation,
) -> AuthDigestPreimage {
    AuthDigestPreimage {
        account: account.clone(),
        signature_payload: signature_payload(e, ACCOUNT_NONCE, root_invocation),
        context_rule_ids: vec![e, rule_id],
    }
}

fn account_entry(
    e: &Env,
    account: &Address,
    signer: &Signer,
    sig_data: Bytes,
    rule_id: u32,
    root_invocation: SorobanAuthorizedInvocation,
) -> SorobanAuthorizationEntry {
    let payload = AuthPayload {
        signers: map![e, (signer.clone(), sig_data)],
        context_rule_ids: vec![e, rule_id],
    };
    auth_entry(account, ACCOUNT_NONCE, to_scval(e, payload), root_invocation)
}

fn delegate_entry(delegate: &Address, account: &Address, arg: ScVal) -> SorobanAuthorizationEntry {
    auth_entry(
        delegate,
        DELEGATE_NONCE,
        ScVal::Void,
        contract_fn(account, "__check_auth", std::vec![arg]),
    )
}

fn external_signer(e: &Env) -> (Signer, SigningKey) {
    let verifier = e.register(Ed25519VerifierContract, ());
    let signing_key = SigningKey::from_bytes(&SECRET_KEY);
    let public_key = Bytes::from_array(e, signing_key.verifying_key().as_bytes());
    (Signer::External(verifier, public_key), signing_key)
}

fn sign(e: &Env, signing_key: &SigningKey, message: &BytesN<32>) -> Bytes {
    Bytes::from_array(e, &signing_key.sign(&message.to_array()).to_bytes())
}

#[test]
fn delegated_signer_authorizes_with_nested_auth_entry() {
    let e = Env::default();
    let delegate = e.register(ApprovingAccount, ());
    let signer = Signer::Delegated(delegate.clone());
    let (account, probe, rule_id) = register_account(&e, &signer);

    let root_invocation = ping_invocation(&e, &probe, &account);
    let preimage = account_preimage(&e, &account, rule_id, &root_invocation);

    e.set_auths(&[
        account_entry(&e, &account, &signer, Bytes::new(&e), rule_id, root_invocation),
        delegate_entry(&delegate, &account, to_scval(&e, preimage)),
    ]);

    ProbeClient::new(&e, &probe).ping(&account);
}

#[test]
fn delegated_signer_without_nested_auth_entry_fails() {
    let e = Env::default();
    let delegate = e.register(ApprovingAccount, ());
    let signer = Signer::Delegated(delegate.clone());
    let (account, probe, rule_id) = register_account(&e, &signer);

    let root_invocation = ping_invocation(&e, &probe, &account);

    e.set_auths(&[account_entry(&e, &account, &signer, Bytes::new(&e), rule_id, root_invocation)]);

    assert!(ProbeClient::new(&e, &probe).try_ping(&account).is_err());
}

#[test]
fn delegated_signer_nested_entry_with_digest_argument_fails() {
    let e = Env::default();
    let delegate = e.register(ApprovingAccount, ());
    let signer = Signer::Delegated(delegate.clone());
    let (account, probe, rule_id) = register_account(&e, &signer);

    let root_invocation = ping_invocation(&e, &probe, &account);
    let preimage = account_preimage(&e, &account, rule_id, &root_invocation);
    let digest = to_scval(&e, preimage.digest(&e));

    e.set_auths(&[
        account_entry(&e, &account, &signer, Bytes::new(&e), rule_id, root_invocation),
        delegate_entry(&delegate, &account, digest),
    ]);

    assert!(ProbeClient::new(&e, &probe).try_ping(&account).is_err());
}

#[test]
fn external_signer_authorizes_by_signing_auth_digest() {
    let e = Env::default();
    let (signer, signing_key) = external_signer(&e);
    let (account, probe, rule_id) = register_account(&e, &signer);

    let root_invocation = ping_invocation(&e, &probe, &account);
    let preimage = account_preimage(&e, &account, rule_id, &root_invocation);
    let sig_data = sign(&e, &signing_key, &preimage.digest(&e));

    e.set_auths(&[account_entry(&e, &account, &signer, sig_data, rule_id, root_invocation)]);

    ProbeClient::new(&e, &probe).ping(&account);
}

#[test]
fn external_signer_signing_signature_payload_fails() {
    let e = Env::default();
    let (signer, signing_key) = external_signer(&e);
    let (account, probe, rule_id) = register_account(&e, &signer);

    let root_invocation = ping_invocation(&e, &probe, &account);
    let preimage = account_preimage(&e, &account, rule_id, &root_invocation);
    let sig_data = sign(&e, &signing_key, &preimage.signature_payload);

    e.set_auths(&[account_entry(&e, &account, &signer, sig_data, rule_id, root_invocation)]);

    assert!(ProbeClient::new(&e, &probe).try_ping(&account).is_err());
}

#[test]
fn auth_digest_matches_off_chain_encoding() {
    let e = Env::default();
    let account = e.register(SmartAccountContract, ());
    let preimage = AuthDigestPreimage {
        account: account.clone(),
        signature_payload: BytesN::from_array(&e, &[7u8; 32]),
        context_rule_ids: vec![&e, 0, 1],
    };

    let scval = to_scval(&e, preimage.clone());
    let ScVal::Map(Some(map)) = &scval else { panic!("preimage must encode as a map") };
    let keys: std::vec::Vec<ScVal> = map.iter().map(|entry| entry.key.clone()).collect();
    assert_eq!(
        keys,
        std::vec![
            ScVal::Symbol("account".try_into().unwrap()),
            ScVal::Symbol("context_rule_ids".try_into().unwrap()),
            ScVal::Symbol("signature_payload".try_into().unwrap()),
        ]
    );

    let xdr = scval.to_xdr(Limits::none()).unwrap();
    let expected = e.crypto().sha256(&Bytes::from_slice(&e, &xdr)).to_bytes();
    assert_eq!(SmartAccountContractClient::new(&e, &account).auth_digest(&preimage), expected);
}
