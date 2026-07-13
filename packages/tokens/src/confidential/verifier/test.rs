extern crate std;

use soroban_sdk::{contract, testutils::Events, Bytes, Env};

use crate::confidential::verifier::{
    storage::{
        get_verification_key, register_verification_key, update_verification_key,
        VerifierStorageKey,
    },
    CircuitType,
};

#[contract]
struct MockContract;

fn verification_key_bytes(e: &Env, seed: u8) -> Bytes {
    Bytes::from_array(e, &[seed; 32])
}

#[test]
fn register_and_get_verification_key_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verification_key = verification_key_bytes(&e, 0xab);

    e.as_contract(&address, || {
        register_verification_key(&e, CircuitType::Register, &verification_key);

        assert_eq!(get_verification_key(&e, CircuitType::Register), verification_key);

        assert_eq!(e.events().all().events().len(), 1);
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
            CircuitType::SpenderTransfer,
            CircuitType::SetSpender,
            CircuitType::RevokeSpender,
        ]
        .into_iter()
        .enumerate()
        {
            let verification_key = verification_key_bytes(&e, i as u8);
            register_verification_key(&e, circuit, &verification_key);
            assert_eq!(get_verification_key(&e, circuit), verification_key);
        }
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3400)")]
fn register_twice_panics_with_already_registered() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        register_verification_key(&e, CircuitType::Withdraw, &verification_key_bytes(&e, 0x01));
        register_verification_key(&e, CircuitType::Withdraw, &verification_key_bytes(&e, 0x02));
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
        let old = verification_key_bytes(&e, 0x10);
        let new = verification_key_bytes(&e, 0x20);

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
        update_verification_key(&e, CircuitType::SetSpender, &verification_key_bytes(&e, 0xff));
    });
}

#[test]
fn update_emits_update_event() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let old = verification_key_bytes(&e, 0x10);
        let new = verification_key_bytes(&e, 0x20);

        register_verification_key(&e, CircuitType::SpenderTransfer, &old);
        update_verification_key(&e, CircuitType::SpenderTransfer, &new);

        // 2 events: VerificationKeyRegistered + VerificationKeyUpdated.
        assert_eq!(e.events().all().events().len(), 2);
    });
}

#[test]
fn storage_key_round_trip() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let verification_key = verification_key_bytes(&e, 0x77);
        register_verification_key(&e, CircuitType::RevokeSpender, &verification_key);

        let stored: Bytes = e
            .storage()
            .instance()
            .get(&VerifierStorageKey::VerificationKey(CircuitType::RevokeSpender))
            .unwrap();
        assert_eq!(stored, verification_key);
    });
}
