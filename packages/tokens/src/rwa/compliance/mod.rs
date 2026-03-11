use soroban_sdk::{
    contractclient, contracterror, contractevent, contracttype, Address, Env, String, Vec,
};

use crate::rwa::utils::token_binder::TokenBinder;

pub mod storage;

#[cfg(test)]
mod test;

/// Hook types for modular compliance system.
///
/// Each hook type represents a specific event or validation point
/// where compliance modules can be executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ComplianceHook {
    /// Called after tokens are successfully transferred from one wallet to
    /// another. Modules registered for this hook can update their state
    /// based on transfer events.
    Transferred,

    /// Called after tokens are successfully created/minted to a wallet.
    /// Modules registered for this hook can update their state based on minting
    /// events.
    Created,

    /// Called after tokens are successfully destroyed/burned from a wallet.
    /// Modules registered for this hook can update their state based on burning
    /// events.
    Destroyed,

    /// Called during transfer validation to check if a transfer should be
    /// allowed. Modules registered for this hook can implement transfer
    /// restrictions. This is a READ-only operation and should not modify
    /// state.
    CanTransfer,

    /// Called during mint validation to check if a mint operation should be
    /// allowed. Modules registered for this hook can implement transfer
    /// restrictions. This is a READ-only operation and should not modify
    /// state.
    CanCreate,
}

/// Trait for implementing custom compliance logic to RWA tokens.
///
/// [`Compliance`] trait is not expected to be an extension to a RWA smart
/// contract, but it is a separate contract on its own. This design allows it to
/// be shared across many RWA tokens. Note that, there is no `RWA` bound on the
/// [`Compliance`] trait:
///
/// ```rust, ignore
/// pub trait Compliance       // ✅
/// pub trait Compliance: RWA  // ❌
/// ```
///
/// # Multi-Token Support
///
/// To enable a single compliance contract to serve multiple RWA tokens, all
/// hook functions accept a `token` parameter identifying the calling RWA
/// token contract. This allows compliance modules to maintain separate state
/// and apply different business logic per token (e.g., token-specific transfer
/// limits, per-token balance tracking).
#[contractclient(name = "ComplianceClient")]
pub trait Compliance: TokenBinder {
    /// Registers a compliance module for a specific hook type.
    /// Only the operator can register modules.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hook` - The type of hook to register the module for.
    /// * `module` - The address of the compliance module contract.
    /// * `operator` - The address of the operator that can add/remove modules.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::add_module_to`] for
    /// the implementation.
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address);

    /// Deregisters a compliance module from a specific hook type.
    /// Only the operator can deregister modules.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hook` - The type of hook to deregister the module from.
    /// * `module` - The address of the compliance module contract.
    /// * `operator` - The address of the operator that can add/remove modules.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::remove_module_from`] for the implementation.
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address);

    /// Gets all modules registered for a specific hook type.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hook` - The hook type to query.
    ///
    /// # Returns
    ///
    /// A vector of module addresses registered for the specified hook.
    fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
        storage::get_modules_for_hook(e, hook)
    }

    /// Checks if a module is registered for a specific hook type.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hook` - The hook type to check.
    /// * `module` - The address of the module to check.
    ///
    /// # Returns
    ///
    /// `true` if the module is registered for the hook, `false` otherwise.
    fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
        storage::is_module_registered(e, hook, module)
    }

    /// Called whenever tokens are transferred from one wallet to another.
    ///
    /// This function calls all modules registered for the `Transfer` hook.
    /// Only modules that need to track transfer events will be called.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    fn transferred(e: &Env, from: Address, to: Address, amount: i128, token: Address) {
        storage::transferred(e, from, to, amount, token)
    }

    /// Called whenever tokens are created on a wallet.
    ///
    /// This function calls all modules registered for the `Created` hook.
    /// Only modules that need to track minting events will be called.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the minting.
    /// * `token` - The address of the contract that is performing the minting.
    fn created(e: &Env, to: Address, amount: i128, token: Address) {
        storage::created(e, to, amount, token)
    }

    /// Called whenever tokens are destroyed from a wallet.
    ///
    /// This function calls all modules registered for the `Destroyed` hook.
    /// Only modules that need to track burning events will be called.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address on which tokens are burnt.
    /// * `amount` - The amount of tokens involved in the burn.
    /// * `token` - The address of the token contract that is performing the
    ///   burn.
    fn destroyed(e: &Env, from: Address, amount: i128, token: Address) {
        storage::destroyed(e, from, amount, token)
    }

    /// Checks whether the transfer is compliant.
    ///
    /// This function calls all modules registered for the `CanTransfer` hook.
    /// If any module returns `false`, the entire check fails.
    /// This is a READ-only function and should not modify state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    ///
    /// # Returns
    ///
    /// `true` if all registered modules allow the transfer, `false` otherwise.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool {
        storage::can_transfer(e, from, to, amount, token)
    }

    /// Checks whether the mint operation is compliant.
    ///
    /// This function calls all modules registered for the `CanCreate` hook.
    /// If any module returns `false`, the entire check fails.
    /// This is a READ-only function and should not modify state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    ///
    /// # Returns
    ///
    /// `true` if all registered modules allow the transfer, `false` otherwise.
    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool {
        storage::can_create(e, to, amount, token)
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ComplianceError {
    /// Indicates a module is already registered for this hook.
    ModuleAlreadyRegistered = 360,
    /// Indicates a module is not registered for this hook.
    ModuleNotRegistered = 361,
    /// Indicates a module bound is exceeded.
    ModuleBoundExceeded = 362,
    /// Indicates a token is not bound to this compliance contract.
    TokenNotBound = 363,
}

/// Error codes shared by all compliance modules.
///
/// Compliance module errors occupy the 390–400 range, following the RWA
/// error numbering convention.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ComplianceModuleError {
    /// The compliance contract address has not been set.
    ComplianceNotSet = 390,
    /// An amount argument is negative when it must be non-negative.
    InvalidAmount = 391,
    /// Arithmetic overflow in a checked addition.
    MathOverflow = 392,
    /// Arithmetic underflow in a checked subtraction.
    MathUnderflow = 393,
    /// A required limit entry is missing for the given token.
    MissingLimit = 394,
    /// A required transfer counter entry is missing.
    MissingCounter = 395,
    /// A required country data entry is missing.
    MissingCountry = 396,
    /// The identity registry storage address has not been configured.
    IdentityRegistryNotSet = 397,
    /// A module is not registered on a required compliance hook.
    MissingRequiredHook = 398,
    /// The compliance contract address has already been set.
    ComplianceAlreadySet = 399,
    /// A token has reached the maximum number of configured limit entries.
    TooManyLimits = 400,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const COMPLIANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const COMPLIANCE_TTL_THRESHOLD: u32 = COMPLIANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
/// TTL extension amount for compliance module storage entries (30 days).
pub const MODULE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// TTL threshold below which compliance module entries are extended (29 days).
pub const MODULE_TTL_THRESHOLD: u32 = MODULE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const MAX_MODULES: u32 = 20;

// ################## EVENTS ##################

/// Event emitted when a module is added to compliance.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleAdded {
    #[topic]
    pub hook: ComplianceHook,
    pub module: Address,
}

