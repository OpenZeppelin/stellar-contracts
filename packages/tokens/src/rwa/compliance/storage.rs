use soroban_sdk::{contracttype, panic_with_error, vec, Address, Env, IntoVal, Symbol, Vec};

use crate::rwa::compliance::{
    emit_module_added, emit_module_removed, ComplianceError, HookType, COMPLIANCE_EXTEND_AMOUNT,
    COMPLIANCE_TTL_THRESHOLD, MAX_MODULES,
};

/// Storage keys for the modular compliance contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum DataKey {
    /// Maps HookType -> Vec<Address> for registered modules
    HookModules(HookType),
    /// Existence check for a module registered for a specific hook type.
    ModuleRegistered(HookType, Address),
}

// ################## QUERY STATE ##################

/// Returns all modules registered for a specific hook type.
///
/// Extends the TTL of the storage entry if it exists to ensure data
/// availability for future operations.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The hook type to query modules for.
///
/// # Returns
///
/// A vector of module addresses registered for the specified hook.
/// Returns an empty vector if no modules are registered.
pub fn get_modules_for_hook(e: &Env, hook: HookType) -> Vec<Address> {
    let key = DataKey::HookModules(hook);
    if let Some(existing_modules) = e.storage().persistent().get(&key) {
        e.storage().persistent().extend_ttl(
            &key,
            COMPLIANCE_TTL_THRESHOLD,
            COMPLIANCE_EXTEND_AMOUNT,
        );
        existing_modules
    } else {
        Vec::new(e)
    }
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
pub fn is_module_registered(e: &Env, hook: HookType, module: Address) -> bool {
    let existence_key = DataKey::ModuleRegistered(hook, module);
    e.storage().persistent().get::<_, ()>(&existence_key).is_some()
}

// ################## CHANGE STATE ##################

/// Registers a compliance module for a specific hook type.
///
/// This allows modules to opt-in to the events they care about.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The type of hook to register the module for.
/// * `module` - The address of the compliance module contract.
///
/// # Events
///
/// * topics - `["module_added", hook: HookType, module: Address]`
/// * data - `[]`
///
/// # Errors
///
/// * [`ComplianceError::ModuleAlreadyRegistered`] - When the module is already
///   registered for this hook type.
/// * [`ComplianceError::ModuleBoundExceeded`] - When the maximum number of
///   modules per hook is exceeded.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods creates significant
/// security risks as it could allow unauthorized module registration.
pub fn add_module_to(e: &Env, hook: HookType, module: Address) {
    // Check if module is already registered
    let existence_key = DataKey::ModuleRegistered(hook.clone(), module.clone());
    if e.storage().persistent().get::<_, ()>(&existence_key).is_some() {
        e.storage().persistent().extend_ttl(
            &existence_key,
            COMPLIANCE_TTL_THRESHOLD,
            COMPLIANCE_EXTEND_AMOUNT,
        );
        panic_with_error!(e, ComplianceError::ModuleAlreadyRegistered);
    }

    // Get the modules for this hook
    let key = DataKey::HookModules(hook.clone());
    let mut modules: Vec<Address> =
        e.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(e));

    // Check the bound
    if modules.len() >= MAX_MODULES {
        panic_with_error!(e, ComplianceError::ModuleBoundExceeded);
    }

    // Add the module
    modules.push_back(module.clone());
    e.storage().persistent().set(&key, &modules);
    e.storage().persistent().set(&existence_key, &());

    // Emit event
    emit_module_added(e, hook, module);
}

/// Deregisters a compliance module from a specific hook type.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `hook` - The type of hook to unregister the module from.
/// * `module` - The address of the compliance module contract to remove.
///
/// # Events
///
/// * topics - `["module_removed", hook: HookType, module: Address]`
/// * data - `[]`
///
/// # Errors
///
/// * [`ComplianceError::ModuleNotRegistered`] - When the module is not
///   currently registered for this hook type.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods creates significant
/// security risks as it could allow unauthorized module removal.
pub fn remove_module_from(e: &Env, hook: HookType, module: Address) {
    // Check if module is registered
    let existence_key = DataKey::ModuleRegistered(hook.clone(), module.clone());
    if e.storage().persistent().get::<_, ()>(&existence_key).is_none() {
        panic_with_error!(e, ComplianceError::ModuleNotRegistered);
    }

    // Get the modules for this hook
    let key = DataKey::HookModules(hook.clone());
    let mut modules: Vec<Address> =
        e.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(e));

    // Remove the module
    let index = modules.iter().position(|x| x == module).expect("module exists") as u32;
    modules.remove(index);

    // Update storage
    e.storage().persistent().set(&key, &modules);

    let existence_key = DataKey::ModuleRegistered(hook.clone(), module.clone());
    e.storage().persistent().remove(&existence_key);

    // Emit event
    emit_module_removed(e, hook, module);
}

