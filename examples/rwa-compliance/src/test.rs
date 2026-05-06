extern crate std;

use rwa_country_allow::contract::{CountryAllowContract, CountryAllowContractClient};
use rwa_country_restrict::contract::{CountryRestrictContract, CountryRestrictContractClient};
use rwa_initial_lockup_period::contract::{
    InitialLockupPeriodContract, InitialLockupPeriodContractClient,
};
use rwa_max_balance::contract::{MaxBalanceContract, MaxBalanceContractClient};
use rwa_supply_limit::contract::{SupplyLimitContract, SupplyLimitContractClient};
use rwa_time_transfers_limits::contract::{
    TimeTransfersLimitsContract, TimeTransfersLimitsContractClient,
};
use rwa_transfer_restrict::contract::{TransferRestrictContract, TransferRestrictContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, IntoVal, String,
};
use stellar_tokens::rwa::{
    compliance::{
        modules::{time_transfers_limits::Limit, ComplianceModuleClient},
        ComplianceHook,
    },
    identity_registry_storage::{CountryData, CountryRelation, IndividualCountryRelation},
};

use crate::{
    compliance::{ComplianceContract, ComplianceContractClient},
    identity_registry::{IdentityRegistryContract, IdentityRegistryContractClient},
    identity_verifier::{SimpleIdentityVerifier, SimpleIdentityVerifierClient},
    token::{RWATokenContract, RWATokenContractClient},
};

// ---------------------------------------------------------------------------
// Test setup
// ---------------------------------------------------------------------------

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    manager: Address,
    token: Address,
    token_client: RWATokenContractClient<'a>,
    compliance: Address,
    compliance_client: ComplianceContractClient<'a>,
    irs: Address,
    irs_client: IdentityRegistryContractClient<'a>,
    verifier: Address,
}

fn us_country_data() -> CountryData {
    CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(840)),
        metadata: None,
    }
}

fn de_country_data() -> CountryData {
    CountryData {
        country: CountryRelation::Individual(IndividualCountryRelation::Residence(276)),
        metadata: None,
    }
}

fn setup() -> TestSetup<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let manager = Address::generate(&env);

    let irs = env.register(IdentityRegistryContract, (&admin, &manager));
    let irs_client = IdentityRegistryContractClient::new(&env, &irs);

    let verifier = env.register(SimpleIdentityVerifier, (&admin, &irs));

    let compliance = env.register(ComplianceContract, (&admin,));
    let compliance_client = ComplianceContractClient::new(&env, &compliance);

    let name = String::from_str(&env, "Compliance Token");
    let symbol = String::from_str(&env, "CRWA");
    let token = env.register(RWATokenContract, (&name, &symbol, &admin, &compliance, &verifier));
    let token_client = RWATokenContractClient::new(&env, &token);

    compliance_client.bind_token(&token, &admin);
    irs_client.bind_tokens(&vec![&env, token.clone()], &manager);

    TestSetup {
        env,
        admin,
        manager,
        token,
        token_client,
        compliance,
        compliance_client,
        irs,
        irs_client,
        verifier,
    }
}

fn register_investor(ts: &TestSetup, investor: &Address, identity: &Address, country: CountryData) {
    ts.irs_client.add_identity(
        investor,
        identity,
        &vec![&ts.env, country.into_val(&ts.env)],
        &ts.manager,
    );
}

fn wire_module(ts: &TestSetup, module_addr: &Address, hooks: &[ComplianceHook]) {
    let cmp_client = ComplianceModuleClient::new(&ts.env, module_addr);
    cmp_client.set_compliance_address(&ts.compliance);

    for hook in hooks {
        ts.compliance_client.add_module_to(hook, module_addr, &ts.admin);
    }
}

// ---------------------------------------------------------------------------
// Test: CountryAllowModule
// ---------------------------------------------------------------------------

#[test]
fn test_country_allow() {
    let ts = setup();

    let investor_us = Address::generate(&ts.env);
    let investor_de = Address::generate(&ts.env);
    let id_us = Address::generate(&ts.env);
    let id_de = Address::generate(&ts.env);

    register_investor(&ts, &investor_us, &id_us, us_country_data());
    register_investor(&ts, &investor_de, &id_de, de_country_data());

    let module = ts.env.register(CountryAllowContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate]);

    let mod_client = CountryAllowContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.add_allowed_country(&ts.token, &840);

    // Mint to US investor passes
    ts.token_client.mint(&investor_us, &500, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor_us), 500);

    // Mint to DE investor fails (276 not allowed)
    let result = ts.token_client.try_mint(&investor_de, &100, &ts.admin);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Test: CountryRestrictModule
