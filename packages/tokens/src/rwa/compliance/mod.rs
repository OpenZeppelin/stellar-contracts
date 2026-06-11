use soroban_sdk::{contracterror, contractevent, contracttrait, contracttype, Address, Env, Vec};

use crate::rwa::utils::token_binder::TokenBinder;

pub mod modules;
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

/// A point-in-time view of one account, captured as of *before* the operation
/// that triggered the hook.
///
/// Soroban forbids reentrancy, and the token contract is still on the call
/// stack while a hook runs, so a module cannot call back into the token to
/// read a balance. The snapshot carries that state into the hook instead, so a
/// module can reason about a wallet's holdings without a balance mirror of its
/// own.
///
/// `balance` and `frozen` are measured at the same instant, before the
/// operation is applied. `balance - frozen` is the wallet's free (movable)
/// amount. Capturing the pre-operation state means the validation hook
/// (`can_transfer`) and the matching state hook (`transferred`) observe
/// identical figures.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountSnapshot {
    /// The wallet address this snapshot describes.
    pub address: Address,
    /// The wallet's total token balance, before the operation.
    pub balance: i128,
    /// The partially-frozen portion of `balance`, before the operation.
    pub frozen: i128,
}

/// Trait for implementing custom compliance logic to RWA tokens.
///
/// The contract implementing this trait serves as the core compliance contract
/// that RWA token contracts interact with. It manages the registration of
/// compliance modules and forwards hook calls to those modules during token
/// operations. In other words, this contract acts as the dispatcher for
/// compliance logic, while the actual logic is implemented in separate module
/// contracts that are registered to it.
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
#[contracttrait]
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
    /// * `from` - Snapshot of the sender, as of before the transfer.
    /// * `to` - Snapshot of the receiver, as of before the transfer.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `spender` - For a `transfer_from`, the delegate that moved the tokens;
    ///   `None` for a direct or forced transfer.
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    fn transferred(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        spender: Option<Address>,
        token: Address,
    ) {
        storage::transferred(e, from, to, amount, spender, token)
    }

    /// Called whenever tokens are created on a wallet.
    ///
    /// This function calls all modules registered for the `Created` hook.
    /// Only modules that need to track minting events will be called.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - Snapshot of the receiver, as of before the mint.
    /// * `amount` - The amount of tokens involved in the minting.
    /// * `token` - The address of the contract that is performing the minting.
    fn created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
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
    /// * `from` - Snapshot of the burned wallet, as of before the burn.
    /// * `amount` - The amount of tokens involved in the burn.
    /// * `token` - The address of the token contract that is performing the
    ///   burn.
    fn destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
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
    /// * `from` - Snapshot of the sender, as of before the transfer.
    /// * `to` - Snapshot of the receiver, as of before the transfer.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `spender` - For a `transfer_from`, the delegate that moved the tokens;
    ///   `None` for a direct or forced transfer.
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    ///
    /// # Returns
    ///
    /// `true` if all registered modules allow the transfer, `false` otherwise.
    fn can_transfer(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        spender: Option<Address>,
        token: Address,
    ) -> bool {
        storage::can_transfer(e, from, to, amount, spender, token)
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
    /// * `to` - Snapshot of the receiver, as of before the mint.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    ///
    /// # Returns
    ///
    /// `true` if all registered modules allow the transfer, `false` otherwise.
    fn can_create(e: &Env, to: AccountSnapshot, amount: i128, token: Address) -> bool {
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

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const COMPLIANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const COMPLIANCE_TTL_THRESHOLD: u32 = COMPLIANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
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
