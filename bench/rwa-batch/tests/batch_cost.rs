//! Scratch benchmark for issue #767 (RWA batch functions).
//!
//! Measures naive-loop vs. hoisted batch implementations against the real
//! library stack: RWA token -> compliance (2 modules, one of which calls back
//! into the IRS) and identity verifier -> IRS + claim topics and issuers +
//! per-investor identity contract + claim issuer (with a real ed25519 check).
//!
//! Access control is omitted from the bench contracts: an RBAC check is a
//! per-call constant that a batch amortises by construction, so it cannot
//! move the naive-vs-hoisted comparison.
//!
//! Measurement harness, not library code: it prints cost curves rather than
//! asserting behaviour. Lives outside the root workspace so `cargo test
//! --workspace` and `cargo llvm-cov --workspace` skip it. Run it with
//! `cargo test --manifest-path bench/rwa-batch/Cargo.toml -- --nocapture`.
//! Results are recorded in the README next to this file.

use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short,
    testutils::{Address as _, EnvTestConfig, Ledger as _},
    vec, Address, Bytes, BytesN, Env, Map, MuxedAddress, String, Vec,
};
use stellar_contract_utils::pausable::{paused, PausableError};
use stellar_tokens::{
    fungible::{emit_transfer, Base, FungibleToken},
    rwa::{
        claim_topics_and_issuers::{storage as cti, ClaimTopicsAndIssuers},
        compliance::{
            self as compliance,
            modules::{
                max_balance::storage as max_balance, storage as module_storage,
                supply_limit::storage as supply_limit, ComplianceModule,
            },
            AccountSnapshot, Compliance, ComplianceClient, ComplianceHook, TransferKind,
        },
        emit_mint,
        identity_claims::{self as claims, Claim, IdentityClaims},
        identity_verification::{
            claim_issuer::ClaimIssuer,
            identity_registry_storage::{
                self as identity_storage, CountryData, CountryRelation, IdentityRegistryStorage,
                IdentityType, IndividualCountryRelation,
            },
        },
        identity_verifier::{storage as identity_verifier, IdentityVerifier},
        utils::token_binder::{self as binder, TokenBinder},
        IdentityVerifierClient, RWAError, RWA,
    },
};

// ################## TOKEN ##################

#[contract]
pub struct BenchToken;

#[contractimpl]
impl BenchToken {
    pub fn __constructor(e: &Env, compliance: Address, identity_verifier: Address) {
        Base::set_metadata(e, 7, String::from_str(e, "Bench"), String::from_str(e, "BNC"));
        RWA::set_compliance(e, &compliance);
        RWA::set_identity_verifier(e, &identity_verifier);
    }

    // ---------- mint ----------

    /// Naive loop: the full singular body, N times.
    pub fn batch_mint_naive(e: &Env, to_list: Vec<Address>, amounts: Vec<i128>) {
        for (to, amount) in to_list.iter().zip(amounts.iter()) {
            RWA::mint(e, &to, amount);
        }
    }

    /// Hoisted: `paused`, the identity-verifier address and the compliance
    /// address are read once for the whole batch.
    pub fn batch_mint_hoisted(e: &Env, to_list: Vec<Address>, amounts: Vec<i128>) {
        if paused(e) {
            panic_with_error!(e, PausableError::EnforcedPause);
        }
        let verifier = IdentityVerifierClient::new(e, &RWA::identity_verifier(e));
        let compliance_client = ComplianceClient::new(e, &RWA::compliance(e));
        let token = e.current_contract_address();

        for (to, amount) in to_list.iter().zip(amounts.iter()) {
            if RWA::is_frozen(e, &to) {
                panic_with_error!(e, RWAError::AddressFrozen);
            }
            verifier.verify_identity(&to);
            let to_snapshot = RWA::account_snapshot(e, &to);
            Base::update(e, None, Some(&to), amount);
            compliance_client.created(&to_snapshot, &amount, &token);
            emit_mint(e, &to, amount);
        }
    }

    // ---------- transfer ----------

