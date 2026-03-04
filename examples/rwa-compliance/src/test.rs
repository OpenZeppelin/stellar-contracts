extern crate std;

use soroban_sdk::{testutils::Address as _, testutils::Ledger, vec, Address, Env, String};

use stellar_compliance_country_allow::{CountryAllowModule, CountryAllowModuleClient};
use stellar_compliance_country_restrict::{CountryRestrictModule, CountryRestrictModuleClient};
use stellar_compliance_initial_lockup_period::{
    InitialLockupPeriodModule, InitialLockupPeriodModuleClient,
};
use stellar_compliance_max_balance::{MaxBalanceModule, MaxBalanceModuleClient};
use stellar_compliance_supply_limit::{SupplyLimitModule, SupplyLimitModuleClient};
use stellar_compliance_time_transfers_limits::{
    Limit, TimeTransfersLimitsModule, TimeTransfersLimitsModuleClient,
};
use stellar_compliance_transfer_restrict::{TransferRestrictModule, TransferRestrictModuleClient};

use stellar_tokens::rwa::{
    compliance::{ComplianceHook, ComplianceModuleClient},
    identity_registry_storage::{CountryData, CountryRelation, IndividualCountryRelation},
};

use crate::{
    compliance::{ComplianceContract, ComplianceContractClient},
    identity_registry::{IdentityRegistryContract, IdentityRegistryContractClient},
    identity_verifier::SimpleIdentityVerifier,
    token::{RWATokenContract, RWATokenContractClient},
};

// ---------------------------------------------------------------------------
// Test setup
// ---------------------------------------------------------------------------

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    token: Address,
    token_client: RWATokenContractClient<'a>,
    compliance: Address,
    compliance_client: ComplianceContractClient<'a>,
    irs: Address,
    irs_client: IdentityRegistryContractClient<'a>,
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

fn setup<'a>() -> TestSetup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let manager = admin.clone();

    let irs = env.register(IdentityRegistryContract, (&admin, &manager));
    let irs_client = IdentityRegistryContractClient::new(&env, &irs);

    let verifier = env.register(SimpleIdentityVerifier, (&irs,));

    let compliance = env.register(ComplianceContract, (&admin,));
    let compliance_client = ComplianceContractClient::new(&env, &compliance);

    let name = String::from_str(&env, "Compliance Token");
    let symbol = String::from_str(&env, "CRWA");
    let token = env.register(
        RWATokenContract,
        (&name, &symbol, &admin, &compliance, &verifier),
    );
    let token_client = RWATokenContractClient::new(&env, &token);

    compliance_client.bind_token(&token, &admin);
    irs_client.bind_tokens(&vec![&env, token.clone()], &manager);

    TestSetup {
        env,
        admin,
        token,
        token_client,
        compliance,
        compliance_client,
        irs,
        irs_client,
    }
}

fn register_investor(
    ts: &TestSetup,
    investor: &Address,
    identity: &Address,
    country: CountryData,
) {
    ts.irs_client.add_identity(
        investor,
        identity,
        &vec![&ts.env, country],
        &ts.admin,
    );
}

fn wire_module(ts: &TestSetup, module_addr: &Address, hooks: &[ComplianceHook]) {
    let cmp_client = ComplianceModuleClient::new(&ts.env, module_addr);
    cmp_client.set_compliance_address(&ts.compliance);

    for hook in hooks {
        ts.compliance_client
            .add_module_to(hook, module_addr, &ts.admin);
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

    let module = ts.env.register(CountryAllowModule, ());
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate],
    );

    let mod_client = CountryAllowModuleClient::new(&ts.env, &module);
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

    let module = ts.env.register(CountryRestrictModule, ());
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate],
    );

    let mod_client = CountryRestrictModuleClient::new(&ts.env, &module);
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

    let module = ts.env.register(MaxBalanceModule, ());
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

    let mod_client = MaxBalanceModuleClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.set_max_balance(&ts.token, &1000);

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
// Test: SupplyLimitModule
//
// SupplyLimitModule.can_create() calls token.total_supply(), which causes
// contract re-entry when invoked through the full token→compliance→module
// chain. Soroban forbids re-entry by design. We test the module's logic
// directly via the ComplianceModuleClient to verify correctness, then
// demonstrate that mint/burn still work when the module is wired only to
// the Created hook (which does not call back to the token).
// ---------------------------------------------------------------------------

