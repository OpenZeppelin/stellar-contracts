//! Compliance modules subsystem.
//!
//! Each module is deployed as its own contract that implements
//! [`ComplianceModule`] plus any module-specific trait (e.g.
//! [`country_allow::CountryAllow`], [`country_restrict::CountryRestrict`]).
//! The modular compliance contract dispatches transfer/mint/burn hooks to the
//! modules registered for each hook type. Shared helpers, storage keys, and
//! TTL constants live in [`storage`].

pub mod country_allow;
pub mod country_restrict;
pub mod initial_lockup_period;
pub mod max_balance;
pub mod storage;
pub mod supply_limit;
pub mod time_transfers_limits;
pub mod transfer_allow;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contracttrait, Address, Env, String};

use crate::rwa::compliance::{AccountSnapshot, TransferKind};

/// Trait for compliance modules that can be registered with the modular
/// compliance system.
///
/// Modules are separate contracts from the core compliance contract. Each
/// module implements the hooks it needs and can maintain its own storage,
/// access control, and business logic.
///
/// # General Workflow
///
/// 1. Operator registers compliance modules, using the compliance contract's
///    [`crate::rwa::compliance::Compliance::add_module_to`] for specific hooks.
/// 2. On token operations (`transfer`, `mint`, `burn`), the token contract
///    calls the matching hook (`transferred`, `created`, `destroyed`) on the
///    compliance contract, after applying its own state changes but within the
///    same transaction.
/// 3. Compliance contract forwards each hook call to all registered modules for
///    that hook type. A module rejects the operation by panicking, which
///    reverts the entire transaction; otherwise it updates its own state as
///    needed.
///
/// ┌─────────────────┐
/// │  Token Contract │
/// └────────┬────────┘
///          │ 1. set_compliance_address()
///          ▼
/// ┌──────────────────────────────────────────┐
/// │ Compliance Contract (bound on the token) │◄──── 2. add_module_to() /
/// │                                          │       remove_module_from()
/// └──────────┬───────────────────────────────┘
///            │ 3. On transfer/mint/burn:
///            │
///            │    - transferred() / created() / destroyed()
///            ▼
/// ┌─────────────────────────────────────────────────┐
/// │           Compliance Modules (1..N)             │
/// ├─────────────────────────────────────────────────┤
/// │  • on_transfer()                                │
/// │  • on_created()                                 │
/// │  • on_destroyed()                               │
/// └─────────────────────────────────────────────────┘
///
/// # Hook Types
///
/// One hook per token operation (Transferred/Created/Destroyed), called after
/// the token action within the same transaction. Each hook both enforces
/// policy (panic to reject; everything reverts atomically) and records
/// bookkeeping. The transfer hook receives a
/// [`crate::rwa::compliance::TransferKind`] so a module can exempt
/// privileged (forced/recovery) operations from its policy while still
/// keeping its books true.
///
/// # Security Note
///
/// State-mutating hooks (potentially [`ComplianceModule::on_transfer`],
/// [`ComplianceModule::on_created`], [`ComplianceModule::on_destroyed`])
/// must authenticate their caller. Hook arguments — including `token` —
/// are forgeable: any contract can call these methods directly with
/// arbitrary values. Soroban's invoker auth is single-level by default —
/// the token contract that triggered the operation is not the direct
/// caller of the module hook, so `token.require_auth()` does not succeed
/// out of the box; the dispatcher (the compliance contract bound to the
/// token) is the direct caller.
///
/// There is a primitive that can extend a contract's auth deeper into
/// the call tree:`Env::authorize_as_current_contract`. With it, the
/// token could pre-authorize the specific module hook invocation so that
/// `token.require_auth()` succeeds inside the module. We deliberately do
/// not use this pattern in the library: it requires the token contract to
/// know every registered module's address, function signature, and exact
/// argument values, and to re-emit that auth tree on every transfer, mint,
/// and burn. Adding or changing a module would then require redeploying
/// every bound token. The per-token `(token → dispatcher)` binding below
/// keeps the token ignorant of the module layer at the cost of one
/// persistent storage entry per (module, token) pair, which we consider
/// the better trade-off for a modular compliance system. Implementors are
/// of course free to choose differently for their own deployments.
///
/// The canonical pattern is to record a per-token mapping of authorized
/// dispatcher addresses (via
/// [`ComplianceModule::set_compliance_address`] / the storage helper
/// [`storage::set_compliance_address`]) and call
/// [`ComplianceModule::get_compliance_address`] inside each state-mutating
/// hook to look up the dispatcher bound to the `token` argument, then
/// `require_auth()` on it:
///
/// ```ignore
/// fn on_transfer(e: &Env, _from: AccountSnapshot, _to: AccountSnapshot, _amount: i128, _kind: TransferKind, token: Address) {
///     // Only the dispatcher bound to `token` may drive this module's
///     // state for that token. A malicious caller passing a forged `token`
///     // argument cannot satisfy this auth check because they did not
///     // make the call as the bound dispatcher.
///     Self::get_compliance_address(e, token.clone()).require_auth();
///
///     // ... module-specific per-token state mutation ...
/// }
/// ```
///
/// The per-token shape lets one module instance be reused across multiple
/// dispatchers safely: a dispatcher that didn't bind itself for `token`
/// cannot drive that token's state.
///
/// A hook implementation that mutates no state (a pure policy rule that
/// only checks and panics) needs no authentication: an unauthenticated
/// call can at worst fail the caller's own transaction, it cannot corrupt
/// anything.
///
/// [`ComplianceModule::get_compliance_address`] has a default
/// implementation that delegates to the storage layer. Every other method
/// must be implemented by each module contract: this trait is designed to
/// be implemented by multiple independent contracts, each with its own
/// storage layout, access control, and business logic, so meaningful
/// defaults are otherwise impossible.
#[contracttrait]
pub trait ComplianceModule {
    /// Called when tokens are transferred (for Transferred hook). Rejects the
    /// transfer by panicking; updates module state otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - Snapshot of the sender, as of before the transfer.
    /// * `to` - Snapshot of the receiver, as of before the transfer.
    /// * `amount` - The amount of tokens transferred.
    /// * `kind` - Who initiated the transfer and under what authority; see
    ///   [`TransferKind`]. Policy modules should generally exempt the
    ///   privileged kinds ([`TransferKind::Forced`],
    ///   [`TransferKind::Recovery`]) from their checks, while still applying
    ///   any bookkeeping. Bookkeeping modules must also decide what a recovery
    ///   means for their state: wallet-bound records should migrate to the
    ///   destination wallet, while identity-bound records are typically
    ///   unaffected.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation. If this implementation mutates state, the trait-level
    /// `# Security Note` applies — authenticate the caller via
    /// [`storage::get_compliance_address`].
    fn on_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        kind: TransferKind,
        token: Address,
    );

    /// Called when tokens are created/minted (for Created hook). Rejects the
    /// mint by panicking; updates module state otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - Snapshot of the receiver, as of before the mint.
    /// * `amount` - The amount of tokens created.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation. If this implementation mutates state, the trait-level
    /// `# Security Note` applies — authenticate the caller via
    /// [`storage::get_compliance_address`].
    fn on_created(e: &Env, to: AccountSnapshot, amount: i128, token: Address);

    /// Called when tokens are destroyed/burned (for Destroyed hook). Rejects
    /// the burn by panicking; updates module state otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - Snapshot of the burned wallet, as of before the burn.
    /// * `amount` - The amount of tokens destroyed.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation. If this implementation mutates state, the trait-level
    /// `# Security Note` applies — authenticate the caller via
    /// [`storage::get_compliance_address`].
    fn on_destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address);

    /// Returns the name of the module for identification purposes.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation.
    fn name(e: &Env) -> String;

    /// Returns the dispatcher address authorized to drive hooks for `token`.
    ///
    /// State-mutating modules should call this and `require_auth()` on the
    /// result at the top of each `on_*` hook — see the trait-level
    /// `# Security Note` for the recommended pattern and an example.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose authorized dispatcher is being queried.
    ///
    /// # Errors
    ///
    /// * [`ComplianceModuleError::ComplianceNotSet`] - When no dispatcher has
    ///   been bound for `token`.
    fn get_compliance_address(e: &Env, token: Address) -> Address {
        storage::get_compliance_address(e, &token)
    }

    /// Binds `compliance` as the dispatcher authorized to drive hooks for
    /// `token`. Calling this with a new value overwrites any prior binding
    /// for the same token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose dispatcher binding is being configured.
    /// * `compliance` - The dispatcher address that should be authorized to
    ///   call this module's hooks for `token`.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::set_compliance_address`] for the implementation.
    fn set_compliance_address(e: &Env, token: Address, compliance: Address, operator: Address);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ComplianceModuleError {
    /// An amount argument is negative when it must be non-negative.
    InvalidAmount = 390,
    /// Arithmetic overflow in a checked addition.
    MathOverflow = 391,
    /// Arithmetic underflow in a checked subtraction.
    MathUnderflow = 392,
    /// A transfer or mint would push an identity's aggregate balance above the
    /// configured maximum.
    MaxBalanceExceeded = 393,
    /// A mint would push the tracked supply above the configured limit.
    SupplyLimitExceeded = 394,
    /// A preset operation was attempted after the preset phase has been
    /// finalized.
    PresetAlreadyCompleted = 395,
    /// The identity registry storage address has not been configured.
    IdentityRegistryNotSet = 396,
    /// The two parallel arrays in a batch call have different lengths.
    BatchSizeMismatch = 397,
    /// No authorized compliance dispatcher has been bound for the given
    /// token.
    ComplianceNotSet = 398,
    /// A transfer or burn would consume more unlocked tokens than the sender
    /// holds.
    InsufficientUnlockedBalance = 399,
    /// A transfer would push the sender identity's cumulative volume above a
    /// configured time-window limit.
    TransferLimitExceeded = 401,
    /// Adding another time-window limit would exceed the per-token bound.
    LimitBoundExceeded = 402,
    /// No time-window limit exists for the given window duration.
    LimitNotFound = 403,
    /// The transfer recipient's country is not on the allowlist.
    CountryNotAllowed = 404,
    /// The transfer recipient's country is on the restriction list.
    CountryRestricted = 405,
    /// Neither transfer party is on the allowlist.
    UserNotAllowed = 406,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for compliance module storage entries (30 days).
pub const MODULE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// TTL threshold below which compliance module entries are extended (29 days).
pub const MODULE_TTL_THRESHOLD: u32 = MODULE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
