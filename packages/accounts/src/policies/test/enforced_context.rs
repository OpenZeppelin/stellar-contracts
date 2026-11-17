extern crate std;

use soroban_sdk::{
    auth::{
        Context, ContractExecutable, ContractExecutableRef, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    testutils::Address as _,
    xdr::{Limits, ToXdr, WriteXdr},
    Address, Bytes, BytesN, Env, Event, IntoVal, String, Vec,
};

use crate::{
    policies::{
        simple_threshold::SimpleEnforced, test::ENFORCED_EVENT_SIZE_CEILING_BYTES,
        weighted_threshold::WeightedEnforced, EnforcedContext, EnforcedExecutable,
    },
    smart_account::MAX_SIGNERS,
};

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

    let without_projection = EnforcedContext::new(&e, &without_constructor);
    let with_projection = EnforcedContext::new(&e, &with_constructor);

    assert_eq!(without_projection.clone().to_xdr(&e), with_projection.to_xdr(&e));

    match without_projection {
        EnforcedContext::CreateContract(EnforcedExecutable::Wasm(actual_hash), actual_salt) => {
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

    let without_projection = EnforcedContext::new(&e, &without_constructor);
    let with_projection = EnforcedContext::new(&e, &with_constructor);

    assert_eq!(without_projection.clone().to_xdr(&e), with_projection.to_xdr(&e));

    match without_projection {
        EnforcedContext::CreateContract(
            EnforcedExecutable::ExternalRef(actual_owner, actual_tag_hash),
            actual_salt,
        ) => {
            assert_eq!(actual_owner, owner);
            assert_eq!(actual_tag_hash, e.crypto().sha256(&tag.to_bytes()).to_bytes());
            assert_eq!(actual_salt, salt);
        }
        _ => panic!("expected an external-ref contract-creation projection"),
    }
}

#[test]
fn external_ref_event_size_does_not_depend_on_tag() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let smart_account = Address::generate(&e);
    let policy = Address::generate(&e);
    let salt = BytesN::from_array(&e, &[3; 32]);
    let signer_ids = Vec::from_iter(&e, 0..MAX_SIGNERS);

    let sizes = [0, 20_000].map(|tag_len| {
        let executable = ContractExecutable::ExternalRef(ContractExecutableRef {
            owner: owner.clone(),
            tag: String::from_bytes(&e, &std::vec![b'x'; tag_len]),
        });
        let contexts = [
            Context::CreateContractHostFn(CreateContractHostFnContext {
                executable: executable.clone(),
                salt: salt.clone(),
            }),
            Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
                executable,
                salt: salt.clone(),
                constructor_args: Vec::from_array(
                    &e,
                    [Bytes::from_slice(&e, &std::vec![0; 20_000]).into_val(&e)],
                ),
            }),
        ];

        contexts.map(|context| {
            [
                SimpleEnforced {
                    smart_account: smart_account.clone(),
                    context_rule_id: u32::MAX,
                    context: EnforcedContext::new(&e, &context),
                    signer_ids: signer_ids.clone(),
                }
                .to_xdr(&e, &policy),
                WeightedEnforced {
                    smart_account: smart_account.clone(),
                    context_rule_id: u32::MAX,
                    context: EnforcedContext::new(&e, &context),
                    signer_ids: signer_ids.clone(),
                }
                .to_xdr(&e, &policy),
            ]
            .map(|event| WriteXdr::to_xdr(&event, Limits::none()).unwrap().len())
        })
    });

    assert_eq!(sizes[0], sizes[1]);
    for event_size in sizes[1].into_iter().flatten() {
        assert!(
            event_size < ENFORCED_EVENT_SIZE_CEILING_BYTES,
            "enforcement event is {event_size} bytes"
        );
    }
}