    /// Naive loop: the singular body minus `require_auth`, N times. Verifies
    /// the sender's identity on every item.
    pub fn batch_transfer_naive(e: &Env, from: Address, to_list: Vec<Address>, amounts: Vec<i128>) {
        from.require_auth();
        for (to, amount) in to_list.iter().zip(amounts.iter()) {
            let from_snapshot = RWA::account_snapshot(e, &from);
            let to_snapshot = RWA::account_snapshot(e, &to);
            RWA::validate_transfer(e, &from_snapshot, &to_snapshot, amount);
            Base::update(e, Some(&from), Some(&to), amount);
            ComplianceClient::new(e, &RWA::compliance(e)).transferred(
                &from_snapshot,
                &to_snapshot,
                &amount,
                &TransferKind::Standard,
                &e.current_contract_address(),
            );
            emit_transfer(e, &from, &to, None, amount);
        }
    }

    /// Hoisted: sender identity, `paused` and `is_frozen(from)` checked once.
    pub fn batch_transfer_hoisted(
        e: &Env,
        from: Address,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
    ) {
        Self::batch_transfer_hoisted_inner(e, from, to_list, amounts, false);
    }

    /// Hoisted plus the end-of-batch sender re-verification (option b).
    pub fn batch_transfer_recheck(
        e: &Env,
        from: Address,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
    ) {
        Self::batch_transfer_hoisted_inner(e, from, to_list, amounts, true);
    }

    fn batch_transfer_hoisted_inner(
        e: &Env,
        from: Address,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
        recheck: bool,
    ) {
        from.require_auth();

        if paused(e) {
            panic_with_error!(e, PausableError::EnforcedPause);
        }
        if RWA::is_frozen(e, &from) {
            panic_with_error!(e, RWAError::AddressFrozen);
        }
        let verifier = IdentityVerifierClient::new(e, &RWA::identity_verifier(e));
        verifier.verify_identity(&from);
        let compliance_client = ComplianceClient::new(e, &RWA::compliance(e));
        let token = e.current_contract_address();

        for (to, amount) in to_list.iter().zip(amounts.iter()) {
            let from_snapshot = RWA::account_snapshot(e, &from);
            let to_snapshot = RWA::account_snapshot(e, &to);

            if RWA::is_frozen(e, &to) {
                panic_with_error!(e, RWAError::AddressFrozen);
            }
            if from_snapshot.balance - from_snapshot.frozen < amount {
                panic_with_error!(e, RWAError::InsufficientFreeTokens);
            }
            verifier.verify_identity(&to);

            Base::update(e, Some(&from), Some(&to), amount);
            compliance_client.transferred(
                &from_snapshot,
                &to_snapshot,
                &amount,
                &TransferKind::Standard,
                &token,
            );
            emit_transfer(e, &from, &to, None, amount);
        }

        if recheck {
            verifier.verify_identity(&from);
        }
    }

    /// The shipped `RWA::batch_transfer`: hoists only the sender's identity
    /// verification (the cheap `paused` and `is_frozen(from)` checks stay in the
    /// loop) and re-verifies the sender after the last item.
    pub fn batch_transfer_shipped(
        e: &Env,
        from: Address,
        to_list: Vec<Address>,
        amounts: Vec<i128>,
    ) {
        RWA::batch_transfer(e, &from, &to_list, &amounts);
    }

    // ---------- freeze family (no invariants; measured for the ceiling) ----------

    pub fn batch_freeze_naive(e: &Env, user_addresses: Vec<Address>, amounts: Vec<i128>) {
        for (user, amount) in user_addresses.iter().zip(amounts.iter()) {
            RWA::freeze_partial_tokens(e, &user, amount);
        }
    }

    // ---------- storage churn probes ----------

    /// N writes to one instance key (what `Base::update` does to
    /// `TotalSupply`).
    pub fn churn_instance(e: &Env, n: u32) {
        for i in 0..n {
            e.storage().instance().set(&symbol_short!("churn"), &(i as i128));
        }
    }

    /// N writes to one persistent key (what a batch does to a sender balance).
    pub fn churn_persistent(e: &Env, n: u32) {
        for i in 0..n {
            e.storage().persistent().set(&symbol_short!("churn"), &(i as i128));
        }
    }

    /// N writes to N distinct persistent keys, for comparison.
    pub fn churn_distinct(e: &Env, n: u32) {
        for i in 0..n {
            e.storage().persistent().set(&i, &(i as i128));
        }
    }

    /// N reads of one persistent key.
    pub fn churn_read(e: &Env, n: u32) {
        for _ in 0..n {
            let _: Option<i128> = e.storage().persistent().get(&symbol_short!("churn"));
        }
    }