// ---------------------------------------------------------------------------

#[test]
fn test_country_restrict() {
    let ts = setup();

    let investor_us = Address::generate(&ts.env);
    let investor_de = Address::generate(&ts.env);
    let id_us = Address::generate(&ts.env);
    let id_de = Address::generate(&ts.env);

    register_investor(&ts, &investor_us, &id_us, us_country_data());
    register_investor(&ts, &investor_de, &id_de, de_country_data());

    let module = ts.env.register(CountryRestrictContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate]);

    let mod_client = CountryRestrictContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.add_country_restriction(&ts.token, &840);

    // Mint to DE investor passes
    ts.token_client.mint(&investor_de, &500, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor_de), 500);

    // Mint to US investor fails (840 restricted)
    let result = ts.token_client.try_mint(&investor_us, &100, &ts.admin);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Test: MaxBalanceModule
// ---------------------------------------------------------------------------

#[test]
fn test_max_balance() {
    let ts = setup();

    let investor_a = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id_a = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);

    register_investor(&ts, &investor_a, &id_a, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(MaxBalanceContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ],
    );

    let mod_client = MaxBalanceContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.set_max_balance(&ts.token, &1000);
    mod_client.verify_hook_wiring();

    // Mint 800 to investor A
    ts.token_client.mint(&investor_a, &800, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor_a), 800);
    assert_eq!(mod_client.get_investor_balance(&ts.token, &id_a), 800);

    // Mint 300 to investor B
    ts.token_client.mint(&investor_b, &300, &ts.admin);
    assert_eq!(mod_client.get_investor_balance(&ts.token, &id_b), 300);

    // Transfer 250 from A to B pushes B to 550 — passes
    ts.token_client.transfer(&investor_a, &investor_b, &250);
    assert_eq!(mod_client.get_investor_balance(&ts.token, &id_b), 550);

    // Transfer 500 from A to B would push B to 1050 — exceeds max
    let result = ts.token_client.try_transfer(&investor_a, &investor_b, &500);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Test: SupplyLimitModule (full stack — internal supply tracking)
// ---------------------------------------------------------------------------

#[test]
fn test_supply_limit() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);
    register_investor(&ts, &investor, &id, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(SupplyLimitContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed],
    );

    let mod_client = SupplyLimitContractClient::new(&ts.env, &module);
    mod_client.set_supply_limit(&ts.token, &1000);
    mod_client.verify_hook_wiring();

    // Mint 800 — internal supply tracks to 800
    ts.token_client.mint(&investor, &800, &ts.admin);
    assert_eq!(ts.token_client.total_supply(), 800);
    assert_eq!(mod_client.get_internal_supply(&ts.token), 800);

    // Mint 200 more — exactly at limit (1000)
    ts.token_client.mint(&investor_b, &200, &ts.admin);
    assert_eq!(ts.token_client.total_supply(), 1000);
    assert_eq!(mod_client.get_internal_supply(&ts.token), 1000);

    // Mint 1 more — exceeds limit, blocked by can_create
    let result = ts.token_client.try_mint(&investor, &1, &ts.admin);
    assert!(result.is_err());

    // Transfer doesn't affect supply — always allowed
    ts.token_client.transfer(&investor, &investor_b, &100);
    assert_eq!(mod_client.get_internal_supply(&ts.token), 1000);
}

#[test]
fn test_supply_limit_burn() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let id = Address::generate(&ts.env);
    register_investor(&ts, &investor, &id, us_country_data());

    let module = ts.env.register(SupplyLimitContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed],
    );

    let mod_client = SupplyLimitContractClient::new(&ts.env, &module);
    mod_client.set_supply_limit(&ts.token, &1000);
    mod_client.verify_hook_wiring();

    // Mint to limit
    ts.token_client.mint(&investor, &1000, &ts.admin);
    assert_eq!(mod_client.get_internal_supply(&ts.token), 1000);

    // Burn 300 — internal supply decrements
    ts.token_client.burn(&investor, &300, &ts.admin);
    assert_eq!(ts.token_client.total_supply(), 700);
    assert_eq!(mod_client.get_internal_supply(&ts.token), 700);

    // Now minting 300 more is possible again
    ts.token_client.mint(&investor, &300, &ts.admin);
    assert_eq!(mod_client.get_internal_supply(&ts.token), 1000);

    // But 1 more still fails
    let result = ts.token_client.try_mint(&investor, &1, &ts.admin);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Test: TimeTransfersLimitsModule