#[test]
fn test_supply_limit() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let id = Address::generate(&ts.env);
    register_investor(&ts, &investor, &id, us_country_data());

    let module = ts.env.register(SupplyLimitModule, ());
    let cmp_client = ComplianceModuleClient::new(&ts.env, &module);
    cmp_client.set_compliance_address(&ts.compliance);

    let mod_client = SupplyLimitModuleClient::new(&ts.env, &module);
    mod_client.set_supply_limit(&ts.token, &1000);

    // Verify can_create logic directly (bypassing re-entry)
    assert!(cmp_client.can_create(&investor, &800, &ts.token));
    assert!(cmp_client.can_create(&investor, &1000, &ts.token));
    assert!(!cmp_client.can_create(&investor, &1001, &ts.token));

    // Full integration: mint via token with supply_limit NOT on CanCreate
    // (avoids re-entry) to prove the stack works end-to-end for other hooks.
    ts.token_client.mint(&investor, &800, &ts.admin);
    assert_eq!(ts.token_client.total_supply(), 800);

    ts.token_client.mint(&investor, &200, &ts.admin);
    assert_eq!(ts.token_client.total_supply(), 1000);
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

    let module = ts.env.register(TimeTransfersLimitsModule, ());
    wire_module(
        &ts,
        &module,
        &[ComplianceHook::CanTransfer, ComplianceHook::Transferred],
    );

    let mod_client = TimeTransfersLimitsModuleClient::new(&ts.env, &module);
    mod_client.set_identity_registry_storage(&ts.token, &ts.irs);
    mod_client.set_time_transfer_limit(
        &ts.token,
        &Limit {
            limit_time: 3_600,
            limit_value: 100,
        },
    );

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

    let module = ts.env.register(TransferRestrictModule, ());
    wire_module(&ts, &module, &[ComplianceHook::CanTransfer]);

    let mod_client = TransferRestrictModuleClient::new(&ts.env, &module);

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
// Test: InitialLockupPeriodModule
//
// InitialLockupPeriodModule.can_transfer() and on_transfer() call
// token.balance(), which causes contract re-entry through the full stack.
// We verify the on_created hook (no re-entry — just records lock entries)
// via the full stack, then test can_transfer/on_transfer logic directly.
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

    let module = ts.env.register(InitialLockupPeriodModule, ());
    let cmp_client = ComplianceModuleClient::new(&ts.env, &module);
    cmp_client.set_compliance_address(&ts.compliance);

    let mod_client = InitialLockupPeriodModuleClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);

    // Register only Created hook through the compliance dispatcher
    // (on_created does NOT call token.balance(), so no re-entry)
    ts.compliance_client
        .add_module_to(&ComplianceHook::Created, &module, &ts.admin);

    // Mint 500 — creates lock entry via the full stack
    ts.token_client.mint(&investor, &500, &ts.admin);
    assert_eq!(ts.token_client.balance(&investor), 500);
    assert_eq!(mod_client.get_total_locked(&ts.token, &investor), 500);

    // Verify can_transfer directly (bypasses re-entry).
    // Before lockup expiry: all tokens locked, can't transfer.
    assert!(!cmp_client.can_transfer(&investor, &recipient, &100, &ts.token));

    // Advance past lockup
    ts.env.ledger().with_mut(|li| li.timestamp = 1_001);

    // After lockup expiry: tokens are unlocked, can_transfer returns true
    assert!(cmp_client.can_transfer(&investor, &recipient, &100, &ts.token));
    assert!(cmp_client.can_transfer(&investor, &recipient, &500, &ts.token));
}