/// Emits an event indicating a module has been added to the compliance
/// system.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The hook type the module is registered for.
/// * `module` - The address of the module.
pub fn emit_module_added(e: &Env, hook: ComplianceHook, module: Address) {
    ModuleAdded { hook, module }.publish(e);
}

/// Event emitted when a module is removed from compliance.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleRemoved {
    #[topic]
    pub hook: ComplianceHook,
    pub module: Address,
}

/// Emits an event indicating a module has been removed from the compliance
/// system.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The hook type the module is registered for.
/// * `module` - The address of the module.
pub fn emit_module_removed(e: &Env, hook: ComplianceHook, module: Address) {
    ModuleRemoved { hook, module }.publish(e);
}

// ################## COMPLIANCE MODULE TRAIT ##################

/// Trait for compliance modules that can be registered with the modular
/// compliance system.
///
/// Modules implement this trait to provide specific compliance logic for
/// different hook types. Each module only needs to implement the methods for
/// hooks it registers for.
///
/// # General Workflow
///
/// 1. Token contract calls `set_compliance_address` to set the address of the
///    compliance contract.
/// 2. Operator registers compliance modules via `add_module_to()` for specific
///    hooks.
/// 3. On token operations (`transfer`, `mint`, `burn`):
///    - **Before**: Token contract calls validation hooks (`can_transfer`,
///      `can_create`)
///    - **After**: Token contract calls notification hooks (`transferred`,
///      `created`, `destroyed`)
/// 4. Compliance contract forwards each hook call to all registered modules for
///    that hook type.
///
/// ┌─────────────────┐
/// │  Token Contract │
/// └────────┬────────┘
///          │ 1. set_compliance_address()
///          ▼
/// ┌─────────────────────┐
/// │ Compliance Contract │◄──── 2. add_module_to() / remove_module_from()
/// └──────────┬──────────┘
///            │ 3. On transfer/mint/burn:
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
///   - Transferred/Created/Destroyed: Potentially State-modifying hooks (called
///     after action)
///   - CanTransfer/CanCreate: Validation hooks (called before action,
///     read-only)
///
/// # Security Note
///
/// If the hooks modify state, they should only be called by the compliance
/// contract to ensure security. `set_compliance_address` and
/// `get_compliance_address` will become handy in this case.
///
/// If the hooks do not modify state, there should be no security concern, and
/// they can be called by any contract/caller. In this case,
/// `set_compliance_address` and `get_compliance_address` will probably not
/// used, and one can provide dummy implementations for them.
///
/// # Default Implementations
///
/// No default implementations are provided for the methods of this trait.
/// [`ComplianceModule`] is designed to be implemented by multiple independent
/// contracts, each with its own storage layout, access control, and business
/// logic. A meaningful default is therefore not possible.
#[contractclient(name = "ComplianceModuleClient")]
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
    /// # Security Note
    ///
    /// If this function modifies state, it should be called only by the
    /// compliance contract. To enforce this, add the following at the start of
    /// the implementation:
    ///
    /// ```ignore
    /// get_compliance_address(e).require_auth();
    /// ```
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation.
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
    /// # Security Note
    ///
    /// If this function modifies state, it should be called only by the
    /// compliance contract. To enforce this, add the following at the start of
    /// the implementation:
    ///
    /// ```ignore
    /// get_compliance_address(e).require_auth();
    /// ```
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation.
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
    /// # Security Note
    ///
    /// If this function modifies state, it should be called only by the
    /// compliance contract. To enforce this, add the following at the start of
    /// the implementation:
    ///
    /// ```ignore
    /// get_compliance_address(e).require_auth();
    /// ```
    ///
    /// # Notes
    ///
    /// No default implementation is provided; see the trait-level
    /// documentation.
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

    /// Returns the address of the compliance contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_compliance_address(e: &Env) -> Address;

    /// Sets the address of the compliance contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `compliance` - The address of the compliance contract.
    fn set_compliance_address(e: &Env, compliance: Address);
}