// ---------------------------------------------------------------------------

#[test]
fn test_time_transfer_limits() {
    let ts = setup();

    let investor_a = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id_a = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);

    register_investor(&ts, &investor_a, &id_a, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(TimeTransfersLimitsContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::Transferred]);

    let mod_client = TimeTransfersLimitsContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.set_time_transfer_limit(&ts.token, &Limit { limit_time: 3_600, limit_value: 100 });
    mod_client.verify_hook_wiring();

    // Mint enough tokens for transfers
    ts.token_client.mint(&investor_a, &500, &ts.admin);

    // Transfer 60 — passes (counter at 60/100)
    ts.token_client.transfer(&investor_a, &investor_b, &60);
    assert_eq!(ts.token_client.balance(&investor_b), 60);

    // Transfer 50 more — would push counter to 110, exceeds 100/hour
    let result = ts.token_client.try_transfer(&investor_a, &investor_b, &50);
    assert!(result.is_err());

    // Transfer 40 — passes (counter at 100, exactly at limit)
    ts.token_client.transfer(&investor_a, &investor_b, &40);
    assert_eq!(ts.token_client.balance(&investor_b), 100);
}

// ---------------------------------------------------------------------------
// Test: TransferRestrictModule
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_restrict() {
    let ts = setup();

    let investor_a = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id_a = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);

    register_investor(&ts, &investor_a, &id_a, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(TransferRestrictContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer]);

    let mod_client = TransferRestrictContractClient::new(&ts.env, &module);

    // Mint tokens (no CanCreate hook, so mints pass)
    ts.token_client.mint(&investor_a, &500, &ts.admin);

    // Transfer without allowlist — fails
    let result = ts.token_client.try_transfer(&investor_a, &investor_b, &100);
    assert!(result.is_err());

    // Allow investor_a as sender
    mod_client.allow_user(&ts.token, &investor_a);

    // Now transfer passes
    ts.token_client.transfer(&investor_a, &investor_b, &100);
    assert_eq!(ts.token_client.balance(&investor_b), 100);
}

// ---------------------------------------------------------------------------
// Test: InitialLockupPeriodModule (full stack — internal balance tracking)
// ---------------------------------------------------------------------------

#[test]
fn test_initial_lockup() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let recipient = Address::generate(&ts.env);
    let id_inv = Address::generate(&ts.env);
    let id_rec = Address::generate(&ts.env);

    register_investor(&ts, &investor, &id_inv, us_country_data());
    register_investor(&ts, &recipient, &id_rec, us_country_data());

    let module = ts.env.register(InitialLockupPeriodContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ],
    );

    let mod_client = InitialLockupPeriodContractClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);
    mod_client.verify_hook_wiring();

    // Mint 500 — creates lock entry, internal balance tracks to 500
    ts.token_client.mint(&investor, &500, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor), 500);
    assert_eq!(mod_client.get_total_locked(&ts.token, &investor), 500);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 500);

    // Before lockup expiry: all tokens locked, transfer blocked
    let result = ts.token_client.try_transfer(&investor, &recipient, &100);
    assert!(result.is_err());

    // Advance past lockup
    ts.env.ledger().with_mut(|li| li.timestamp = 1_001);

    // After lockup expiry: transfer succeeds through the full stack
    ts.token_client.transfer(&investor, &recipient, &200);
    assert_eq!(ts.token_client.balance(&investor), 300);
    assert_eq!(ts.token_client.balance(&recipient), 200);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 300);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &recipient), 200);

    // Transfer rest
    ts.token_client.transfer(&investor, &recipient, &300);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 0);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &recipient), 500);
}