// ################## HOOK EXECUTION ##################

/// Executes all modules registered for the Transfer hook.
///
/// Called after tokens are successfully transferred from one address to
/// another. Only modules that have registered for the Transfer hook will be
/// invoked.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address that sent the tokens.
/// * `to` - The address that received the tokens.
/// * `amount` - The amount of tokens transferred.
///
/// # Cross-Contract Calls
///
/// Invokes `on_transfer(from, to, amount)` on each registered module.
pub fn transferred(e: &Env, from: Address, to: Address, amount: i128) {
    let modules = get_modules_for_hook(e, HookType::Transfer);

    // Call each registered module
    for module_address in modules.iter() {
        let _result: () = e.invoke_contract(
            &module_address,
            &Symbol::new(e, "on_transfer"),
            vec![&e, from.to_val(), to.to_val(), amount.into_val(e)],
        );
    }
}

/// Executes all modules registered for the Created hook.
///
/// Called after tokens are successfully created/minted to an address.
/// Only modules that have registered for the Created hook will be invoked.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address that received the newly created tokens.
/// * `amount` - The amount of tokens created.
///
/// # Cross-Contract Calls
///
/// Invokes `on_created(to, amount)` on each registered module.
pub fn created(e: &Env, to: Address, amount: u32) {
    let modules = get_modules_for_hook(e, HookType::Created);

    // Call each registered module
    for module_address in modules.iter() {
        let _result: () = e.invoke_contract(
            &module_address,
            &Symbol::new(e, "on_created"),
            vec![&e, to.to_val(), amount.into_val(e)],
        );
    }
}

/// Executes all modules registered for the Destroyed hook.
///
/// Called after tokens are successfully destroyed/burned from an address.
/// Only modules that have registered for the Destroyed hook will be invoked.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address from which tokens were destroyed.
/// * `amount` - The amount of tokens destroyed.
///
/// # Cross-Contract Calls
///
/// Invokes `on_destroyed(from, amount)` on each registered module.
pub fn destroyed(e: &Env, from: Address, amount: i128) {
    let modules = get_modules_for_hook(e, HookType::Destroyed);

    // Call each registered module
    for module_address in modules.iter() {
        let _result: () = e.invoke_contract(
            &module_address,
            &Symbol::new(e, "on_destroyed"),
            vec![&e, from.to_val(), amount.into_val(e)],
        );
    }
}

/// Executes all modules registered for the CanTransfer hook to validate a
/// transfer.
///
/// Called during transfer validation to check if a transfer should be allowed.
/// Only modules that have registered for the CanTransfer hook will be invoked.
/// This is a read-only operation and should not modify state.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address attempting to send tokens.
/// * `to` - The address that would receive tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Returns
///
/// `true` if all registered modules allow the transfer, `false` if any module
/// rejects it. Returns `true` if no modules are registered for this hook.
///
/// # Cross-Contract Calls
///
/// Invokes `can_transfer(from, to, amount)` on each registered module.
/// Stops execution and returns `false` on the first module that rejects.
pub fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool {
    // This can be called by anyone for read-only checks
    let modules = get_modules_for_hook(e, HookType::CanTransfer);

    // Call each registered module and check if all return true
    for module_address in modules.iter() {
        let result: bool = e.invoke_contract(
            &module_address,
            &Symbol::new(e, "can_transfer"),
            vec![&e, from.to_val(), to.to_val(), amount.into_val(e)],
        );

        // If any module returns false, the entire check fails
        if !result {
            return false;
        }
    }

    // All modules passed (or no modules registered)
    true
}