    /// N reads of one instance key (the shape of a hoistable address read).
    pub fn churn_read_instance(e: &Env, n: u32) {
        for _ in 0..n {
            let _ = RWA::compliance(e);
        }
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for BenchToken {
    type ContractType = RWA;
}

// ################## COMPLIANCE ##################

#[contract]
pub struct BenchCompliance;

#[contractimpl]
impl BenchCompliance {
    pub fn bind(e: &Env, token: Address) {
        binder::bind_token(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl TokenBinder for BenchCompliance {
    fn bind_token(e: &Env, token: Address, _operator: Address) {
        binder::bind_token(e, &token);
    }

    fn unbind_token(e: &Env, token: Address, _operator: Address) {
        binder::unbind_token(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl Compliance for BenchCompliance {
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, _operator: Address) {
        compliance::storage::add_module_to(e, hook, module);
    }

    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, _operator: Address) {
        compliance::storage::remove_module_from(e, hook, module);
    }
}

// ################## MODULES ##################

#[contract]
pub struct BenchSupplyLimit;

#[contractimpl]
impl BenchSupplyLimit {
    pub fn setup(e: &Env, token: Address, compliance: Address, limit: i128) {
        module_storage::set_compliance_address(e, &token, &compliance);
        supply_limit::set_supply_limit(e, &token, limit);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for BenchSupplyLimit {
    fn on_transfer(
        _e: &Env,
        _from: AccountSnapshot,
        _to: AccountSnapshot,
        _amount: i128,
        _kind: TransferKind,
        _token: Address,
    ) {
    }

    fn on_created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
        module_storage::get_compliance_address(e, &token).require_auth();
        supply_limit::on_created(e, &to.address, amount, &token);
    }

    fn on_destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
        module_storage::get_compliance_address(e, &token).require_auth();
        supply_limit::on_destroyed(e, &from.address, amount, &token);
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "SupplyLimit")
    }

    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        module_storage::set_compliance_address(e, &token, &compliance);
    }
}

#[contract]
pub struct BenchMaxBalance;

#[contractimpl]
impl BenchMaxBalance {
    pub fn setup(e: &Env, token: Address, compliance: Address, irs: Address, max: i128) {
        module_storage::set_compliance_address(e, &token, &compliance);
        module_storage::set_irs_address(e, &token, &irs);
        max_balance::set_max_balance(e, &token, max);
    }
}

#[contractimpl(contracttrait)]
impl ComplianceModule for BenchMaxBalance {
    fn on_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        kind: TransferKind,
        token: Address,
    ) {
        module_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_transfer(e, &from.address, &to.address, amount, &kind, &token);
    }

    fn on_created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
        module_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_created(e, &to.address, amount, &token);
    }

    fn on_destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
        module_storage::get_compliance_address(e, &token).require_auth();
        max_balance::on_destroyed(e, &from.address, amount, &token);
    }

    fn name(e: &Env) -> String {
        String::from_str(e, "MaxBalance")
    }

    fn set_compliance_address(e: &Env, token: Address, compliance: Address, _operator: Address) {
        module_storage::set_compliance_address(e, &token, &compliance);
    }
}

// ################## IDENTITY VERIFIER ##################

#[contract]
pub struct BenchVerifier;

#[contractimpl]
impl BenchVerifier {
    pub fn __constructor(e: &Env, irs: Address, cti: Address) {
        identity_verifier::set_identity_registry_storage(e, &irs);
        identity_verifier::set_claim_topics_and_issuers(e, &cti);
    }
}

#[contractimpl(contracttrait)]
impl IdentityVerifier for BenchVerifier {
    fn verify_identity(e: &Env, account: &Address) {
        identity_verifier::verify_identity(e, account);
    }

    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
        identity_verifier::recovery_target(e, old_account)
    }

    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, _op: Address) {
        identity_verifier::set_claim_topics_and_issuers(e, &claim_topics_and_issuers);
    }
}

// ################## IDENTITY REGISTRY STORAGE ##################

#[contract]
pub struct BenchIrs;

#[contractimpl]
impl BenchIrs {
    pub fn register(e: &Env, account: Address, identity: Address, country: u32) {
        let data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(country)),
            metadata: None,
        };
        identity_storage::add_identity(
            e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![e, data],
        );
    }

    /// Naive loop over the singular helper: nothing here is loop-invariant.
    pub fn batch_register(e: &Env, accounts: Vec<Address>, identities: Vec<Address>) {
        for (account, identity) in accounts.iter().zip(identities.iter()) {
            let data = CountryData {
                country: CountryRelation::Individual(IndividualCountryRelation::Residence(250)),
                metadata: None,
            };
            identity_storage::add_identity(
                e,
                &account,
                &identity,
                IdentityType::Individual,
                &vec![e, data],
            );
        }
    }
}

