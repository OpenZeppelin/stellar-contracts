use soroban_sdk::{contractclient, contracterror, contracttype, Address, Env, String, Symbol, Vec};

pub mod storage;

#[cfg(test)]
mod test;

/// Hook types for modular compliance system.
///
/// Each hook type represents a specific event or validation point
/// where compliance modules can be executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum HookType {
    /// Called after tokens are successfully transferred from one wallet to
    /// another. Modules registered for this hook can update their state
    /// based on transfer events.
    Transfer,

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
pub trait Compliance {
    // TODO: add `TokenBinder` bound when it is available

    /// Registers a compliance module for a specific hook type.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hook` - The type of hook to register the module for.
    /// * `module` - The address of the compliance module contract.
    fn add_module_to(e: &Env, hook: HookType, module: Address);

    /// Deregisters a compliance module from a specific hook type.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hook` - The type of hook to deregister the module from.
    /// * `module` - The address of the compliance module contract.
    fn remove_module_from(e: &Env, hook: HookType, module: Address);

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
    fn get_modules_for_hook(e: &Env, hook: HookType) -> Vec<Address>;

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
    fn is_module_registered(e: &Env, hook: HookType, module: Address) -> bool;

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
    fn transferred(e: &Env, from: Address, to: Address, amount: i128);

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
    fn created(e: &Env, to: Address, amount: i128);

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
    fn destroyed(e: &Env, from: Address, amount: i128);

    /// Checks that the transfer is compliant.
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
    ///
    /// # Returns
    ///
    /// `true` if all registered modules allow the transfer, `false` otherwise.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool;
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
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const COMPLIANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const COMPLIANCE_TTL_THRESHOLD: u32 = COMPLIANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const MAX_MODULES: u32 = 20;

// ################## EVENTS ##################

/// Emits an event indicating a module has been added to the compliance system.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The hook type the module is registered for.
/// * `module` - The address of the module.
///
/// # Events
///
/// * topics - `["module_added", hook: u32]`
/// * data - `[module: Address]`
pub fn emit_module_added(e: &Env, hook: HookType, module: Address) {
    let topics = (Symbol::new(e, "module_added"), hook.clone());
    e.events().publish(topics, module)
}

/// Emits an event indicating a module has been removed from the compliance
/// system.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The hook type the module is registered for.
/// * `module` - The address of the module.
///
/// # Events
///
/// * topics - `["module_removed", hook: u32]`
/// * data - `[module: Address]`
pub fn emit_module_removed(e: &Env, hook: HookType, module: Address) {
    let topics = (Symbol::new(e, "module_removed"), hook.clone());
    e.events().publish(topics, module)
}

// ################## COMPLIANCE MODULE TRAIT ##################

/// Trait for compliance modules that can be registered with the modular
/// compliance system.
///
/// Modules implement this trait to provide specific compliance logic for
/// different hook types. Each module only needs to implement the methods for
/// hooks it registers for.
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
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128);

    /// Called when tokens are created/minted (for Created hook).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the tokens.
    /// * `amount` - The amount of tokens created.
    fn on_created(e: &Env, to: Address, amount: i128);

    /// Called when tokens are destroyed/burned (for Destroyed hook).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address from which tokens are burned.
    /// * `amount` - The amount of tokens destroyed.
    fn on_destroyed(e: &Env, from: Address, amount: i128);

    /// Called to check if a transfer should be allowed (for CanTransfer hook).
    ///
    /// This is a read-only function and should not modify state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens to transfer.
    ///
    /// # Returns
    ///
    /// `true` if the transfer should be allowed, `false` otherwise.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool;

    /// Returns the name of the module for identification purposes.
    ///
    /// # Returns
    ///
    /// A string identifying the module.
    fn name() -> String;
}
