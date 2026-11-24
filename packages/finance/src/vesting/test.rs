extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{
        storage::Persistent, Address as _, Events, IssuerFlags, Ledger, StellarAssetContract,
    },
    token::{StellarAssetClient, TokenClient},
    Address, Env, Error, Event, MuxedAddress,
};
use stellar_access::ownable::{set_owner, Ownable};

use crate::vesting::{
    get_released, linear_vesting_curve, set_schedule, Released, VestingError, VestingSchedule,
    VestingScheduleSet, VestingStorageKey, VestingWallet, RELEASED_EXTEND_AMOUNT,
};

// Default timeline used by most tests: cliff at 1_100, end at 2_000.
const START: u64 = 1_000;
const DURATION: u64 = 1_000;
const CLIFF_DURATION: u64 = 100;
const CLIFF: u64 = START + CLIFF_DURATION;
const END: u64 = START + DURATION;

// ################## TEST CONTRACTS ##################

/// A vesting contract with the default (linear) curve and the library's
/// default `Ownable`, so renouncing is allowed.
#[contract]
struct VestingMock;

#[contractimpl]
impl VestingMock {
    pub fn __constructor(
        e: &Env,
        receiver: Address,
        start: u64,
        duration: u64,
        cliff_duration: u64,
    ) {
        set_owner(e, &receiver);
        set_schedule(e, start, duration, cliff_duration);
    }
}

#[contractimpl(contracttrait)]
impl VestingWallet for VestingMock {}

#[contractimpl(contracttrait)]
impl Ownable for VestingMock {}

/// A vesting contract whose schedule was never set.
#[contract]
struct NoScheduleMock;

#[contractimpl(contracttrait)]
impl VestingWallet for NoScheduleMock {}

/// Which curve `CurveMock` uses.
#[contracttype]
#[derive(Clone, Copy)]
enum CurveMode {
    /// Returns `total_allocation + 1`.
    AboveTotal,
    /// Returns `-1`.
    Negative,
    /// Returns `0`.
    Zero,
    /// Returns `total_allocation`.
    Total,
    /// Linear until the stored timestamp, `0` from then on.
    DecreasingAt(u64),
    /// Linear, but panics if called outside `[cliff, end)`.
    Guarded,
}

#[contracttype]
enum CurveMockKey {
    Mode,
}

/// A vesting contract with a custom (possibly misbehaving) curve.
#[contract]
struct CurveMock;

#[contractimpl]
impl CurveMock {
    pub fn __constructor(
        e: &Env,
        receiver: Address,
        start: u64,
        duration: u64,
        cliff_duration: u64,
        mode: CurveMode,
    ) {
        set_owner(e, &receiver);
        set_schedule(e, start, duration, cliff_duration);
        e.storage().instance().set(&CurveMockKey::Mode, &mode);
    }
}

#[contractimpl(contracttrait)]
impl VestingWallet for CurveMock {
    fn vesting_curve(
        e: &Env,
        schedule: VestingSchedule,
        total_allocation: i128,
        timestamp: u64,
    ) -> i128 {
        let mode: CurveMode = e.storage().instance().get(&CurveMockKey::Mode).unwrap();
        match mode {
            CurveMode::AboveTotal => total_allocation + 1,
            CurveMode::Negative => -1,
            CurveMode::Zero => 0,
            CurveMode::Total => total_allocation,
            CurveMode::DecreasingAt(at) if timestamp >= at => 0,
            CurveMode::DecreasingAt(_) => {
                linear_vesting_curve(e, &schedule, total_allocation, timestamp)
            }
            CurveMode::Guarded => {
                if timestamp < schedule.cliff || timestamp >= schedule.end {
                    panic!("curve called outside [cliff, end)");
                }
                linear_vesting_curve(e, &schedule, total_allocation, timestamp)
            }
        }
    }
}

#[contracttype]
enum StubKey {
    Balance,
    Transfers,
    Reenter,
}

/// A minimal SEP-41 token for what a Stellar Asset Contract cannot do: report
/// any balance (`i128::MAX`, negative), count `transfer` calls, and try to
/// re-enter the vesting contract from `transfer`.
#[contract]
struct StubToken;

#[contractimpl]
impl StubToken {
    pub fn set_balance(e: &Env, balance: i128) {
        e.storage().instance().set(&StubKey::Balance, &balance);
    }