#[test]
fn test_initial_lockup_partial_unlock() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let recipient = Address::generate(&ts.env);
    let id_inv = Address::generate(&ts.env);
    let id_rec = Address::generate(&ts.env);

    register_investor(&ts, &investor, &id_inv, us_country_data());
    register_investor(&ts, &recipient, &id_rec, us_country_data());

    let module = ts.env.register(InitialLockupPeriodContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ],
    );

    let mod_client = InitialLockupPeriodContractClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);
    mod_client.verify_hook_wiring();

    // Mint 300 at t=0 (lock until t=1000)
    ts.token_client.mint(&investor, &300, &ts.admin);

    // Advance to t=500, mint 200 more (lock until t=1500)
    ts.env.ledger().with_mut(|li| li.timestamp = 500);
    ts.token_client.mint(&investor, &200, &ts.admin);
    assert_eq!(mod_client.get_total_locked(&ts.token, &investor), 500);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 500);

    // At t=1001: first batch unlocked, second still locked
    ts.env.ledger().with_mut(|li| li.timestamp = 1_001);

    // Can transfer up to 300 (first batch unlocked)
    ts.token_client.transfer(&investor, &recipient, &300);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 200);

    // Can't transfer remaining 200 (still locked)
    let result = ts.token_client.try_transfer(&investor, &recipient, &200);
    assert!(result.is_err());

    // At t=1501: second batch also unlocked
    ts.env.ledger().with_mut(|li| li.timestamp = 1_501);
    ts.token_client.transfer(&investor, &recipient, &200);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 0);
}

#[test]
fn test_initial_lockup_burn() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let id_inv = Address::generate(&ts.env);

    register_investor(&ts, &investor, &id_inv, us_country_data());

    let module = ts.env.register(InitialLockupPeriodContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ],
    );

    let mod_client = InitialLockupPeriodContractClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);
    mod_client.verify_hook_wiring();

    // Mint 500 (locked until t=1000)
    ts.token_client.mint(&investor, &500, &ts.admin);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 500);

    // Advance past lockup
    ts.env.ledger().with_mut(|li| li.timestamp = 1_001);

    // Burn 200 — internal balance decrements
    ts.token_client.burn(&investor, &200, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor), 300);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 300);

    // Burn remaining
    ts.token_client.burn(&investor, &300, &ts.admin);
    assert_eq!(mod_client.get_internal_balance(&ts.token, &investor), 0);
}

// ---------------------------------------------------------------------------
// Test: Full stack — multiple modules active simultaneously
// ---------------------------------------------------------------------------

#[test]
fn test_full_stack() {
    let ts = setup();

    let investor_us = Address::generate(&ts.env);
    let investor_de = Address::generate(&ts.env);
    let id_us = Address::generate(&ts.env);
    let id_de = Address::generate(&ts.env);

    register_investor(&ts, &investor_us, &id_us, us_country_data());
    register_investor(&ts, &investor_de, &id_de, de_country_data());

    // --- Wire CountryAllowModule (allow US only) ---
    let country_mod = ts.env.register(CountryAllowContract, (&ts.admin,));
    wire_module(&ts, &country_mod, &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate]);
    let country_client = CountryAllowContractClient::new(&ts.env, &country_mod);
    country_client.set_identity_registry_storage(&ts.token, &ts.irs);
    country_client.add_allowed_country(&ts.token, &840);

    // --- Wire MaxBalanceModule (max 1000 per identity) ---
    let balance_mod = ts.env.register(MaxBalanceContract, (&ts.admin,));
    wire_module(
        &ts,
        &balance_mod,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ],
    );
    let balance_client = MaxBalanceContractClient::new(&ts.env, &balance_mod);
    balance_client.set_identity_registry_storage(&ts.token, &ts.irs);
    balance_client.set_max_balance(&ts.token, &1000);
    balance_client.verify_hook_wiring();

    // 1) Mint 800 to US investor — passes all modules
    ts.token_client.mint(&investor_us, &800, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor_us), 800);

    // 2) Mint to DE investor — fails (country not allowed)
    let result = ts.token_client.try_mint(&investor_de, &100, &ts.admin);
    assert!(result.is_err());

    // 3) Allow DE, then mint 300 to DE investor — passes
    country_client.add_allowed_country(&ts.token, &276);
    ts.token_client.mint(&investor_de, &300, &ts.admin);

    // 4) Mint 200 more to US investor (total 1000) — exactly at max balance
    ts.token_client.mint(&investor_us, &200, &ts.admin);
    assert_eq!(balance_client.get_investor_balance(&ts.token, &id_us), 1000);

    // 5) Mint 1 more to US investor — exceeds max balance of 1000
    let result = ts.token_client.try_mint(&investor_us, &1, &ts.admin);
    assert!(result.is_err());

    // 6) Transfer 100 from US to DE — passes (DE at 400, under 1000)
    ts.token_client.transfer(&investor_us, &investor_de, &100);
    assert_eq!(ts.token_client.balance(&investor_us), 900);
    assert_eq!(ts.token_client.balance(&investor_de), 400);

    // 7) Transfer 700 to DE would push DE identity to 1100 — exceeds max
    let result = ts.token_client.try_transfer(&investor_us, &investor_de, &700);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: Hook wiring verification guards
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "not armed")]
fn guard_supply_limit_without_verification() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let id = Address::generate(&ts.env);
    register_investor(&ts, &investor, &id, us_country_data());

    let module = ts.env.register(SupplyLimitContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed],
    );

    let mod_client = SupplyLimitContractClient::new(&ts.env, &module);
    mod_client.set_supply_limit(&ts.token, &1000);
    // Intentionally NOT calling verify_hook_wiring()

    ts.token_client.mint(&investor, &100, &ts.admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #398)")]
fn guard_supply_limit_missing_hook() {
    let ts = setup();

    let module = ts.env.register(SupplyLimitContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanCreate], // missing Created and Destroyed
    );

    let mod_client = SupplyLimitContractClient::new(&ts.env, &module);
    mod_client.verify_hook_wiring();
}

