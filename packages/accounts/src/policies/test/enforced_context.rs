extern crate std;

use soroban_sdk::{
    auth::{
        Context, ContractExecutable, ContractExecutableRef, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    testutils::Address as _,
    xdr::ToXdr,
    Address, BytesN, Env, IntoVal, String, Vec,
};

use super::ENFORCED_EVENT_SIZE_CEILING_BYTES;
use crate::policies::EnforcedContext;

#[test]
fn projects_contract_creation_variants_identically() {
    let e = Env::default();
    let wasm_hash = BytesN::from_array(&e, &[1; 32]);
    let salt = BytesN::from_array(&e, &[2; 32]);
    let without_constructor = Context::CreateContractHostFn(CreateContractHostFnContext {
        executable: ContractExecutable::Wasm(wasm_hash.clone()),
        salt: salt.clone(),
    });
    let with_constructor =
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable: ContractExecutable::Wasm(wasm_hash.clone()),
            salt: salt.clone(),
            constructor_args: Vec::from_array(&e, [1_i128.into_val(&e)]),
        });

    let without_projection = EnforcedContext::from(&without_constructor);
    let with_projection = EnforcedContext::from(&with_constructor);

    assert_eq!(without_projection.clone().to_xdr(&e), with_projection.to_xdr(&e));

    match without_projection {
        EnforcedContext::CreateContract(ContractExecutable::Wasm(actual_hash), actual_salt) => {
            assert_eq!(actual_hash, wasm_hash);
            assert_eq!(actual_salt, salt);
        }
        _ => panic!("expected a projected contract-creation context"),
    }
}

#[test]
fn projects_external_ref_creation_variants_identically() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let tag = String::from_str(&e, "release-v1");
    let salt = BytesN::from_array(&e, &[3; 32]);
    let executable = ContractExecutable::ExternalRef(ContractExecutableRef {
        owner: owner.clone(),
        tag: tag.clone(),
    });
    let without_constructor = Context::CreateContractHostFn(CreateContractHostFnContext {
        executable: executable.clone(),
        salt: salt.clone(),
    });
    let with_constructor =
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable,
            salt: salt.clone(),
            constructor_args: Vec::from_array(&e, [1_i128.into_val(&e)]),
        });

    let without_projection = EnforcedContext::from(&without_constructor);
    let with_projection = EnforcedContext::from(&with_constructor);

    assert_eq!(without_projection.clone().to_xdr(&e), with_projection.to_xdr(&e));
    assert!(without_projection.clone().to_xdr(&e).len() < ENFORCED_EVENT_SIZE_CEILING_BYTES as u32);

    match without_projection {
        EnforcedContext::CreateContract(
            ContractExecutable::ExternalRef(actual_ref),
            actual_salt,
        ) => {
            assert_eq!(actual_ref.owner, owner);
            assert_eq!(actual_ref.tag, tag);
            assert_eq!(actual_salt, salt);
        }
        _ => panic!("expected an external-ref contract-creation projection"),
    }
}
