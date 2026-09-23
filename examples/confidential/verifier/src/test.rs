extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};
use stellar_tokens::confidential::verifier::CircuitType;

use crate::contract::{ConfidentialVerifierContract, ConfidentialVerifierContractClient};

// Real UltraHonk verification keys in the packed on-chain format, generated
// from the committed circuits by `circuits/scripts/build_vk_bins.sh`. Using
// real keys here exercises the wired UltraHonk backend end to end: a malformed
// key would be rejected by `UltraHonkVerifier::new` with `#3403`.
macro_rules! vk_bin {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../packages/tokens/src/confidential/circuits/vks/",
            $name,
            ".vk.bin"
        ))
    };
}

const REGISTER_VK: &[u8; 1760] = vk_bin!("register");
const WITHDRAW_VK: &[u8; 1760] = vk_bin!("withdraw");
const TRANSFER_VK: &[u8; 1760] = vk_bin!("transfer");
const SET_SPENDER_VK: &[u8; 1760] = vk_bin!("set_spender");
const SPENDER_TRANSFER_VK: &[u8; 1760] = vk_bin!("spender_transfer");
const CLAWBACK_VK: &[u8; 1760] = vk_bin!("clawback");

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
) -> ConfidentialVerifierContractClient<'a> {
    let address = e.register(ConfidentialVerifierContract, (admin, manager));
    ConfidentialVerifierContractClient::new(e, &address)
}

#[test]
fn register_and_get_verification_key_works() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let vk = Bytes::from_array(&e, REGISTER_VK);
    client.register_verification_key(&CircuitType::Register, &vk, &manager);

    assert_eq!(client.get_verification_key(&CircuitType::Register), vk);
}

#[test]
fn verify_proof_runs_backend_on_real_vk() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_verification_key(
        &CircuitType::Register,
        &Bytes::from_array(&e, REGISTER_VK),
        &manager,
    );

    // The key parses (no `#3403`), so the UltraHonk backend actually runs and
    // rejects a junk proof rather than panicking. A real positive case needs a
    // matching proof + public inputs produced by the prover toolchain.
    let junk = Bytes::from_array(&e, &[0u8; 32]);
    assert!(!client.verify_proof(&CircuitType::Register, &junk, &junk));
}

#[test]
fn every_committed_vk_parses_under_the_backend() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    // The backend validates header ranges and every G1 commitment when the
    // key is loaded, so a `verify_proof` that returns `false` instead of
    // panicking with `#3403` proves each committed key is accepted as-is.
    let junk = Bytes::from_array(&e, &[0u8; 32]);
    for (circuit, vk) in [
        (CircuitType::Register, REGISTER_VK),
        (CircuitType::Withdraw, WITHDRAW_VK),
        (CircuitType::Transfer, TRANSFER_VK),
        (CircuitType::SetSpender, SET_SPENDER_VK),
        (CircuitType::SpenderTransfer, SPENDER_TRANSFER_VK),
        (CircuitType::Clawback, CLAWBACK_VK),
    ] {
        client.register_verification_key(&circuit, &Bytes::from_array(&e, vk), &manager);
        assert!(!client.verify_proof(&circuit, &junk, &junk));
    }
}

#[test]
fn update_verification_key_replaces_in_place() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    let old = Bytes::from_array(&e, REGISTER_VK);
    let new = Bytes::from_array(&e, WITHDRAW_VK);

    client.register_verification_key(&CircuitType::Register, &old, &manager);
    client.update_verification_key(&CircuitType::Register, &new, &manager);

    assert_eq!(client.get_verification_key(&CircuitType::Register), new);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn register_by_non_manager_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let stranger = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    client.register_verification_key(
        &CircuitType::Register,
        &Bytes::from_array(&e, REGISTER_VK),
        &stranger,
    );
}