#[test]
#[should_panic(expected = "not armed")]
fn guard_initial_lockup_without_verification() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let recipient = Address::generate(&ts.env);
    let id_inv = Address::generate(&ts.env);
    let id_rec = Address::generate(&ts.env);

    register_investor(&ts, &investor, &id_inv, us_country_data());
    register_investor(&ts, &recipient, &id_rec, us_country_data());

    let module = ts.env.register(InitialLockupPeriodContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ],
    );

    let mod_client = InitialLockupPeriodContractClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);
    // Intentionally NOT calling verify_hook_wiring()

    ts.token_client.mint(&investor, &500, &ts.admin);
    ts.env.ledger().with_mut(|li| li.timestamp = 1_001);
    ts.token_client.transfer(&investor, &recipient, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #398)")]
fn guard_max_balance_missing_hook() {
    let ts = setup();

    let module = ts.env.register(MaxBalanceContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate], /* missing Transferred,
                                                                    * Created, Destroyed */
    );

    let mod_client = MaxBalanceContractClient::new(&ts.env, &module);
    mod_client.verify_hook_wiring();
}

// ---------------------------------------------------------------------------
// Test: Burn during lockup panics (3A)
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "insufficient unlocked balance for burn")]
fn test_initial_lockup_burn_during_lockup() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let id_inv = Address::generate(&ts.env);
    register_investor(&ts, &investor, &id_inv, us_country_data());

    let module = ts.env.register(InitialLockupPeriodContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::Created,
            ComplianceHook::Transferred,
            ComplianceHook::Destroyed,
        ],
    );

    let mod_client = InitialLockupPeriodContractClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);
    mod_client.verify_hook_wiring();

    ts.token_client.mint(&investor, &500, &ts.admin);
    // All 500 tokens are locked (lockup until t=1000), burn should panic
    ts.token_client.burn(&investor, &100, &ts.admin);
}

// ---------------------------------------------------------------------------
// Test: TimeTransfersLimits window reset (3B)
// ---------------------------------------------------------------------------

#[test]
fn test_time_transfer_limits_window_reset() {
    let ts = setup();

    let investor_a = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id_a = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);

    register_investor(&ts, &investor_a, &id_a, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(TimeTransfersLimitsContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::Transferred]);

    let mod_client = TimeTransfersLimitsContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.set_time_transfer_limit(&ts.token, &Limit { limit_time: 3_600, limit_value: 100 });
    mod_client.verify_hook_wiring();

    ts.token_client.mint(&investor_a, &500, &ts.admin);

    // Transfer up to the limit
    ts.token_client.transfer(&investor_a, &investor_b, &100);
    assert_eq!(ts.token_client.balance(&investor_b), 100);

    // At the limit — next transfer should fail
    let result = ts.token_client.try_transfer(&investor_a, &investor_b, &1);
    assert!(result.is_err());

    // Advance past the 1-hour window
    ts.env.ledger().with_mut(|li| li.timestamp = 3_601);

    // Counter reset — transfers succeed again
    ts.token_client.transfer(&investor_a, &investor_b, &80);
    assert_eq!(ts.token_client.balance(&investor_b), 180);

    // Still within new window, push to limit again
    ts.token_client.transfer(&investor_a, &investor_b, &20);

    // Exceeds new window limit
    let result = ts.token_client.try_transfer(&investor_a, &investor_b, &1);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Tests: TimeTransfersLimits wiring guards (3C)
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "not armed")]
fn guard_time_transfers_without_verification() {
    let ts = setup();

    let investor_a = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id_a = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);

    register_investor(&ts, &investor_a, &id_a, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(TimeTransfersLimitsContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::Transferred]);

    let mod_client = TimeTransfersLimitsContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.set_time_transfer_limit(&ts.token, &Limit { limit_time: 3_600, limit_value: 100 });
    // Intentionally NOT calling verify_hook_wiring()

    ts.token_client.mint(&investor_a, &500, &ts.admin);
    ts.token_client.transfer(&investor_a, &investor_b, &50);
}