#[contractimpl(contracttrait)]
impl TokenBinder for BenchIrs {
    fn bind_token(e: &Env, token: Address, _operator: Address) {
        binder::bind_token(e, &token);
    }

    fn unbind_token(e: &Env, token: Address, _operator: Address) {
        binder::unbind_token(e, &token);
    }
}

#[contractimpl(contracttrait)]
impl IdentityRegistryStorage for BenchIrs {
    fn add_identity(
        e: &Env,
        account: Address,
        identity: Address,
        _country_data_list: Vec<soroban_sdk::Val>,
        _operator: Address,
    ) {
        let data = CountryData {
            country: CountryRelation::Individual(IndividualCountryRelation::Residence(250)),
            metadata: None,
        };
        identity_storage::add_identity(
            e,
            &account,
            &identity,
            IdentityType::Individual,
            &vec![e, data],
        );
    }

    fn batch_add_identity(
        _e: &Env,
        _accounts: Vec<Address>,
        _identities: Vec<Address>,
        _country_data_lists: Vec<Vec<soroban_sdk::Val>>,
        _operator: Address,
    ) {
        unreachable!("the bench registers identities through the inherent helper");
    }

    fn remove_identity(e: &Env, account: Address, _operator: Address) {
        identity_storage::remove_identity(e, &account);
    }

    fn recover_identity(e: &Env, old_account: Address, new_account: Address, _operator: Address) {
        identity_storage::recover_identity(e, &old_account, &new_account);
    }
}

// ################## CLAIM TOPICS AND ISSUERS ##################

#[contract]
pub struct BenchCti;

#[contractimpl(contracttrait)]
impl ClaimTopicsAndIssuers for BenchCti {
    fn add_claim_topic(e: &Env, claim_topic: u32, _operator: Address) {
        cti::add_claim_topic(e, claim_topic);
    }

    fn remove_claim_topic(e: &Env, claim_topic: u32, _operator: Address) {
        cti::remove_claim_topic(e, claim_topic);
    }

    fn add_trusted_issuer(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        _operator: Address,
    ) {
        cti::add_trusted_issuer(e, &trusted_issuer, &claim_topics);
    }

    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, _operator: Address) {
        cti::remove_trusted_issuer(e, &trusted_issuer);
    }

    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        _operator: Address,
    ) {
        cti::update_issuer_claim_topics(e, &trusted_issuer, &claim_topics);
    }
}

// ################## INVESTOR IDENTITY ##################

#[contract]
pub struct BenchIdentity;

#[contractimpl(contracttrait)]
impl IdentityClaims for BenchIdentity {
    fn add_claim(
        e: &Env,
        topic: u32,
        scheme: u32,
        issuer: Address,
        signature: Bytes,
        data: Bytes,
        uri: String,
    ) -> BytesN<32> {
        claims::add_claim(e, topic, scheme, &issuer, &signature, &data, &uri)
    }
}

// ################## CLAIM ISSUER ##################

#[contract]
pub struct BenchIssuer;

#[contractimpl]
impl BenchIssuer {
    pub fn setup(e: &Env, public_key: BytesN<32>, message: Bytes, signature: BytesN<64>) {
        e.storage().instance().set(&symbol_short!("pk"), &public_key);
        e.storage().instance().set(&symbol_short!("msg"), &message);
        e.storage().instance().set(&symbol_short!("sig"), &signature);
    }
}

#[contractimpl]
impl ClaimIssuer for BenchIssuer {
    /// A representative live validity check: one storage read plus one
    /// ed25519 verification, which is the shape of the example issuer.
    fn is_claim_valid(
        e: &Env,
        _identity: Address,
        _claim_topic: u32,
        _scheme: u32,
        _sig_data: Bytes,
        _claim_data: Bytes,
    ) {
        let pk: BytesN<32> = e.storage().instance().get(&symbol_short!("pk")).unwrap();
        let msg: Bytes = e.storage().instance().get(&symbol_short!("msg")).unwrap();
        let sig: BytesN<64> = e.storage().instance().get(&symbol_short!("sig")).unwrap();
        e.crypto().ed25519_verify(&pk, &msg, &sig);
    }
}

// ################## LIVE MAINNET LIMITS ##################

