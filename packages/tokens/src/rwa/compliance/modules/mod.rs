//! Compliance modules subsystem.
//!
//! Each module is deployed as its own contract that implements
//! [`ComplianceModule`] plus any module-specific trait (e.g.
//! [`country_allow::CountryAllow`], [`country_restrict::CountryRestrict`]).
//! The modular compliance contract dispatches transfer/mint/burn hooks to the
//! modules registered for each hook type. Shared helpers, storage keys, and
//! TTL constants live in [`storage`].

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contracttrait, Address, Env, String};

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
/// 2. On token operations (`transfer`, `mint`, `burn`):
///    - **Before**: Token contract calls validation hooks (`can_transfer`,
///      `can_create`) on the compliance contract.
///    - **After**: Token contract calls notification hooks (`transferred`,
///      `created`, `destroyed`) on the compliance contract.
/// 3. Compliance contract forwards each hook call to all registered modules for
///    that hook type.
///
/// ┌──────────────────────────────────────────┐
/// │ Compliance Contract (bound on the token) │◄──── 1. add_module_to() /
/// │                                          │       remove_module_from()
/// └──────────┬───────────────────────────────┘
///            │ 2. On transfer/mint/burn:
///            │
///            │    - transferred() / created() / destroyed()
///            │    - can_transfer() / can_create()
///            ▼
/// ┌─────────────────────────────────────────────────┐
/// │           Compliance Modules (1..N)             │
/// ├─────────────────────────────────────────────────┤
/// │  • on_transfer()    • can_transfer()            │
/// │  • on_created()     • can_create()              │
/// │  • on_destroyed()                               │
/// └─────────────────────────────────────────────────┘
///
/// # Hook Types
///
///   - Transferred/Created/Destroyed: Potentially state-modifying hooks called
///     after the token action
///   - CanTransfer/CanCreate: Validation hooks called before the token action
///
/// # Security Note
///
/// State-mutating hooks, (potentially[`ComplianceModule::on_transfer`],
/// [`ComplianceModule::on_created`], [`ComplianceModule::on_destroyed`])
/// must authenticate their caller. Hook arguments — including `token` —
/// are forgeable: any contract can call these methods directly with
/// arbitrary values, and Soroban provides no built-in caller-identity
/// primitive beyond `require_auth()`. Soroban's invoker auth is also
/// single-level — the token contract that triggered the operation is not
/// the direct caller of the module hook, so `token.require_auth()` does
/// not work; the dispatcher (Compliance Contract that is bound to the token)
/// is the direct caller.
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
/// fn on_transfer(e: &Env, _from: Address, _to: Address, _amount: i128, token: Address) {
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
/// Read-only hooks ([`ComplianceModule::can_transfer`],
/// [`ComplianceModule::can_create`]) do not mutate state, so
/// unauthenticated calls are harmless — they may return a boolean but
/// cannot corrupt anything.
///
/// [`ComplianceModule::get_compliance_address`] has a default
/// implementation that delegates to the storage layer. Every other method
/// must be implemented by each module contract: this trait is designed to
/// be implemented by multiple independent contracts, each with its own
/// storage layout, access control, and business logic, so meaningful
/// defaults are otherwise impossible.
#[contracttrait]
pub trait ComplianceModule {
    /// Called when tokens are transferred (for Transfer hook).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens transferred.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation. If this implementation mutates state, the trait-level
    /// `# Security Note` applies — authenticate the caller via
    /// [`storage::get_compliance_address`].
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address);

    /// Called when tokens are created/minted (for Created hook).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the tokens.
    /// * `amount` - The amount of tokens created.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation. If this implementation mutates state, the trait-level
    /// `# Security Note` applies — authenticate the caller via
    /// [`storage::get_compliance_address`].
    fn on_created(e: &Env, to: Address, amount: i128, token: Address);

    /// Called when tokens are destroyed/burned (for Destroyed hook).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address from which tokens are burned.
    /// * `amount` - The amount of tokens destroyed.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation. If this implementation mutates state, the trait-level
    /// `# Security Note` applies — authenticate the caller via
    /// [`storage::get_compliance_address`].
    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address);

    /// Called to check if a transfer should be allowed (for CanTransfer hook).
    /// Returns `true` if the transfer should be allowed, `false` otherwise.
    ///
    /// This is a read-only function and should not modify state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens to transfer.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool;

    /// Called to check if a mint operation should be allowed (for CanCreate
    /// hook). Returns `true` if the mint operation should be allowed,
    /// `false` otherwise.
    ///
    /// This is a read-only function and should not modify state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens to mint.
    /// * `token` - The address of the token contract that triggered the hook.
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation.
    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool;

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
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should
    /// be enforced before calling [`storage::set_compliance_address`] for
    /// the implementation.
    fn set_compliance_address(e: &Env, token: Address, compliance: Address);
}

// ################## ERRORS ##################

/// Error codes shared by all compliance modules.
///
/// Compliance module errors occupy the 390–400 range, following the RWA
/// error numbering convention.
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
    /// A required limit entry is missing for the given token.
    MissingLimit = 393,
    /// A required transfer counter entry is missing.
    MissingCounter = 394,
    /// A required country data entry is missing.
    MissingCountry = 395,
    /// The identity registry storage address has not been configured.
    IdentityRegistryNotSet = 396,
    /// A token has reached the maximum number of configured limit entries.
    TooManyLimits = 397,
    /// No authorized compliance dispatcher has been bound for the given
    /// token.
    ComplianceNotSet = 398,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;

/// TTL extension amount for compliance module storage entries (30 days).
pub const MODULE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// TTL threshold below which compliance module entries are extended (29 days).
pub const MODULE_TTL_THRESHOLD: u32 = MODULE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