#[test]
#[should_panic(expected = "Error(Contract, #398)")]
fn guard_time_transfers_missing_hook() {
    let ts = setup();

    let module = ts.env.register(TimeTransfersLimitsContract, (&ts.admin,));
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanTransfer], // missing Transferred
    );

    let mod_client = TimeTransfersLimitsContractClient::new(&ts.env, &module);
    mod_client.verify_hook_wiring();
}

// ---------------------------------------------------------------------------
// Test: CountryAllow/Restrict on can_transfer (not just can_create) (3D)
// ---------------------------------------------------------------------------

#[test]
fn test_country_allow_blocks_transfer_to_non_allowed() {
    let ts = setup();

    let investor_us = Address::generate(&ts.env);
    let investor_de = Address::generate(&ts.env);
    let id_us = Address::generate(&ts.env);
    let id_de = Address::generate(&ts.env);

    register_investor(&ts, &investor_us, &id_us, us_country_data());
    register_investor(&ts, &investor_de, &id_de, de_country_data());

    let module = ts.env.register(CountryAllowContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate]);

    let mod_client = CountryAllowContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.add_allowed_country(&ts.token, &840); // US only

    // Mint to US investor passes
    ts.token_client.mint(&investor_us, &500, &ts.admin);

    // Transfer to DE investor (276 not allowed) should fail on can_transfer
    let result = ts.token_client.try_transfer(&investor_us, &investor_de, &100);
    assert!(result.is_err());

    // Allow DE, then transfer succeeds
    mod_client.add_allowed_country(&ts.token, &276);
    ts.token_client.transfer(&investor_us, &investor_de, &100);
    assert_eq!(ts.token_client.balance(&investor_de), 100);
}

#[test]
fn test_country_restrict_blocks_transfer_to_restricted() {
    let ts = setup();

    let investor_us = Address::generate(&ts.env);
    let investor_de = Address::generate(&ts.env);
    let id_us = Address::generate(&ts.env);
    let id_de = Address::generate(&ts.env);

    register_investor(&ts, &investor_us, &id_us, us_country_data());
    register_investor(&ts, &investor_de, &id_de, de_country_data());

    let module = ts.env.register(CountryRestrictContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate]);

    let mod_client = CountryRestrictContractClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.add_country_restriction(&ts.token, &276); // Restrict DE

    // Mint to US investor passes (840 not restricted)
    ts.token_client.mint(&investor_us, &500, &ts.admin);

    // Transfer to DE investor (276 restricted) should fail on can_transfer
    let result = ts.token_client.try_transfer(&investor_us, &investor_de, &100);
    assert!(result.is_err());

    // Unrestrict DE, then transfer succeeds
    mod_client.remove_country_restriction(&ts.token, &276);
    ts.token_client.transfer(&investor_us, &investor_de, &100);
    assert_eq!(ts.token_client.balance(&investor_de), 100);
}

// ---------------------------------------------------------------------------
// Test: TransferRestrict recipient-allowed path (3E)
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_restrict_recipient_allowed() {
    let ts = setup();

    let investor_a = Address::generate(&ts.env);
    let investor_b = Address::generate(&ts.env);
    let id_a = Address::generate(&ts.env);
    let id_b = Address::generate(&ts.env);

    register_investor(&ts, &investor_a, &id_a, us_country_data());
    register_investor(&ts, &investor_b, &id_b, us_country_data());

    let module = ts.env.register(TransferRestrictContract, (&ts.admin,));
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer]);

    let mod_client = TransferRestrictContractClient::new(&ts.env, &module);

    ts.token_client.mint(&investor_a, &500, &ts.admin);

    // Neither on allowlist — transfer fails
    let result = ts.token_client.try_transfer(&investor_a, &investor_b, &100);
    assert!(result.is_err());

    // Allow ONLY the recipient (investor_b), NOT the sender (investor_a)
    mod_client.allow_user(&ts.token, &investor_b);

    // Transfer passes because recipient is allowlisted (T-REX: sender OR recipient)
    ts.token_client.transfer(&investor_a, &investor_b, &100);
    assert_eq!(ts.token_client.balance(&investor_b), 100);
}

