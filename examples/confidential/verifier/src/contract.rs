//! Confidential Verifier Example Contract.
//!
//! A deployable [`ConfidentialVerifier`] registry: it stores one UltraHonk
//! verification key per [`CircuitType`] and exposes `verify_proof` to the
//! confidential token (called cross-contract on every state-changing
//! operation). VK management is gated behind a `manager` role; `verify_proof`
//! and `get_verification_key` use the trait's default implementations, which
//! run the UltraHonk backend from `NethermindEth/rs-soroban-ultrahonk`.
//!
//! # ⚠️ Not Production Ready
//!
//! The UltraHonk backend and the circuits the verification keys are derived
//! from are **not audited**. Do not deploy this anywhere handling real value.
//!
//! # Security
//!
//! `update_verification_key` is a soundness-critical break-glass operation: a
//! wrong key makes the circuit accept forged proofs. This example gates it
//! behind the same `manager` role as registration purely for illustration. A
//! real deployment should follow the trait's guidance — ship VKs immutably
//! where possible, and put any update path behind multisig + timelock.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Bytes, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::confidential::verifier::{
    storage as verifier, CircuitType, ConfidentialVerifier,
};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct ConfidentialVerifierContract;

#[contractimpl]
impl ConfidentialVerifierContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialVerifier for ConfidentialVerifierContract {
    #[only_role(operator, "manager")]
    fn register_verification_key(e: &Env, circuit_type: CircuitType, vk: Bytes, operator: Address) {
        verifier::register_verification_key(e, circuit_type, &vk);
    }

    #[only_role(operator, "manager")]
    fn update_verification_key(
        e: &Env,
        circuit_type: CircuitType,
        new_vk: Bytes,
        operator: Address,
    ) {
        verifier::update_verification_key(e, circuit_type, &new_vk);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ConfidentialVerifierContract {}
