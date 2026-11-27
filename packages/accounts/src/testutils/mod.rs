//! Native test helpers for custom policies. Enable the `testutils` feature
//! on `stellar-accounts` as a dev-dependency.
//!
//! [`PolicyTestHarness`] installs policies on a test account and exercises
//! its real authorization hook with a selected subset of Ed25519 signers.

extern crate alloc;

use ed25519_dalek::{Signer as _, SigningKey};
use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractimpl,
    crypto::Hash,
    vec, Address, Bytes, BytesN, Env, Error, IntoVal, Map, String, Val, Vec,
};

use crate::{
    smart_account::{
        add_context_rule, do_check_auth, AuthDigestPreimage, AuthPayload, ContextRuleType, Signer,
        SmartAccountError,
    },
    verifiers::{ed25519, Verifier},
};

#[cfg(test)]
mod test;

#[contract]
struct PolicyTestAccount;

#[contractimpl]
impl CustomAccountInterface for PolicyTestAccount {
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

#[contract]
pub(crate) struct TestEd25519Verifier;

#[contractimpl]
impl Verifier for TestEd25519Verifier {
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

/// A native Soroban test account with one context rule and Ed25519 signers.
///
/// Each check calls the account's real `__check_auth` directly, with a
/// synthetic host payload and a single supplied authorization context. Policy
/// state persists across successful checks. Use a fresh [`Env`] without
/// mocked authorization.
pub struct PolicyTestHarness {
    env: Env,
    account: Address,
    context_rule_id: u32,
    signers: alloc::vec::Vec<(Signer, SigningKey)>,
}

impl PolicyTestHarness {
    /// Registers a test account and installs the policies on one context rule.
    ///
    /// # Arguments
    /// * `env` - The environment in which the policies are already registered.
    /// * `context_type` - The context type the rule accepts.
    /// * `signer_seeds` - Distinct Ed25519 seeds in signer index order. Use
    ///   dedicated test seeds.
    /// * `policies` - Builds the map of policy addresses to installation
    ///   parameters from the rule signers in seed order. Use this to configure
    ///   policies whose installation parameters refer to specific signers.
    ///
    /// # Panics
    /// Panics if the smart account rejects the rule or a policy rejects
    /// installation.
    pub fn new(
        env: &Env,
        context_type: &ContextRuleType,
        signer_seeds: &[[u8; 32]],
        policies: impl FnOnce(&Vec<Signer>) -> Map<Address, Val>,
    ) -> Self {
        let account = env.register(PolicyTestAccount, ());
        let verifier = env.register(TestEd25519Verifier, ());
        let signers: alloc::vec::Vec<_> = signer_seeds
            .iter()
            .map(|seed| {
                let key = SigningKey::from_bytes(seed);
                let public_key = Bytes::from_array(env, key.verifying_key().as_bytes());
                (Signer::External(verifier.clone(), public_key), key)
            })
            .collect();
        let rule_signers = Vec::from_iter(env, signers.iter().map(|(signer, _)| signer.clone()));
        let policies = policies(&rule_signers);
        let context_rule_id = env.as_contract(&account, || {
            add_context_rule(
                env,
                context_type,
                &String::from_str(env, "policy test"),
                None,
                &rule_signers,
                &policies,
            )
            .id
        });
        Self { env: env.clone(), account, context_rule_id, signers }
    }

    /// Returns the address of the registered smart account.
    pub fn account(&self) -> &Address {
        &self.account
    }

    /// Returns the ID of the installed context rule.
    pub fn context_rule_id(&self) -> u32 {
        self.context_rule_id
    }

    /// Signs with the selected zero-based signer indices and invokes
    /// `__check_auth`. Pass an empty slice to test unsigned authorization.
    ///
    /// # Arguments
    /// * `context` - The authorization context to pass to the account.
    /// * `signer_indices` - The indices of the test signers to authenticate. A
    ///   repeated index contributes only one signature.
    ///
    /// # Errors
    /// Returns the Soroban error if the authorization check fails.
    ///
    /// # Panics
    /// Panics if a signer index is out of range.
    pub fn try_check_auth(&self, context: Context, signer_indices: &[usize]) -> Result<(), Error> {
        let e = &self.env;
        let signature_payload = BytesN::from_array(e, &[0; 32]);
        let context_rule_ids = vec![e, self.context_rule_id];
        let digest = AuthDigestPreimage {
            account: self.account.clone(),
            signature_payload: signature_payload.clone(),
            context_rule_ids: context_rule_ids.clone(),
        }
        .digest(e)
        .to_array();
        let mut signers = Map::new(e);
        for &index in signer_indices {
            let (signer, key) = &self.signers[index];
            let signature = Bytes::from_array(e, &key.sign(&digest).to_bytes());
            signers.set(signer.clone(), signature);
        }
        e.try_invoke_contract_check_auth::<Error>(
            &self.account,
            &signature_payload,
            AuthPayload { signers, context_rule_ids }.into_val(e),
            &vec![e, context],
        )
        .map_err(|error| error.expect("conversion from a Soroban error to itself is infallible"))
    }
}
