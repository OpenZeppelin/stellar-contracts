#![cfg(test)]

//extern crate std;

//use soroban_sdk::{contract, testutils::Address as _, Address, Bytes, Env,
// Vec};

//use super::storage::Signer;

//#[contract]
//struct MockContract;

//fn create_test_signers(e: &Env) -> (Vec<Signer>, Vec<Signer>) {
//let addr1 = Address::generate(e);
//let addr2 = Address::generate(e);
//let addr3 = Address::generate(e);
//let addr4 = Address::generate(e);

//let verifier_addr = Address::generate(e);
//let pub_key = Bytes::from_array(e, &[1, 2, 3, 4]);

//let rule_signers = Vec::from_array(
//e,
//[
//Signer::Native(addr1.clone()),
//Signer::Native(addr2.clone()),
//Signer::Delegated(verifier_addr.clone(), pub_key.clone()),
//],
//);

//let provided_signers = Vec::from_array(
//e,
//[
//Signer::Native(addr1.clone()),
//Signer::Native(addr3.clone()),
//Signer::Delegated(verifier_addr.clone(), pub_key.clone()),
//Signer::Native(addr4.clone()),
//],
//);

//(rule_signers, provided_signers)
//}