/// Live mainnet config settings, fetched with
/// `stellar network settings --rpc-url https://mainnet.sorobanrpc.com`.
/// The SDK's `InvocationResourceLimits::mainnet()` is a stale snapshot
/// (footprint 100, writes 50, instructions 600M), so it is not used here.
const TX_MAX_FOOTPRINT_ENTRIES: u32 = 400;
const TX_MAX_WRITE_LEDGER_ENTRIES: u32 = 200;
const TX_MAX_DISK_READ_ENTRIES: u32 = 200;
const TX_MAX_INSTRUCTIONS: i64 = 400_000_000;
const TX_MEMORY_LIMIT: i64 = 41_943_040;
const TX_MAX_WRITE_BYTES: u32 = 132_096;
const TX_MAX_EVENTS_SIZE_BYTES: u32 = 16_384;

// Resource figures from the last measured invocation, so the reporting line can
// print them alongside the verdict.
std::thread_local! {
    static LAST_KEYS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static LAST_HOST: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static LAST_WR: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static LAST_EV: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static LAST_IN: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

/// Returns the live-mainnet limits that the last invocation exceeded.
///
/// Two readings of the footprint are reported, because the host's in-test check
/// sums read entries and write entries (double counting every read-write entry)
/// while a transaction footprint lists each key exactly once.
fn live_limit_breaches(e: &Env) -> std::vec::Vec<std::string::String> {
    let r = e.cost_estimate().resources();
    let distinct_keys = r.disk_read_entries + r.memory_read_entries;
    let host_style = distinct_keys + r.write_entries;
    LAST_KEYS.with(|c| c.set(distinct_keys));
    LAST_HOST.with(|c| c.set(host_style));
    LAST_WR.with(|c| c.set(r.write_entries));
    LAST_EV.with(|c| c.set(r.contract_events_size_bytes));
    LAST_IN.with(|c| c.set(r.instructions));
    let mut out = std::vec::Vec::new();
    if distinct_keys > TX_MAX_FOOTPRINT_ENTRIES {
        out.push(std::format!(
            "footprint(distinct keys) {distinct_keys} > {TX_MAX_FOOTPRINT_ENTRIES}"
        ));
    }
    if host_style > TX_MAX_FOOTPRINT_ENTRIES {
        out.push(std::format!(
            "footprint(host-style rd+wr) {host_style} > {TX_MAX_FOOTPRINT_ENTRIES}"
        ));
    }
    if r.write_entries > TX_MAX_WRITE_LEDGER_ENTRIES {
        out.push(std::format!("write entries {} > {TX_MAX_WRITE_LEDGER_ENTRIES}", r.write_entries));
    }
    if r.disk_read_entries > TX_MAX_DISK_READ_ENTRIES {
        out.push(std::format!("disk reads {} > {TX_MAX_DISK_READ_ENTRIES}", r.disk_read_entries));
    }
    if r.instructions > TX_MAX_INSTRUCTIONS {
        out.push(std::format!("instructions {} > {TX_MAX_INSTRUCTIONS}", r.instructions));
    }
    if r.mem_bytes > TX_MEMORY_LIMIT {
        out.push(std::format!("mem {} > {TX_MEMORY_LIMIT}", r.mem_bytes));
    }
    if r.write_bytes > TX_MAX_WRITE_BYTES {
        out.push(std::format!("write bytes {} > {TX_MAX_WRITE_BYTES}", r.write_bytes));
    }
    if r.contract_events_size_bytes > TX_MAX_EVENTS_SIZE_BYTES {
        out.push(std::format!(
            "events {} > {TX_MAX_EVENTS_SIZE_BYTES}",
            r.contract_events_size_bytes
        ));
    }
    out
}

// ################## HARNESS ##################

struct Stack<'a> {
    e: Env,
    token: BenchTokenClient<'a>,
    investors: std::vec::Vec<Address>,
}

/// Registers the whole stack with `topics` required claim topics and
/// `investors` registered wallets (one identity contract each).
fn setup(topics: u32, investors: u32) -> Stack<'static> {
    setup_cfg(topics, investors, 2, false)
}

