#![cfg(test)]

use hex_literal::hex;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, vec, Address, BytesN, Env};
use stellar_proc_macros::default_impl;
use stellar_tokens::fungible::{Base, FungibleToken};

use crate::contract::{AirdropContract, AirdropContractClient};

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
        Base::mint(e, &owner, initial_supply);
    }

    pub fn mint(e: &Env, to: Address, amount: i128) {
        Base::mint(e, &to, amount);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for TokenContract {
    type ContractType = Base;
}

fn create_token_contract<'a>(e: &Env, owner: &Address) -> TokenContractClient<'a> {
    let address = e.register(TokenContract, (owner, 10_000i128));
    TokenContractClient::new(e, &address)
}

fn make_args(
    e: &Env,
    hash_bytes: [u8; 32],
    token: Address,
    funding_amount: i128,
    funding_source: Address,
) -> (BytesN<32>, Address, i128, Address) {
    let root_hash = BytesN::from_array(e, &hash_bytes);

    (root_hash, token, funding_amount, funding_source)
}

fn hex_to_bytes(e: &Env, hash_bytes: [u8; 32]) -> BytesN<32> {
    BytesN::from_array(e, &hash_bytes)
}

#[test]
fn test_valid_claim() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let constructor_args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        1000,
        owner.clone(),
    );

    let contract_id = e.register(AirdropContract, constructor_args);
    let client = AirdropContractClient::new(&e, &contract_id);

    let receiver =
        Address::from_str(&e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let amount = 100;
    let proofs = vec![
        &e,
        hex_to_bytes(&e, hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7")),
        hex_to_bytes(&e, hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4")),
    ];

    client.claim(&3_u32, &receiver, &amount, &proofs);
    assert_eq!(token_client.balance(&receiver), 100);
    assert_eq!(token_client.balance(&contract_id), 900);
}

#[test]
fn test_double_claim() {
    let e: Env = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        1000,
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);

    let receiver =
        Address::from_str(&e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let amount: i128 = 100;
    let proofs = vec![
        &e,
        hex_to_bytes(&e, hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7")),
        hex_to_bytes(&e, hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4")),
    ];

    client.claim(&3_u32, &receiver, &amount, &proofs);
    let second_claim = client.try_claim(&3_u32, &receiver, &amount, &proofs);

    assert!(second_claim.is_err());
}
#[test]
fn test_claimed_not_reset() {
    let e: Env = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("9ecccb575ce934ab36a6db174e9f521137c942422b76332b047b49f5a1a58048"),
        token_client.address.clone(),
        1000,
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);

    let receiver_1 =
        Address::from_str(&e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let amount_1: i128 = 100;
    let proofs_1 = vec![
        &e,
        hex_to_bytes(&e, hex!("cd9bbfb141e8c63b620238d79aabfbe5eaf16309874b3f32fc443b4f477c9b2f")),
        hex_to_bytes(&e, hex!("ae7ed9c150e2d582d1db0a32dc7370c00a22405324e5b5f1c9272e57274a08f4")),
        hex_to_bytes(&e, hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7")),
    ];

    let receiver_2 =
        Address::from_str(&e, "CCAYN4HGXBYMAREFANQKKRNCIPLXYGXT7OVXDXG6APXBGKJPKARAOHAK");
    let amount_2: i128 = 100;
    let proofs_2 = vec![
        &e,
        hex_to_bytes(&e, hex!("bab7bc2e36db8910a5e047989f1bfb6791bb8a2d3b3218fd363969294aaac83e")),
        hex_to_bytes(&e, hex!("c8b6359bcd036ed19bff1e307c7f0eeb410ec193a5a4647f7cf36fdba86af070")),
        hex_to_bytes(&e, hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7")),
    ];

    client.claim(&3_u32, &receiver_1, &amount_1, &proofs_1);
    client.claim(&4_u32, &receiver_2, &amount_2, &proofs_2);
    let second_receiver_1_claim = client.try_claim(&3_u32, &receiver_1, &amount_2, &proofs_2);

    assert!(second_receiver_1_claim.is_err());
}

#[test]
fn test_bad_claim() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let owner = Address::generate(&e);
    let token_client = create_token_contract(&e, &owner);

    let args = make_args(
        &e,
        hex!("11932105f1a4d0092e87cead3a543da5afd8adcff63f9a8ceb6c5db3c8135722"),
        token_client.address.clone(),
        1000,
        owner.clone(),
    );
    let contract_id = e.register(AirdropContract, args);
    let client = AirdropContractClient::new(&e, &contract_id);

    let receiver =
        Address::from_str(&e, "CAASCQKVVBSLREPEUGPOTQZ4BC2NDBY2MW7B2LGIGFUPIY4Z3XUZRVTX");
    let amount = 100000; // This is a different amount
    let proofs = vec![
        &e,
        hex_to_bytes(&e, hex!("fc0d9c2f46c1e910bd3af8665318714c7c97486d2a206f96236c6e7e50c080d7")),
        hex_to_bytes(&e, hex!("c83f7b26055572e5e84c78ec4d4f45b85b71698951077baafe195279c1f30be4")),
    ];

    let claim = client.try_claim(&3_u32, &receiver, &amount, &proofs);

    assert!(claim.is_err());
}