    /// Makes `transfer` call `release(token)` on `vesting`.
    pub fn set_reenter(e: &Env, vesting: Address, token: Address) {
        e.storage().instance().set(&StubKey::Reenter, &(vesting, token));
    }

    pub fn transfer_count(e: &Env) -> u32 {
        e.storage().instance().get(&StubKey::Transfers).unwrap_or(0)
    }

    pub fn balance(e: &Env, _id: Address) -> i128 {
        e.storage().instance().get(&StubKey::Balance).unwrap_or(0)
    }

    pub fn transfer(e: &Env, _from: Address, _to: MuxedAddress, amount: i128) {
        if let Some((vesting, token)) =
            e.storage().instance().get::<_, (Address, Address)>(&StubKey::Reenter)
        {
            VestingMockClient::new(e, &vesting).release(&token);
        }
        let count = Self::transfer_count(e);
        e.storage().instance().set(&StubKey::Transfers, &(count + 1));
        let balance = Self::balance(e, e.current_contract_address());
        e.storage().instance().set(&StubKey::Balance, &(balance - amount));
    }
}

// ################## HELPERS ##################

fn vesting_error(error: VestingError) -> Error {
    Error::from_contract_error(error as u32)
}

fn set_time(e: &Env, timestamp: u64) {
    e.ledger().set_timestamp(timestamp);
}

/// Registers a SAC, returning it and its address.
fn create_sac(e: &Env) -> (StellarAssetContract, Address) {
    let sac = e.register_stellar_asset_contract_v2(Address::generate(e));
    let token = sac.address();
    (sac, token)
}

fn mint(e: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(e, token).mint(to, &amount);
}

fn register_vesting(
    e: &Env,
    receiver: &Address,
    start: u64,
    duration: u64,
    cliff_duration: u64,
) -> Address {
    e.register(VestingMock, (receiver, start, duration, cliff_duration))
}

/// Default setup: a linear vesting contract funded with 1_000 of a SAC.
/// Returns (env, receiver, vesting, token).
fn setup() -> (Env, Address, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let vesting = register_vesting(&e, &receiver, START, DURATION, CLIFF_DURATION);
    let (_, token) = create_sac(&e);
    mint(&e, &token, &vesting, 1_000);
    (e, receiver, vesting, token)
}

/// A curve-mock vesting contract funded with 1_000 of a SAC, default
/// timeline. Returns (env, vesting client, token).
fn setup_curve<'a>(mode: CurveMode) -> (Env, CurveMockClient<'a>, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let vesting = e.register(CurveMock, (&receiver, START, DURATION, CLIFF_DURATION, mode));
    let (_, token) = create_sac(&e);
    mint(&e, &token, &vesting, 1_000);
    let client = CurveMockClient::new(&e, &vesting);
    (e, client, token)
}

/// A linear vesting contract and a stub token reporting `balance`, with
/// `released` preset for the stub. Returns (env, vesting client, stub client).
fn setup_stub<'a>(
    balance: i128,
    released: i128,
) -> (Env, VestingMockClient<'a>, StubTokenClient<'a>) {
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let vesting = register_vesting(&e, &receiver, START, DURATION, CLIFF_DURATION);
    let stub = e.register(StubToken, ());
    let stub_client = StubTokenClient::new(&e, &stub);
    stub_client.set_balance(&balance);
    if released != 0 {
        e.as_contract(&vesting, || {
            e.storage().persistent().set(&VestingStorageKey::Released(stub.clone()), &released);
        });
    }
    let client = VestingMockClient::new(&e, &vesting);
    (e, client, stub_client)
}

// ################## SCHEDULE SETUP ##################

#[test]
fn set_schedule_stores_absolute_cliff_and_end_works() {
    let (e, _, vesting, _) = setup();
    let client = VestingMockClient::new(&e, &vesting);

    assert_eq!(client.start(), START);
    assert_eq!(client.duration(), DURATION);
    assert_eq!(client.cliff(), CLIFF);
    assert_eq!(client.end(), END);

    e.as_contract(&vesting, || {
        let stored: VestingSchedule =
            e.storage().instance().get(&VestingStorageKey::Schedule).unwrap();
        assert_eq!(
            stored,
            VestingSchedule { start: START, duration: DURATION, cliff: CLIFF, end: END }
        );
    });
}

#[test]
fn set_schedule_emits_event_works() {
    let e = Env::default();
    let address = e.register(NoScheduleMock, ());

    e.as_contract(&address, || {
        set_schedule(&e, START, DURATION, CLIFF_DURATION);
    });

    let events = e.events().all();
    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap(),
        &VestingScheduleSet { start: START, duration: DURATION, cliff: CLIFF, end: END }
            .to_xdr(&e, &address)
    );
}

#[test]
fn set_schedule_cliff_equal_duration_works() {
    let e = Env::default();
    let client =
        VestingMockClient::new(&e, &register_vesting(&e, &Address::generate(&e), 10, 50, 50));
    assert_eq!(client.cliff(), 60);
    assert_eq!(client.end(), 60);
}

#[test]
fn set_schedule_zero_duration_works() {
    let e = Env::default();
    let client =
        VestingMockClient::new(&e, &register_vesting(&e, &Address::generate(&e), 10, 0, 0));
    assert_eq!(client.start(), 10);
    assert_eq!(client.cliff(), 10);
    assert_eq!(client.end(), 10);
}

#[test]
fn set_schedule_end_at_u64_max_works() {
    let e = Env::default();
    let client = VestingMockClient::new(
        &e,
        &register_vesting(&e, &Address::generate(&e), u64::MAX - 5, 5, 2),
    );
    assert_eq!(client.cliff(), u64::MAX - 3);
    assert_eq!(client.end(), u64::MAX);
}