// ---------------------------------------------------------------------------
// Test: Full stack with burn step (3F)
// ---------------------------------------------------------------------------

#[test]
fn test_full_stack_with_burn() {
    let ts = setup();

    let investor_us = Address::generate(&ts.env);
    let id_us = Address::generate(&ts.env);

    register_investor(&ts, &investor_us, &id_us, us_country_data());

    // --- Wire CountryAllowModule (allow US) ---
    let country_mod = ts.env.register(CountryAllowContract, (&ts.admin,));
    wire_module(&ts, &country_mod, &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate]);
    let country_client = CountryAllowContractClient::new(&ts.env, &country_mod);
    country_client.set_identity_registry_storage(&ts.token, &ts.irs);
    country_client.add_allowed_country(&ts.token, &840);

    // --- Wire MaxBalanceModule (max 1000) ---
    let balance_mod = ts.env.register(MaxBalanceContract, (&ts.admin,));
    wire_module(
        &ts,
        &balance_mod,
        &[
            ComplianceHook::CanTransfer,
            ComplianceHook::CanCreate,
            ComplianceHook::Transferred,
            ComplianceHook::Created,
            ComplianceHook::Destroyed,
        ],
    );
    let balance_client = MaxBalanceContractClient::new(&ts.env, &balance_mod);
    balance_client.set_identity_registry_storage(&ts.token, &ts.irs);
    balance_client.set_max_balance(&ts.token, &1000);
    balance_client.verify_hook_wiring();

    // --- Wire SupplyLimitModule (limit 2000) ---
    let supply_mod = ts.env.register(SupplyLimitContract, (&ts.admin,));
    wire_module(
        &ts,
        &supply_mod,
        &[ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed],
    );
    let supply_client = SupplyLimitContractClient::new(&ts.env, &supply_mod);
    supply_client.set_supply_limit(&ts.token, &2000);
    supply_client.verify_hook_wiring();

    // 1) Mint 800
    ts.token_client.mint(&investor_us, &800, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor_us), 800);
    assert_eq!(balance_client.get_investor_balance(&ts.token, &id_us), 800);
    assert_eq!(supply_client.get_internal_supply(&ts.token), 800);

    // 2) Burn 300 — all internal state decrements
    ts.token_client.burn(&investor_us, &300, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor_us), 500);
    assert_eq!(balance_client.get_investor_balance(&ts.token, &id_us), 500);
    assert_eq!(supply_client.get_internal_supply(&ts.token), 500);

    // 3) Mint back to 1000 (identity max) — succeeds
    ts.token_client.mint(&investor_us, &500, &ts.admin);
    assert_eq!(balance_client.get_investor_balance(&ts.token, &id_us), 1000);
    assert_eq!(supply_client.get_internal_supply(&ts.token), 1000);

    // 4) Mint 1 more — exceeds max balance
    let result = ts.token_client.try_mint(&investor_us, &1, &ts.admin);
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn identity_verifier_maps_missing_identity_to_rwa_error() {
    let ts = setup();
    let verifier_client = SimpleIdentityVerifierClient::new(&ts.env, &ts.verifier);
    let unknown_account = Address::generate(&ts.env);

    verifier_client.verify_identity(&unknown_account);
}

#[test]
#[should_panic(expected = "Error(Contract, #310)")]
fn identity_verifier_claim_topics_getter_uses_contract_error() {
    let ts = setup();
    let verifier_client = SimpleIdentityVerifierClient::new(&ts.env, &ts.verifier);

    verifier_client.claim_topics_and_issuers();
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn identity_verifier_claim_topics_setter_requires_admin_role() {
    let ts = setup();
    let verifier_client = SimpleIdentityVerifierClient::new(&ts.env, &ts.verifier);
    let claim_topics_and_issuers = Address::generate(&ts.env);
    let unauthorized_operator = Address::generate(&ts.env);

    verifier_client.set_claim_topics_and_issuers(&claim_topics_and_issuers, &unauthorized_operator);
}