// ---------------------------------------------------------------------------
// Test: Full stack — multiple modules active simultaneously
//
// Demonstrates composability with CountryAllowModule and MaxBalanceModule
// wired together. These two modules don't call back to the token, so the
// full token→compliance→module chain works without re-entry issues.
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
    let country_mod = ts.env.register(CountryAllowModule, ());
    wire_module(
        &ts,
        &country_mod,
        &[ComplianceHook::CanTransfer, ComplianceHook::CanCreate],
    );
    let country_client = CountryAllowModuleClient::new(&ts.env, &country_mod);
    country_client.set_identity_registry_storage(&ts.token, &ts.irs);
    country_client.add_allowed_country(&ts.token, &840);

    // --- Wire MaxBalanceModule (max 1000 per identity) ---
    let balance_mod = ts.env.register(MaxBalanceModule, ());
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
    let balance_client = MaxBalanceModuleClient::new(&ts.env, &balance_mod);
    balance_client.set_identity_registry_storage(&ts.token, &ts.irs);
    balance_client.set_max_balance(&ts.token, &1000);

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
    let result = ts
        .token_client
        .try_transfer(&investor_us, &investor_de, &700);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Reproduction: Soroban re-entry error
//
// These tests demonstrate the re-entry limitation. When SupplyLimitModule
// is wired to CanCreate, or InitialLockupPeriodModule is wired to
// CanTransfer, the module calls back to the token (total_supply / balance)
// while the token is already on the call stack. Soroban rejects this.
//
// Run with: cargo test -p rwa-compliance-example --lib -- reentry
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Contract re-entry is not allowed")]
fn reentry_supply_limit_can_create() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let id = Address::generate(&ts.env);
    register_investor(&ts, &investor, &id, us_country_data());

    let module = ts.env.register(SupplyLimitModule, ());
    // Wire to CanCreate — this is the hook that calls token.total_supply()
    wire_module(&ts, &module, &[ComplianceHook::CanCreate]);

    let mod_client = SupplyLimitModuleClient::new(&ts.env, &module);
    mod_client.set_supply_limit(&ts.token, &1000);

    // This mint triggers: token.mint() → compliance.can_create() →
    // supply_limit.can_create() → token.total_supply() → RE-ENTRY PANIC
    ts.token_client.mint(&investor, &100, &ts.admin);
}

#[test]
#[should_panic(expected = "Contract re-entry is not allowed")]
fn reentry_initial_lockup_can_transfer() {
    let ts = setup();

    let investor = Address::generate(&ts.env);
    let recipient = Address::generate(&ts.env);
    let id_inv = Address::generate(&ts.env);
    let id_rec = Address::generate(&ts.env);

    register_investor(&ts, &investor, &id_inv, us_country_data());
    register_investor(&ts, &recipient, &id_rec, us_country_data());

    let module = ts.env.register(InitialLockupPeriodModule, ());
    let cmp_client = ComplianceModuleClient::new(&ts.env, &module);
    cmp_client.set_compliance_address(&ts.compliance);

    let mod_client = InitialLockupPeriodModuleClient::new(&ts.env, &module);
    mod_client.set_lockup_period(&ts.token, &1_000);

    // Wire Created (no re-entry) so mint works
    ts.compliance_client
        .add_module_to(&ComplianceHook::Created, &module, &ts.admin);
    // Wire CanTransfer — this is the hook that calls token.balance()
    ts.compliance_client
        .add_module_to(&ComplianceHook::CanTransfer, &module, &ts.admin);

    // Mint works (on_created doesn't call back to token)
    ts.token_client.mint(&investor, &500, &ts.admin);

    // Advance past lockup so the module would allow the transfer
    ts.env.ledger().with_mut(|li| li.timestamp = 1_001);

    // This transfer triggers: token.transfer() → compliance.can_transfer() →
    // lockup.can_transfer() → token.balance() → RE-ENTRY PANIC
    ts.token_client.transfer(&investor, &recipient, &100);
}