#[test]
#[should_panic(expected = "Error(Contract, #7002)")] // VestingError::InvalidCliffDuration
fn set_schedule_cliff_longer_than_duration_panics() {
    let e = Env::default();
    register_vesting(&e, &Address::generate(&e), START, DURATION, DURATION + 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #7003)")] // VestingError::EndOverflow
fn set_schedule_end_overflow_panics() {
    let e = Env::default();
    register_vesting(&e, &Address::generate(&e), u64::MAX, 1, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #7002)")] // VestingError::InvalidCliffDuration
fn set_schedule_invalid_cliff_checked_before_end_overflow_panics() {
    // Both inputs are invalid; the cliff check comes first.
    let e = Env::default();
    register_vesting(&e, &Address::generate(&e), u64::MAX, 1, 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #7001)")] // VestingError::ScheduleAlreadySet
fn set_schedule_twice_panics() {
    let (e, _, vesting, _) = setup();
    e.as_contract(&vesting, || {
        set_schedule(&e, START, DURATION, CLIFF_DURATION);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #7001)")] // VestingError::ScheduleAlreadySet
fn set_schedule_twice_with_invalid_cliff_panics() {
    // The one-shot check comes before validation.
    let (e, _, vesting, _) = setup();
    e.as_contract(&vesting, || {
        set_schedule(&e, START, DURATION, DURATION + 1);
    });
}

#[test]
fn schedule_reads_fail_without_schedule() {
    let e = Env::default();
    let client = NoScheduleMockClient::new(&e, &e.register(NoScheduleMock, ()));
    let token = Address::generate(&e);
    let not_set = vesting_error(VestingError::ScheduleNotSet);

    assert_eq!(client.try_start(), Err(Ok(not_set)));
    assert_eq!(client.try_duration(), Err(Ok(not_set)));
    assert_eq!(client.try_cliff(), Err(Ok(not_set)));
    assert_eq!(client.try_end(), Err(Ok(not_set)));
    assert_eq!(client.try_vested_amount(&token, &START), Err(Ok(not_set)));
    assert_eq!(client.try_releasable(&token), Err(Ok(not_set)));
    assert_eq!(client.try_release(&token), Err(Ok(not_set)));
}

#[test]
fn released_works_without_schedule() {
    // `released` does not read the schedule.
    let e = Env::default();
    let client = NoScheduleMockClient::new(&e, &e.register(NoScheduleMock, ()));
    assert_eq!(client.released(&Address::generate(&e)), 0);
}

// ################## AUTH AND OWNER ##################

#[test]
fn release_requires_no_auth_works() {
    // No signature from anyone; the outgoing transfer
    // is covered by invoker auth.
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    e.set_auths(&[]);
    set_time(&e, END);

    assert_eq!(client.release(&token), 1_000);
    assert!(e.auths().is_empty());
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #7004)")] // VestingError::OwnerRenounced
fn release_after_renounce_panics() {
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    client.renounce_ownership();
    set_time(&e, 1_500);
    client.release(&token);
}

#[test]
#[should_panic(expected = "Error(Contract, #7004)")] // VestingError::OwnerRenounced
fn release_after_renounce_with_nothing_releasable_panics() {
    // The owner check comes before the zero early return.
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    client.renounce_ownership();
    set_time(&e, CLIFF - 1);
    client.release(&token);
}

#[test]
fn release_pays_current_owner_works() {
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    let token_client = TokenClient::new(&e, &token);
    let new_owner = Address::generate(&e);

    client.transfer_ownership(&new_owner, &1_000);
    set_time(&e, 1_200);
    assert_eq!(client.release(&token), 200);
    assert_eq!(token_client.balance(&receiver), 200);
    assert_eq!(token_client.balance(&new_owner), 0);

    client.accept_ownership();
    set_time(&e, 1_500);
    assert_eq!(client.release(&token), 300);
    assert_eq!(token_client.balance(&receiver), 200);
    assert_eq!(token_client.balance(&new_owner), 300);
}

// ################## VESTED AMOUNT BY PHASE ##################

#[test]
fn vested_amount_by_phase_works() {
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);

    assert_eq!(client.vested_amount(&token, &0), 0);
    assert_eq!(client.vested_amount(&token, &(CLIFF - 1)), 0);
    assert_eq!(client.vested_amount(&token, &CLIFF), 100);
    assert_eq!(client.vested_amount(&token, &1_500), 500);
    assert_eq!(client.vested_amount(&token, &(END - 1)), 999);
    assert_eq!(client.vested_amount(&token, &END), 1_000);
    assert_eq!(client.vested_amount(&token, &(END + 1)), 1_000);
    assert_eq!(client.vested_amount(&token, &u64::MAX), 1_000);
}

#[test]
fn vested_amount_before_cliff_skips_balance_works() {
    // The time check comes first, so the overflowing
    // `balance + released` sum is not computed before the cliff.
    let (_, client, stub) = setup_stub(i128::MAX, 1);

    assert_eq!(client.vested_amount(&stub.address, &(CLIFF - 1)), 0);
    assert_eq!(
        client.try_vested_amount(&stub.address, &CLIFF),
        Err(Ok(vesting_error(VestingError::AllocationOverflow)))
    );
}

#[test]
fn vested_amount_zero_duration_works() {
    // With `duration == 0` the curve is never called.
    let e = Env::default();
    e.mock_all_auths();
    let vesting =
        e.register(CurveMock, (Address::generate(&e), START, 0u64, 0u64, CurveMode::Guarded));
    let client = CurveMockClient::new(&e, &vesting);
    let (_, token) = create_sac(&e);
    mint(&e, &token, &vesting, 1_000);

    assert_eq!(client.vested_amount(&token, &(START - 1)), 0);
    assert_eq!(client.vested_amount(&token, &START), 1_000);
}

#[test]
fn vested_amount_cliff_equal_end_works() {
    // With `cliff == end` the curve is never called.
    let e = Env::default();
    e.mock_all_auths();
    let vesting = e.register(
        CurveMock,
        (Address::generate(&e), START, DURATION, DURATION, CurveMode::Guarded),
    );
    let client = CurveMockClient::new(&e, &vesting);
    let (_, token) = create_sac(&e);
    mint(&e, &token, &vesting, 1_000);

    assert_eq!(client.vested_amount(&token, &(END - 1)), 0);
    assert_eq!(client.vested_amount(&token, &END), 1_000);
}

#[test]
fn curve_called_only_inside_cliff_and_end_works() {
    // The guarded curve panics if called outside
    // `[cliff, end)`.
    let (_, client, token) = setup_curve(CurveMode::Guarded);

    for (timestamp, expected) in [
        (0, 0),
        (CLIFF - 1, 0),
        (CLIFF, 100),
        (1_750, 750),
        (END - 1, 999),
        (END, 1_000),
        (u64::MAX, 1_000),
    ] {
        assert_eq!(client.vested_amount(&token, &timestamp), expected);
    }
}

#[test]
fn linear_curve_rounds_down_works() {
    // Flooring; the remainder is paid at `end`.
    let e = Env::default();
    e.mock_all_auths();
    let vesting = register_vesting(&e, &Address::generate(&e), START, 3, 0);
    let client = VestingMockClient::new(&e, &vesting);
    let (_, token) = create_sac(&e);
    mint(&e, &token, &vesting, 1_000);

    assert_eq!(client.vested_amount(&token, &START), 0);
    assert_eq!(client.vested_amount(&token, &(START + 1)), 333);
    assert_eq!(client.vested_amount(&token, &(START + 2)), 666);
    assert_eq!(client.vested_amount(&token, &(START + 3)), 1_000);
}

#[test]
fn linear_curve_large_allocation_works() {
    // `total * elapsed` overflows i128 and goes
    // through `mul_div_floor`'s i256 path.
    let (_, client, stub) = setup_stub(i128::MAX, 0);
    assert_eq!(client.vested_amount(&stub.address, &1_500), i128::MAX / 2);
}

// ################## CURVE BOUNDS ##################

#[test]
#[should_panic(expected = "Error(Contract, #7005)")] // VestingError::CurveOutOfBounds
fn vested_amount_curve_above_total_panics() {
    let (_, client, token) = setup_curve(CurveMode::AboveTotal);
    client.vested_amount(&token, &1_500);
}

#[test]
#[should_panic(expected = "Error(Contract, #7005)")] // VestingError::CurveOutOfBounds
fn vested_amount_curve_negative_panics() {
    let (_, client, token) = setup_curve(CurveMode::Negative);
    client.vested_amount(&token, &1_500);
}

#[test]
#[should_panic(expected = "Error(Contract, #7005)")] // VestingError::CurveOutOfBounds
fn release_curve_above_total_panics() {
    let (e, client, token) = setup_curve(CurveMode::AboveTotal);
    set_time(&e, 1_500);
    client.release(&token);
}

#[test]
#[should_panic(expected = "Error(Contract, #7005)")] // VestingError::CurveOutOfBounds
fn releasable_curve_above_total_panics() {
    let (e, client, token) = setup_curve(CurveMode::AboveTotal);
    set_time(&e, 1_500);
    client.releasable(&token);
}

#[test]
#[should_panic(expected = "Error(Contract, #7005)")] // VestingError::CurveOutOfBounds
fn release_curve_negative_panics() {
    let (e, client, token) = setup_curve(CurveMode::Negative);
    set_time(&e, 1_500);
    client.release(&token);
}

#[test]
fn curve_at_bounds_works() {
    // `0` and exactly `total_allocation` pass.
    let (e, client, token) = setup_curve(CurveMode::Zero);
    set_time(&e, 1_500);
    assert_eq!(client.vested_amount(&token, &1_500), 0);
    assert_eq!(client.release(&token), 0);

    let (e, client, token) = setup_curve(CurveMode::Total);
    set_time(&e, 1_500);
    assert_eq!(client.vested_amount(&token, &1_500), 1_000);
    assert_eq!(client.release(&token), 1_000);
}

// ################## VESTED BELOW RELEASED ##################

#[test]
#[should_panic(expected = "Error(Contract, #7006)")] // VestingError::ClawbackOrDecreasingCurve
fn release_decreasing_curve_panics() {
    let (e, client, token) = setup_curve(CurveMode::DecreasingAt(1_500));
    set_time(&e, 1_400);
    assert_eq!(client.release(&token), 400);
    set_time(&e, 1_600);
    client.release(&token);
}

#[test]
#[should_panic(expected = "Error(Contract, #7006)")] // VestingError::ClawbackOrDecreasingCurve
fn releasable_decreasing_curve_panics() {
    let (e, client, token) = setup_curve(CurveMode::DecreasingAt(1_500));
    set_time(&e, 1_400);
    client.release(&token);
    set_time(&e, 1_600);
    client.releasable(&token);
}

#[test]
fn release_after_clawback_stalls_works() {
    // The design's example. 1_000 over 12 "months"
    // (100 s each), 500 released at month 6, 400 clawed back at month 7.
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let (sac, token) = create_sac(&e);
    sac.issuer().set_flag(IssuerFlags::ClawbackEnabledFlag);
    let vesting = register_vesting(&e, &receiver, START, 1_200, 0);
    let client = VestingMockClient::new(&e, &vesting);
    mint(&e, &token, &vesting, 1_000);
    let month = |m: u64| START + m * 100;
    let stalled = Err(Ok(vesting_error(VestingError::ClawbackOrDecreasingCurve)));

    set_time(&e, month(6));
    assert_eq!(client.release(&token), 500);

    set_time(&e, month(7));
    StellarAssetClient::new(&e, &token).clawback(&vesting, &400);
    // Total allocation is now 100 + 500 = 600, vested at month m is 50 * m.
    assert_eq!(client.try_release(&token), stalled);
    set_time(&e, month(9));
    assert_eq!(client.try_release(&token), stalled);

    set_time(&e, month(10));
    assert_eq!(client.release(&token), 0);
    set_time(&e, month(11));
    assert_eq!(client.release(&token), 50);

    set_time(&e, month(12));
    assert_eq!(client.release(&token), 50);
    assert_eq!(TokenClient::new(&e, &token).balance(&vesting), 0);
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 600);
}

// ################## AFTER END ##################

#[test]
fn release_after_end_pays_full_balance_works() {
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, 2_500);

    assert_eq!(client.releasable(&token), 1_000);
    assert_eq!(client.release(&token), 1_000);
    assert_eq!(TokenClient::new(&e, &token).balance(&vesting), 0);
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 1_000);
}

#[test]
fn release_after_end_new_deposit_works() {
    // Funding after `end` is fully vested on arrival.
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, 2_500);
    client.release(&token);

    let funder = Address::generate(&e);
    mint(&e, &token, &funder, 300);
    TokenClient::new(&e, &token).transfer(&funder, &vesting, &300);
    assert_eq!(client.release(&token), 300);
    assert_eq!(client.released(&token), 1_300);
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 1_300);
}

#[test]
fn top_up_after_partial_release_before_end_works() {
    // A plain transfer while the schedule runs vests on the same schedule:
    // nothing from the original grant is unlocked early.
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, 1_500);
    assert_eq!(client.release(&token), 500);

    let funder = Address::generate(&e);
    mint(&e, &token, &funder, 300);
    TokenClient::new(&e, &token).transfer(&funder, &vesting, &300);
    // Allocation is now 1_300, half of it (650) vested; 500 already released.
    assert_eq!(client.releasable(&token), 150);
    assert_eq!(client.release(&token), 150);

    set_time(&e, END);
    assert_eq!(client.release(&token), 650);
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 1_300);
}

#[test]
fn release_after_end_following_clawback_works() {
    // A clawback stalls at most until `end`.
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let (sac, token) = create_sac(&e);
    sac.issuer().set_flag(IssuerFlags::ClawbackEnabledFlag);
    let vesting = register_vesting(&e, &receiver, START, DURATION, CLIFF_DURATION);
    let client = VestingMockClient::new(&e, &vesting);
    mint(&e, &token, &vesting, 1_000);

    set_time(&e, 1_500);
    assert_eq!(client.release(&token), 500);
    StellarAssetClient::new(&e, &token).clawback(&vesting, &400);
    assert_eq!(
        client.try_release(&token),
        Err(Ok(vesting_error(VestingError::ClawbackOrDecreasingCurve)))
    );

    set_time(&e, END);
    assert_eq!(client.release(&token), 100);
    assert_eq!(TokenClient::new(&e, &token).balance(&vesting), 0);
}

// ################## OVERFLOW ##################

#[test]
#[should_panic(expected = "Error(Contract, #7007)")] // VestingError::AllocationOverflow
fn vested_amount_allocation_overflow_panics() {
    let (_, client, stub) = setup_stub(i128::MAX, 1);
    client.vested_amount(&stub.address, &1_500);
}

#[test]
#[should_panic(expected = "Error(Contract, #7007)")] // VestingError::AllocationOverflow
fn release_before_end_allocation_overflow_panics() {
    let (e, client, stub) = setup_stub(i128::MAX, 1);
    set_time(&e, 1_500);
    client.release(&stub.address);
}

#[test]
fn release_after_end_overflow_saturates_works() {
    // After `end` the overflow stalls nothing, `released`
    // saturates, and later deposits are still paid in full.
    let (e, client, stub) = setup_stub(i128::MAX, 1);
    set_time(&e, END);

    assert_eq!(client.release(&stub.address), i128::MAX);
    assert_eq!(client.released(&stub.address), i128::MAX);

    stub.set_balance(&50);
    assert_eq!(client.release(&stub.address), 50);
    assert_eq!(client.released(&stub.address), i128::MAX);

    // With a balance of 0 the sum still fits; any deposit makes it overflow.
    assert_eq!(client.vested_amount(&stub.address, &END), i128::MAX);
    stub.set_balance(&10);
    assert_eq!(
        client.try_vested_amount(&stub.address, &END),
        Err(Ok(vesting_error(VestingError::AllocationOverflow)))
    );
    assert_eq!(stub.transfer_count(), 2);
}

// ################## NEGATIVE BALANCE ##################

#[test]
fn negative_balance_treated_as_zero_works() {
    let (e, client, stub) = setup_stub(-1, 0);
    set_time(&e, 2_500);

    assert_eq!(client.releasable(&stub.address), 0);
    assert_eq!(client.release(&stub.address), 0);
    assert_eq!(e.events().all().events().len(), 0);
    assert_eq!(stub.transfer_count(), 0);
    assert_eq!(client.released(&stub.address), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #7006)")] // VestingError::ClawbackOrDecreasingCurve
fn negative_balance_before_end_panics() {
    // Before `end` a negative balance behaves like a clawback.
    let (e, client, stub) = setup_stub(-1, 100);
    set_time(&e, 1_500);
    client.release(&stub.address);
}

// ################## ZERO RELEASE ##################

#[test]
fn release_before_cliff_returns_zero_works() {
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, CLIFF - 1);

    assert_eq!(client.release(&token), 0);
    assert_eq!(e.events().all().events().len(), 0);
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 0);
    e.as_contract(&vesting, || {
        assert!(!e.storage().persistent().has(&VestingStorageKey::Released(token.clone())));
    });
}

#[test]
fn release_twice_same_ledger_returns_zero_works() {
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, 1_500);

    assert_eq!(client.release(&token), 500);
    assert_eq!(client.release(&token), 0);
    assert_eq!(e.events().all().events().len(), 0);
    assert_eq!(client.released(&token), 500);
}

// ################## ACCOUNTING ##################

#[test]
fn release_across_schedule_works() {
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    let token_client = TokenClient::new(&e, &token);

    for timestamp in [START, CLIFF - 1, CLIFF, 1_250, 1_333, 1_500, 1_777, END - 1, END, 2_500] {
        set_time(&e, timestamp);
        let released_before = client.released(&token);
        let releasable = client.releasable(&token);

        assert_eq!(client.release(&token), releasable);
        assert_eq!(client.released(&token), released_before + releasable);
        assert_eq!(client.released(&token), client.vested_amount(&token, &timestamp));
        assert_eq!(token_client.balance(&receiver), client.released(&token));
    }
    assert_eq!(client.released(&token), 1_000);
    assert_eq!(token_client.balance(&vesting), 0);
}

#[test]
fn release_tracks_tokens_separately_works() {
    let (e, _, vesting, token_a) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    let (_, token_b) = create_sac(&e);
    mint(&e, &token_b, &vesting, 600);
    set_time(&e, 1_500);

    assert_eq!(client.release(&token_a), 500);
    assert_eq!(client.released(&token_a), 500);
    assert_eq!(client.released(&token_b), 0);
    assert_eq!(client.releasable(&token_b), 300);
}

#[test]
fn release_emits_event_works() {
    // `Released` is emitted before the token's own
    // transfer event.
    let (e, receiver, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, 1_500);

    client.release(&token);

    let events = e.events().all();
    assert_eq!(events.events().len(), 2);
    assert_eq!(
        events.events().first().unwrap(),
        &Released { token: token.clone(), to: receiver.clone(), amount: 500 }.to_xdr(&e, &vesting)
    );
}

// ################## STORAGE AND TTL ##################

#[test]
fn released_read_extends_ttl_works() {
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    set_time(&e, 1_500);
    client.release(&token);

    e.as_contract(&vesting, || {
        let key = VestingStorageKey::Released(token.clone());
        let ttl = e.storage().persistent().get_ttl(&key);
        e.ledger().with_mut(|l| l.sequence_number += ttl);
        assert_eq!(get_released(&e, &token), 500);
        assert_eq!(e.storage().persistent().get_ttl(&key), RELEASED_EXTEND_AMOUNT);
    });
}

#[test]
fn released_unknown_token_returns_zero_works() {
    // A missing entry reads as 0 and is not created.
    let (e, _, vesting, _) = setup();
    let unknown = Address::generate(&e);

    e.as_contract(&vesting, || {
        assert_eq!(get_released(&e, &unknown), 0);
        assert!(!e.storage().persistent().has(&VestingStorageKey::Released(unknown.clone())));
    });
}

// ################## TOKEN SIDE ##################

#[test]
fn release_fails_when_receiver_deauthorized() {
    // A token-side failure reverts everything, and
    // the release succeeds in full once the cause is fixed.
    let e = Env::default();
    e.mock_all_auths();
    let receiver = Address::generate(&e);
    let (sac, token) = create_sac(&e);
    sac.issuer().set_flag(IssuerFlags::RevocableFlag);
    let vesting = register_vesting(&e, &receiver, START, DURATION, CLIFF_DURATION);
    let client = VestingMockClient::new(&e, &vesting);
    let admin = StellarAssetClient::new(&e, &token);
    mint(&e, &token, &vesting, 1_000);
    set_time(&e, 1_500);

    admin.set_authorized(&receiver, &false);
    assert!(client.try_release(&token).is_err());
    let released_event =
        Released { token: token.clone(), to: receiver.clone(), amount: 500 }.to_xdr(&e, &vesting);
    assert!(!e.events().all().events().iter().any(|event| event == &released_event));
    assert_eq!(client.released(&token), 0);
    assert_eq!(TokenClient::new(&e, &token).balance(&vesting), 1_000);

    admin.set_authorized(&receiver, &true);
    assert_eq!(client.release(&token), 500);
    assert_eq!(TokenClient::new(&e, &token).balance(&receiver), 500);
}

#[test]
fn transfer_from_vesting_contract_without_auth_fails() {
    // Nobody can sign for the vesting contract's balance, and
    // no allowance exists.
    let (e, _, vesting, token) = setup();
    let token_client = TokenClient::new(&e, &token);
    let attacker = Address::generate(&e);
    e.set_auths(&[]);

    assert!(token_client.try_transfer(&vesting, &attacker, &100).is_err());
    assert_eq!(token_client.allowance(&vesting, &attacker), 0);
    assert_eq!(token_client.balance(&vesting), 1_000);
}

#[test]
fn reentrant_token_cannot_release_works() {
    // A token's `transfer` cannot call back into the vesting
    // contract.
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    let stub = StubTokenClient::new(&e, &e.register(StubToken, ()));
    stub.set_balance(&10);
    stub.set_reenter(&vesting, &token);
    set_time(&e, 2_500);

    assert!(client.try_release(&stub.address).is_err());
    assert_eq!(client.released(&token), 0);
    assert_eq!(client.released(&stub.address), 0);
    assert_eq!(TokenClient::new(&e, &token).balance(&vesting), 1_000);
}

#[test]
fn junk_token_does_not_affect_real_token_works() {
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    let stub = StubTokenClient::new(&e, &e.register(StubToken, ()));
    stub.set_balance(&i128::MAX);
    set_time(&e, 2_500);

    assert_eq!(client.release(&stub.address), i128::MAX);
    assert_eq!(client.released(&token), 0);
    assert_eq!(client.releasable(&token), 1_000);
}

#[test]
fn vesting_curve_direct_call_is_raw_works() {
    // The exported curve is a pure computation with an
    // unchecked result.
    let (e, _, vesting, token) = setup();
    let client = VestingMockClient::new(&e, &vesting);
    let schedule = VestingSchedule { start: START, duration: DURATION, cliff: CLIFF, end: END };

    // Past `end`, the raw linear value exceeds the allocation.
    assert_eq!(client.vesting_curve(&schedule, &1_000, &5_000), 4_000);
    // Before `start`, saturating keeps it at 0.
    assert_eq!(client.vesting_curve(&schedule, &1_000, &0), 0);
    assert_eq!(e.events().all().events().len(), 0);
    assert_eq!(client.released(&token), 0);
}
