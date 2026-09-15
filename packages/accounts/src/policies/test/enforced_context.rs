extern crate std;

use soroban_sdk::{
    auth::{
        Context, ContractExecutable, ContractExecutableRef, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    testutils::Address as _,
    xdr::{
        ContractDataDurability, LedgerKey, LedgerKeyContractData, Limits, ScString, ScVal, ToXdr,
        WriteXdr,
    },
    Address, Bytes, BytesN, Env, Event, IntoVal, String, Vec,
};

use crate::{
    policies::{
        simple_threshold::SimpleEnforced, test::ENFORCED_EVENT_SIZE_CEILING_BYTES,
        weighted_threshold::WeightedEnforced, EnforcedContext,
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

#[test]
fn external_ref_events_fit_at_ledger_key_size_boundary() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let smart_account = Address::generate(&e);
    let policy = Address::generate(&e);
    let salt = BytesN::from_array(&e, &[3; 32]);

    // Fixture for a 250-byte ledger-key limit. XDR padding makes 196 bytes
    // the largest tag that fits: the full keys are 248 and 252 bytes.
    // This is a test assumption, not a tag limit imposed by the policy.
    const LEDGER_KEY_SIZE_LIMIT_BYTES: usize = 250;
    let tag_bytes = std::vec![b'x'; 196];
    let key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: (&owner).into(),
        key: ScVal::ExecutableTag(ScString(tag_bytes.clone().try_into().unwrap())),
        durability: ContractDataDurability::Persistent,
    });
    let oversized_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: (&owner).into(),
        key: ScVal::ExecutableTag(ScString(std::vec![b'x'; 197].try_into().unwrap())),
        durability: ContractDataDurability::Persistent,
    });
    assert!(WriteXdr::to_xdr(&key, Limits::none()).unwrap().len() <= LEDGER_KEY_SIZE_LIMIT_BYTES);
    assert!(
        WriteXdr::to_xdr(&oversized_key, Limits::none()).unwrap().len()
            > LEDGER_KEY_SIZE_LIMIT_BYTES
    );

    let executable = ContractExecutable::ExternalRef(ContractExecutableRef {
        owner,
        tag: String::from_bytes(&e, &tag_bytes),
    });
    let contexts = [
        Context::CreateContractHostFn(CreateContractHostFnContext {
            executable: executable.clone(),
            salt: salt.clone(),
        }),
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable,
            salt,
            constructor_args: Vec::from_array(
                &e,
                [Bytes::from_slice(&e, &std::vec![0; 20_000]).into_val(&e)],
            ),
        }),
    ];
    let signer_ids = Vec::from_iter(&e, 0..MAX_SIGNERS);

    for context in contexts {
        // Measure complete events for both threshold policies, including
        // topics and the maximum signer count, not just the projection.
        let events = [
            SimpleEnforced {
                smart_account: smart_account.clone(),
                context_rule_id: u32::MAX,
                context: (&context).into(),
                signer_ids: signer_ids.clone(),
            }
            .to_xdr(&e, &policy),
            WeightedEnforced {
                smart_account: smart_account.clone(),
                context_rule_id: u32::MAX,
                context: (&context).into(),
                signer_ids: signer_ids.clone(),
            }
            .to_xdr(&e, &policy),
        ];
        for event in events {
            let event_size = WriteXdr::to_xdr(&event, Limits::none()).unwrap().len();
            assert!(
                event_size < ENFORCED_EVENT_SIZE_CEILING_BYTES,
                "enforcement event is {event_size} bytes"
            );
        }
    }
}

#[test]
fn external_ref_event_size_still_grows_with_tag() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let smart_account = Address::generate(&e);
    let policy = Address::generate(&e);
    let salt = BytesN::from_array(&e, &[3; 32]);

    // The projection retains tags even when they cannot form a valid ledger
    // key. Authorization contexts can include unused child deployments, so
    // eventual validation by a deployment host function is not a size bound.
    let sizes = [0, 20_000].map(|tag_len| {
        let context = Context::CreateContractHostFn(CreateContractHostFnContext {
            executable: ContractExecutable::ExternalRef(ContractExecutableRef {
                owner: owner.clone(),
                tag: String::from_bytes(&e, &std::vec![b'x'; tag_len]),
            }),
            salt: salt.clone(),
        });
        let event = SimpleEnforced {
            smart_account: smart_account.clone(),
            context_rule_id: 0,
            context: (&context).into(),
            signer_ids: Vec::from_array(&e, [0]),
        }
        .to_xdr(&e, &policy);
        WriteXdr::to_xdr(&event, Limits::none()).unwrap().len()
    });

    assert_eq!(sizes[1] - sizes[0], 20_000);
    assert!(sizes[1] > 16_384);
}