/// `modules`: how many compliance modules are registered (0, 1 = supply limit
/// only, 2 = supply limit + max balance, the latter calling back into the IRS).
/// `shared_identity`: all wallets point at one identity contract instead of one
/// each.
fn setup_cfg(topics: u32, investors: u32, modules: u32, shared_identity: bool) -> Stack<'static> {
    // Snapshot capture at drop meters the recorded auth tree in shadow mode and
    // blows the shadow budget on larger batches, so turn it off.
    let e = Env::new_with_config(EnvTestConfig { capture_snapshot_at_drop: false });
    e.mock_all_auths();
    e.cost_estimate().disable_resource_limits();
    // Per-invocation resets zero the counters but keep the limits, so lifting
    // them once here is enough to measure past the mainnet ceiling.
    e.cost_estimate().budget().reset_unlimited();
    e.ledger().with_mut(|l| l.sequence_number = 1000);

    // Claim issuer with a real signature to verify.
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pk = BytesN::from_array(&e, &sk.verifying_key().to_bytes());
    let msg_bytes = [42u8; 64];
    let sig = BytesN::from_array(&e, &sk.sign(&msg_bytes).to_bytes());
    let msg = Bytes::from_array(&e, &msg_bytes);

    let issuer = e.register(BenchIssuer, ());
    BenchIssuerClient::new(&e, &issuer).setup(&pk, &msg, &sig);

    let cti = e.register(BenchCti, ());
    let cti_client = BenchCtiClient::new(&e, &cti);
    let operator = Address::generate(&e);
    let mut topic_list = Vec::new(&e);
    for t in 1..=topics {
        cti_client.add_claim_topic(&t, &operator);
        topic_list.push_back(t);
    }
    cti_client.add_trusted_issuer(&issuer, &topic_list, &operator);

    let irs = e.register(BenchIrs, ());
    let verifier = e.register(BenchVerifier, (irs.clone(), cti.clone()));
    let compliance = e.register(BenchCompliance, ());
    let token = e.register(BenchToken, (compliance.clone(), verifier.clone()));

    // Bind the token to the compliance contract and register the modules.
    let compliance_client = BenchComplianceClient::new(&e, &compliance);
    compliance_client.bind(&token);

    let supply = e.register(BenchSupplyLimit, ());
    BenchSupplyLimitClient::new(&e, &supply).setup(&token, &compliance, &i128::MAX);
    let maxbal = e.register(BenchMaxBalance, ());
    BenchMaxBalanceClient::new(&e, &maxbal).setup(&token, &compliance, &irs, &i128::MAX);

    if modules >= 1 {
        compliance_client.add_module_to(&ComplianceHook::Created, &supply, &operator);
    }
    if modules >= 2 {
        compliance_client.add_module_to(&ComplianceHook::Created, &maxbal, &operator);
        compliance_client.add_module_to(&ComplianceHook::Transferred, &maxbal, &operator);
    }

    // Register investors: one identity contract each, one claim per topic.
    let irs_client = BenchIrsClient::new(&e, &irs);
    let mut investor_list = std::vec::Vec::new();
    let mut shared: Option<Address> = None;
    for _ in 0..investors {
        let account = Address::generate(&e);
        let identity = match (shared_identity, shared.clone()) {
            (true, Some(id)) => id,
            _ => {
                let identity = e.register(BenchIdentity, ());
                let identity_client = BenchIdentityClient::new(&e, &identity);
                for t in 1..=topics {
                    identity_client.add_claim(
                        &t,
                        &101u32,
                        &issuer,
                        &Bytes::from_array(&e, &[1u8; 8]),
                        &Bytes::from_array(&e, &[2u8; 8]),
                        &String::from_str(&e, "uri"),
                    );
                }
                if shared_identity {
                    shared = Some(identity.clone());
                }
                identity
            }
        };
        irs_client.register(&account, &identity, &250u32);
        investor_list.push(account);
    }

    let token_client = BenchTokenClient::new(&e, &token);
    Stack { e, token: token_client, investors: investor_list }
}

fn report(e: &Env, label: &str, n: usize) {
    let r = e.cost_estimate().resources();
    let cpu = e.cost_estimate().budget().cpu_instruction_cost();
    println!(
        "{label:<28} n={n:<3} cpu={cpu:>11}  insns={:>11}  mem={:>9}  rd_entries={:>3} \
         wr_entries={:>3} rd_bytes={:>6} wr_bytes={:>6} events={:>5}",
        r.instructions,
        r.mem_bytes,
        r.disk_read_entries + r.memory_read_entries,
        r.write_entries,
        r.disk_read_bytes,
        r.write_bytes,
        r.contract_events_size_bytes,
    );
}

// ################## MEASUREMENTS ##################

