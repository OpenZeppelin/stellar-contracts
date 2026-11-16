extern crate std;

use soroban_sdk::{contract, contractevent, contractimpl, BytesN, Env, Vec};

use crate::{
    assert_events_and_return_within, mainnet_limits, return_value_size, LIBRARY_HEADROOM_PERCENT,
};

#[contractevent]
struct Filled {
    payload: Vec<BytesN<32>>,
}

#[contract]
struct MockContract;

#[contractimpl]
impl MockContract {
    pub fn fill(e: Env, n: u32, with_event: bool) -> Vec<BytesN<32>> {
        let mut out = Vec::new(&e);
        for _ in 0..n {
            out.push_back(BytesN::from_array(&e, &[7u8; 32]));
        }
        if with_event {
            Filled { payload: out.clone() }.publish(&e);
        }
        out
    }
}

fn setup() -> (Env, MockContractClient<'static>) {
    let e = Env::default();
    let id = e.register(MockContract, ());
    let client = MockContractClient::new(&e, &id);
    (e, client)
}

#[test]
fn mainnet_limits_match_sdk_snapshot() {
    assert_eq!(mainnet_limits().contract_events_size_bytes, 16_384);
}

#[test]
fn return_value_size_counts_xdr_bytes() {
    let (e, client) = setup();
    let ret = client.fill(&3, &false);
    assert_eq!(return_value_size(&e, ret), 12 + 3 * 40);
}

#[test]
fn small_return_value_fits_headroom() {
    let (e, client) = setup();
    let ret = client.fill(&10, &false);
    assert_events_and_return_within(&e, ret, LIBRARY_HEADROOM_PERCENT);
}

#[test]
#[should_panic(expected = "exceeds 12288 B (75% of the 16384 B Mainnet")]
fn large_return_value_exceeds_headroom() {
    let (e, client) = setup();
    let ret = client.fill(&310, &false);
    assert_events_and_return_within(&e, ret, LIBRARY_HEADROOM_PERCENT);
}

#[test]
#[should_panic(expected = "exceeds 16384 B (100% of the 16384 B Mainnet")]
fn events_count_toward_the_same_limit() {
    let (e, client) = setup();
    let ret = client.fill(&210, &true);
    assert_events_and_return_within(&e, ret, 100);
}
