use soroban_sdk::{contracttype, panic_with_error, token, Address, Env};

use crate::confidential::{
    compliance::{
        emit_compliance_config_changed, emit_frozen, emit_unfrozen, ComplianceError, PolicyClient,
        FROZEN_EXTEND_AMOUNT, FROZEN_TTL_THRESHOLD,
    },
    storage::get_underlying_asset,
};

// ################## TYPES ##################

/// Compliance configuration written once at construction and rotatable under
/// admin auth thereafter. Stored as an instance storage entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceConfig {
    /// Optional external authorization policy (see
    /// [`crate::confidential::compliance::Policy`]). `None` disables the
    /// policy gate.
    pub policy: Option<Address>,
    /// When `true`, the gates additionally consult the underlying SAC's
    /// `authorized()` view. Requires the underlying token to be a Stellar
    /// Asset Contract — `authorized` is not part of SEP-41, and enabling
    /// this flag over a non-SAC underlying makes every gated operation trap
    /// (see [`check_sac`]).
    pub sac_passthrough: bool,
}

/// Storage keys for the confidential token compliance extension.
#[contracttype]
pub enum ComplianceStorageKey {
    /// Singleton [`ComplianceConfig`]. Instance storage.
    Config,
    /// Per-account frozen flag. Persistent storage; only set when an account
    /// is frozen and removed on unfreeze.
    Frozen(Address),
}

// ################## QUERY STATE ##################

/// Returns the active [`ComplianceConfig`], or `None` when compliance has not
/// been configured for this deployment.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn compliance_config(e: &Env) -> Option<ComplianceConfig> {
    e.storage().instance().get(&ComplianceStorageKey::Config)
}

/// Returns whether `account` is currently frozen.
///
/// Returns `false` when compliance is not configured, ignoring any stale
/// `Frozen` entry left over from a prior configuration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to query.
pub fn is_frozen(e: &Env, account: &Address) -> bool {
    if compliance_config(e).is_none() {
        return false;
    }
    let key = ComplianceStorageKey::Frozen(account.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, FROZEN_TTL_THRESHOLD, FROZEN_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

// ################## CHANGE STATE ##################

/// Writes `config` into instance storage, overwriting any prior value. The
/// function does not guard against re-initialization: it is the single setter
/// used both for the initial deployment-time write and for subsequent
/// rotations.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `config` - The new [`ComplianceConfig`].
///
/// # Events
///
/// * topics - `["compliance_config_changed"]`
/// * data - `[policy: Option<Address>, sac_passthrough: bool]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn set_compliance_config(e: &Env, config: &ComplianceConfig) {
    e.storage().instance().set(&ComplianceStorageKey::Config, config);
    emit_compliance_config_changed(e, &config.policy, config.sac_passthrough);
}

/// Marks `account` as frozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to freeze.
///
/// # Errors
///
/// * [`ComplianceError::NotConfigured`] - When [`compliance_config`] returns
///   `None`.
///
/// # Events
///
/// * topics - `["frozen", account: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn freeze(e: &Env, account: &Address) {
    if compliance_config(e).is_none() {
        panic_with_error!(e, ComplianceError::NotConfigured);
    }
    e.storage().persistent().set(&ComplianceStorageKey::Frozen(account.clone()), &true);
    emit_frozen(e, account);
}

/// Clears the frozen flag on `account`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to unfreeze.
///
/// # Errors
///
/// * [`ComplianceError::NotConfigured`] - When [`compliance_config`] returns
///   `None`.
///
/// # Events
///
/// * topics - `["unfrozen", account: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn unfreeze(e: &Env, account: &Address) {
    if compliance_config(e).is_none() {
        panic_with_error!(e, ComplianceError::NotConfigured);
    }
    e.storage().persistent().remove(&ComplianceStorageKey::Frozen(account.clone()));
    emit_unfrozen(e, account);
}

// ################## LOW-LEVEL HELPERS ##################

/// Asserts that `account` passes every configured compliance gate against the
/// given `config`: not frozen, authorized by the policy (when one is set),
/// and authorized by the SAC (when `sac_passthrough` is enabled).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to check.
/// * `config` - The active [`ComplianceConfig`].
///
/// # Errors
///
/// * [`ComplianceError::AccountFrozen`] - When `account` is frozen.
/// * refer to [`check_policy`] errors.
/// * refer to [`check_sac`] errors.
pub fn gate_account(e: &Env, account: &Address, config: &ComplianceConfig) {
    if is_frozen(e, account) {
        panic_with_error!(e, ComplianceError::AccountFrozen);
    }
    check_policy(e, account, config);
    check_sac(e, account, config);
}

/// Asserts that the configured external policy authorizes `account` for the
/// current token contract. A no-op when `config.policy` is `None`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to check.
/// * `config` - The active [`ComplianceConfig`].
///
/// # Errors
///
/// * [`ComplianceError::NotAuthorizedByPolicy`] - When the configured policy
///   returns `false` for `account`.
pub fn check_policy(e: &Env, account: &Address, config: &ComplianceConfig) {
    if let Some(policy_addr) = &config.policy {
        let policy = PolicyClient::new(e, policy_addr);
        if !policy.is_authorized(account, &e.current_contract_address()) {
            panic_with_error!(e, ComplianceError::NotAuthorizedByPolicy);
        }
    }
}

/// Asserts that the underlying token's `authorized` view returns `true`
/// for `account`. A no-op when `config.sac_passthrough` is `false`.
///
/// The `authorized` view belongs to the Stellar Asset Contract admin
/// interface, not to generic SEP-41 (DESIGN §3.4). Enabling
/// `sac_passthrough` over a non-SAC underlying (e.g. a plain SEP-41 token)
/// makes this call — and with it every gated operation — trap on the
/// missing function.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to check.
/// * `config` - The active [`ComplianceConfig`].
///
/// # Errors
///
/// * [`ComplianceError::NotAuthorizedBySac`] - When the SAC's `authorized` view
///   returns `false` for `account`.
pub fn check_sac(e: &Env, account: &Address, config: &ComplianceConfig) {
    if config.sac_passthrough {
        let sac = token::StellarAssetClient::new(e, &get_underlying_asset(e));
        if !sac.authorized(account) {
            panic_with_error!(e, ComplianceError::NotAuthorizedBySac);
        }
    }
}