#[test]
fn bench_mint_naive_vs_hoisted() {
    println!("\n--- batch_mint: naive vs hoisted (1 claim topic) ---");
    for n in [1usize, 2, 5, 10, 20] {
        let s = setup(1, n as u32);
        let tos = Vec::from_iter(&s.e, s.investors.iter().cloned());
        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 100i128));
        s.token.batch_mint_naive(&tos, &amounts);
        report(&s.e, "mint naive", n);

        let s = setup(1, n as u32);
        let tos = Vec::from_iter(&s.e, s.investors.iter().cloned());
        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 100i128));
        s.token.batch_mint_hoisted(&tos, &amounts);
        report(&s.e, "mint hoisted", n);
    }
}

#[test]
fn bench_transfer_naive_vs_hoisted() {
    for topics in [1u32, 2] {
        println!("\n--- batch_transfer: naive vs hoisted ({topics} claim topic(s)) ---");
        for n in [1usize, 2, 5, 10, 20] {
            for variant in 0..4 {
                let s = setup(topics, n as u32 + 1);
                let from = s.investors[0].clone();
                s.token.batch_mint_naive(&vec![&s.e, from.clone()], &vec![&s.e, 1_000_000i128]);
                let tos = Vec::from_iter(&s.e, s.investors[1..].iter().cloned());
                let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 10i128));
                match variant {
                    0 => {
                        s.token.batch_transfer_naive(&from, &tos, &amounts);
                        report(&s.e, "transfer naive", n);
                    }
                    1 => {
                        s.token.batch_transfer_hoisted(&from, &tos, &amounts);
                        report(&s.e, "transfer hoisted", n);
                    }
                    2 => {
                        s.token.batch_transfer_recheck(&from, &tos, &amounts);
                        report(&s.e, "transfer hoisted+recheck", n);
                    }
                    _ => {
                        s.token.batch_transfer_shipped(&from, &tos, &amounts);
                        report(&s.e, "transfer SHIPPED", n);
                    }
                }
            }
        }
    }
}

#[test]
fn bench_storage_churn() {
    println!("\n--- storage churn: N writes to one key vs N keys ---");
    let s = setup(1, 1);
    for n in [1u32, 5, 10, 20, 40] {
        s.token.churn_instance(&n);
        report(&s.e, "instance same key", n as usize);
        s.token.churn_persistent(&n);
        report(&s.e, "persistent same key", n as usize);
        s.token.churn_read(&n);
        report(&s.e, "persistent read same key", n as usize);
        s.token.churn_read_instance(&n);
        report(&s.e, "instance read (address)", n as usize);
        if n <= 40 {
            s.token.churn_distinct(&n);
            report(&s.e, "persistent distinct keys", n as usize);
        }
    }
}

#[test]
fn bench_ceiling() {
    println!("\n--- raw resource curve (compare against live limits in bench_ceiling_live) ---");
    for n in [10usize, 20, 30, 40, 50] {
        let s = setup(1, n as u32);
        let tos = Vec::from_iter(&s.e, s.investors.iter().cloned());
        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 100i128));
        s.token.batch_mint_naive(&tos, &amounts);
        report(&s.e, "mint naive", n);
    }
    for n in [10usize, 20, 40, 60, 80] {
        let s = setup(1, n as u32);
        let tos = Vec::from_iter(&s.e, s.investors.iter().cloned());
        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 0i128));
        s.token.batch_freeze_naive(&tos, &amounts);
        report(&s.e, "freeze naive", n);
    }
}

/// Ceiling for the IRS batch. The identity address is only stored, never
/// called, so a registration does not drag the identity contract into the
/// footprint.
#[test]
fn bench_irs_ceiling() {
    println!("\n--- batch_add_identity: footprint per registration ---");
    for n in [1usize, 10, 20, 30, 40, 50, 60] {
        let e = Env::new_with_config(EnvTestConfig { capture_snapshot_at_drop: false });
        e.mock_all_auths();
        e.cost_estimate().disable_resource_limits();
        e.cost_estimate().budget().reset_unlimited();
        let irs = e.register(BenchIrs, ());
        let client = BenchIrsClient::new(&e, &irs);
        let mut accounts = Vec::new(&e);
        let mut identities = Vec::new(&e);
        for _ in 0..n {
            accounts.push_back(Address::generate(&e));
            identities.push_back(Address::generate(&e));
        }
        client.batch_register(&accounts, &identities);
        report(&e, "irs batch_register", n);
    }
}

