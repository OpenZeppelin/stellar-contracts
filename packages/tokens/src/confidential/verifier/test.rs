extern crate std;

use soroban_sdk::{contract, Bytes, Env};
use stellar_event_assertion::EventAssertion;

use crate::confidential::verifier::{
    storage::{
        get_verification_key, register_verification_key, update_verification_key,
        VerifierStorageKey,
    },
    CircuitType,
};

#[contract]
struct MockContract;

fn vk_bytes(e: &Env, seed: u8) -> Bytes {
    Bytes::from_array(e, &[seed; 32])
}

#[test]
fn register_and_get_verification_key_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let vk = vk_bytes(&e, 0xab);

    e.as_contract(&address, || {
        register_verification_key(&e, CircuitType::Register, &vk);

        assert_eq!(get_verification_key(&e, CircuitType::Register), vk);

        EventAssertion::new(&e, address.clone()).assert_event_count(1);
    });
}

#[test]
fn register_each_circuit_round_trips() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        for (i, circuit) in [
            CircuitType::Register,
            CircuitType::Withdraw,
            CircuitType::Transfer,
            CircuitType::OperatorTransfer,
            CircuitType::SetOperator,
            CircuitType::RevokeOperator,
        ]
        .into_iter()
        .enumerate()
        {
            let vk = vk_bytes(&e, i as u8);
            register_verification_key(&e, circuit, &vk);
            assert_eq!(get_verification_key(&e, circuit), vk);
        }
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3400)")]
fn register_twice_panics_with_already_registered() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_verification_key(&e, CircuitType::Withdraw, &vk_bytes(&e, 0x01));
        register_verification_key(&e, CircuitType::Withdraw, &vk_bytes(&e, 0x02));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3401)")]
fn get_unregistered_panics_with_not_registered() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let _ = get_verification_key(&e, CircuitType::Transfer);
    });
}

#[test]
fn update_verification_key_replaces_in_place() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let old = vk_bytes(&e, 0x10);
        let new = vk_bytes(&e, 0x20);

        register_verification_key(&e, CircuitType::Transfer, &old);
        update_verification_key(&e, CircuitType::Transfer, &new);

        assert_eq!(get_verification_key(&e, CircuitType::Transfer), new);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3401)")]
fn update_unregistered_panics_with_not_registered() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        update_verification_key(&e, CircuitType::SetOperator, &vk_bytes(&e, 0xff));
    });
}

#[test]
fn update_emits_update_event() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let old = vk_bytes(&e, 0x10);
        let new = vk_bytes(&e, 0x20);

        register_verification_key(&e, CircuitType::OperatorTransfer, &old);
        update_verification_key(&e, CircuitType::OperatorTransfer, &new);

        // 2 events: VerificationKeyRegistered + VerificationKeyUpdated.
        EventAssertion::new(&e, address.clone()).assert_event_count(2);
    });
}

#[test]
fn storage_key_round_trip() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let vk = vk_bytes(&e, 0x77);
        register_verification_key(&e, CircuitType::RevokeOperator, &vk);

        let stored: Bytes = e
            .storage()
            .instance()
            .get(&VerifierStorageKey::Vk(CircuitType::RevokeOperator))
            .unwrap();
        assert_eq!(stored, vk);
    });
}
