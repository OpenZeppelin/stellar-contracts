extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::identity_registry_storage::{IdentityRegistryContract, IdentityRegistryContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> IdentityRegistryContractClient<'a> {
    let address = e.register(IdentityRegistryContract, (admin, manager));
    IdentityRegistryContractClient::new(e, &address)
}

#[test]
fn bind_max() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();

    let mut tokens = vec![&e];
    for _ in 0..200 {
        let token = Address::generate(&e);
        tokens.push_back(token.clone());
    }

    client.bind_tokens(&tokens, &manager);
    assert_eq!(client.linked_tokens().len(), 200)
}

// TODO: add test for checking `recovery_address` fails when contract is paused

// TODO: add below tests

// #[test]
// #[should_panic(expected = "Error(Contract, #304)")]
// fn mint_fails_when_not_same_claim_topic() {
//     let e = Env::default();
//     let address = e.register(MockRWAContract, ());
//     let to = Address::generate(&e);

//     e.as_contract(&address, || {
//         let (identity, issuer, ..) = set_and_return_identity_verifier(&e);
//         e.as_contract(&identity, || {
//             let claim = construct_claim(&e, &issuer, 2);
//             e.storage().persistent().set(&symbol_short!("claim"), &claim);
//         });

//         set_and_return_compliance(&e);

//         RWA::mint(&e, &to, 100);
//     });
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #304)")]
// fn mint_fails_when_not_same_issuers() {
//     let e = Env::default();
//     let address = e.register(MockRWAContract, ());
//     let to = Address::generate(&e);

//     e.as_contract(&address, || {
//         let (identity, ..) = set_and_return_identity_verifier(&e);
//         let other_issuer = e.register(MockClaimIssuer, ());
//         e.as_contract(&identity, || {
//             let claim = construct_claim(&e, &other_issuer, 1);
//             e.storage().persistent().set(&symbol_short!("claim"), &claim);
//         });

//         set_and_return_compliance(&e);

//         RWA::mint(&e, &to, 100);
//     });
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #304)")]
// fn mint_fails_when_claim_not_valid() {
//     let e = Env::default();
//     let address = e.register(MockRWAContract, ());
//     let to = Address::generate(&e);

//     e.as_contract(&address, || {
//         let (_, claim_issuer, ..) = set_and_return_identity_verifier(&e);
//         e.as_contract(&claim_issuer, || {
//             e.storage().persistent().set(&symbol_short!("claim_ok"), &false)
//         });

//         set_and_return_compliance(&e);

//         RWA::mint(&e, &to, 100);
//     });
// }

// #[test]
// #[should_panic(expected = "Error(Contract, #304)")]
// fn mint_fails_with_claim_issuer_conversion_error() {
//     let e = Env::default();
//     let address = e.register(MockRWAContract, ());
//     let to = Address::generate(&e);

//     e.as_contract(&address, || {
//         let (_, claim_issuer, ..) = set_and_return_identity_verifier(&e);
//         // Claim issuer returns invalid data, u32 instead of bool
//         e.as_contract(&claim_issuer, || {
//             e.storage().persistent().set(&symbol_short!("claim_ok"), &12u32)
//         });

//         set_and_return_compliance(&e);

//         RWA::mint(&e, &to, 100);
//     });
// }

// #[test]
// fn mint_with_two_claim_issuers() {
//     let e = Env::default();
//     let address = e.register(MockRWAContract, ());
//     let to = Address::generate(&e);

//     e.as_contract(&address, || {
//         let (identity, claim_issuer, _, cti) =
// set_and_return_identity_verifier(&e);

//         // First claim issuer returns invalid data, u32 instead of bool
//         e.as_contract(&claim_issuer, || {
//             e.storage().persistent().set(&symbol_short!("claim_ok"), &12u32)
//         });

//         // Second claim issuer returns claim is valid
//         let claim_issuer_2 = e.register(MockClaimIssuer, ());
//         e.as_contract(&identity, || {
//             let claim = construct_claim(&e, &claim_issuer_2, 1);
//             e.storage().persistent().set(&symbol_short!("claim"), &claim);
//         });
//         e.as_contract(&cti, || {
//             e.storage()
//                 .persistent()
//                 .set(&symbol_short!("issuers"), &vec![&e,
// claim_issuer.clone(), claim_issuer_2]);         });

//         set_and_return_compliance(&e);

//         RWA::mint(&e, &to, 100);
//     });
// }