/// Attributes the per-recipient footprint of `batch_mint` to its sources by
/// removing one contributor at a time. Per-item cost is the n=10 minus n=5
/// delta, divided by 5.
#[test]
fn bench_footprint_decomposition() {
    println!("\n--- batch_mint: per-recipient footprint, by stack variant ---");
    let variants: [(&str, u32, u32, bool); 5] = [
        ("2 modules, own identity", 1, 2, false),
        ("1 module,  own identity", 1, 1, false),
        ("0 modules, own identity", 1, 0, false),
        ("2 modules, shared identity", 1, 2, true),
        ("0 modules, shared identity", 1, 0, true),
    ];
    for (label, topics, modules, shared) in variants {
        let mut pts = std::vec::Vec::new();
        for n in [5usize, 10] {
            let s = setup_cfg(topics, n as u32, modules, shared);
            let tos = Vec::from_iter(&s.e, s.investors.iter().cloned());
            let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 100i128));
            s.token.batch_mint_naive(&tos, &amounts);
            let r = s.e.cost_estimate().resources();
            pts.push((r.disk_read_entries + r.memory_read_entries, r.write_entries));
        }
        let (rd5, wr5) = pts[0];
        let (rd10, wr10) = pts[1];
        println!(
            "{label:<28} reads/item={:<5.1} writes/item={:<4.1} distinct-keys/item={:<4.1}",
            (rd10 - rd5) as f64 / 5.0,
            (wr10 - wr5) as f64 / 5.0,
            (rd10 - rd5) as f64 / 5.0,
        );
    }
}

/// Largest batch that fits under the LIVE mainnet limits.
#[test]
fn bench_ceiling_live() {
    println!("\n--- live mainnet limits: footprint 400, writes 200, 400M insns, 16KB events ---");
    for (label, ns) in [
        ("mint", std::vec![36usize, 38, 39, 46, 47, 48]),
        ("transfer", std::vec![29, 30, 31, 36, 37, 38]),
        ("freeze", std::vec![98, 99, 100, 132, 133, 134]),
        ("irs", std::vec![36, 37, 38]),
    ] {
        for n in ns {
            let breaches = if label == "irs" {
                let e = Env::new_with_config(EnvTestConfig { capture_snapshot_at_drop: false });
                e.mock_all_auths();
                e.cost_estimate().disable_resource_limits();
                e.cost_estimate().budget().reset_unlimited();
                let irs = e.register(BenchIrs, ());
                let client = BenchIrsClient::new(&e, &irs);
                let mut accounts = Vec::new(&e);
                let mut identities = Vec::new(&e);
                for _ in 0..n {
                    accounts.push_back(Address::generate(&e));
                    identities.push_back(Address::generate(&e));
                }
                client.batch_register(&accounts, &identities);
                live_limit_breaches(&e)
            } else {
                let s = setup(1, n as u32 + 1);
                match label {
                    "mint" => {
                        let tos = Vec::from_iter(&s.e, s.investors[..n].iter().cloned());
                        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 100i128));
                        s.token.batch_mint_naive(&tos, &amounts);
                    }
                    "transfer" => {
                        let from = s.investors[0].clone();
                        s.token.batch_mint_naive(
                            &vec![&s.e, from.clone()],
                            &vec![&s.e, 10_000_000i128],
                        );
                        let tos = Vec::from_iter(&s.e, s.investors[1..=n].iter().cloned());
                        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 10i128));
                        s.token.batch_transfer_naive(&from, &tos, &amounts);
                    }
                    _ => {
                        let tos = Vec::from_iter(&s.e, s.investors[..n].iter().cloned());
                        let amounts = Vec::from_iter(&s.e, (0..n).map(|_| 0i128));
                        s.token.batch_freeze_naive(&tos, &amounts);
                    }
                }
                live_limit_breaches(&s.e)
            };
            println!(
                "{label:<9} n={n:<4} {:<7} keys={:<4} rd+wr={:<4} wr={:<4} events={:<6} insns={}",
                if breaches.is_empty() { "fits" } else { "EXCEEDS" },
                LAST_KEYS.with(|c| c.get()),
                LAST_HOST.with(|c| c.get()),
                LAST_WR.with(|c| c.get()),
                LAST_EV.with(|c| c.get()),
                LAST_IN.with(|c| c.get()),
            );
            if !breaches.is_empty() {
                println!("          {}", breaches.join("; "));
            }
        }
    }
}
