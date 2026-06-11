use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::{
    compliance::{
        emit_module_added, emit_module_removed, modules::ComplianceModuleClient, AccountSnapshot,
        ComplianceError, ComplianceHook, TransferKind, COMPLIANCE_EXTEND_AMOUNT,
        COMPLIANCE_TTL_THRESHOLD, MAX_MODULES,
    },
    utils::token_binder::is_token_bound,
};

/// Storage keys for the modular compliance contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ComplianceDataKey {
    /// Maps ComplianceHook -> `Vec<Address>` for registered modules
    HookModules(ComplianceHook),
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
pub fn get_modules_for_hook(e: &Env, hook: ComplianceHook) -> Vec<Address> {
    let key = ComplianceDataKey::HookModules(hook);
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
pub fn is_module_registered(e: &Env, hook: ComplianceHook, module: Address) -> bool {
    let modules = get_modules_for_hook(e, hook);
    modules.iter().any(|m| m == module)
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
/// * topics - `["module_added", hook: ComplianceHook, module: Address]`
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
pub fn add_module_to(e: &Env, hook: ComplianceHook, module: Address) {
    // Check if module is already registered
    let mut modules = get_modules_for_hook(e, hook.clone());

    // Check if module is already registered
    if modules.iter().any(|m| m == module) {
        panic_with_error!(e, ComplianceError::ModuleAlreadyRegistered);
    }

    // Check the bound
    if modules.len() >= MAX_MODULES {
        panic_with_error!(e, ComplianceError::ModuleBoundExceeded);
    }

    // Add the module
    let key = ComplianceDataKey::HookModules(hook.clone());
    modules.push_back(module.clone());
    e.storage().persistent().set(&key, &modules);

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
/// * topics - `["module_removed", hook: ComplianceHook, module: Address]`
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
pub fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address) {
    // Get the modules for this hook
    let mut modules = get_modules_for_hook(e, hook.clone());

    // Check if module is registered
    if !modules.iter().any(|m| m == module) {
        panic_with_error!(e, ComplianceError::ModuleNotRegistered);
    }

    // Remove the module from the list
    let index = modules.iter().position(|x| x == module).expect("module exists") as u32;
    modules.remove(index);

    // Update storage
    let key = ComplianceDataKey::HookModules(hook.clone());
    e.storage().persistent().set(&key, &modules);

    // Emit event
    emit_module_removed(e, hook, module);
}

// ################## HOOK EXECUTION ##################

/// Executes all modules registered for the Transferred hook.
///
/// Called when tokens are transferred from one address to another. A module
/// rejects the transfer by panicking, which reverts the whole operation;
/// only modules registered for the Transferred hook will be invoked.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - Snapshot of the sender, as of before the transfer.
/// * `to` - Snapshot of the receiver, as of before the transfer.
/// * `amount` - The amount of tokens transferred.
/// * `kind` - Who initiated the transfer and under what authority; see
///   [`TransferKind`].
/// * `token` - The address of the token contract that is performing the
///   transfer.
///
/// # Errors
///
/// * refer to [`require_auth_from_bound_token`]
///
/// # Cross-Contract Calls
///
/// Invokes `on_transfer(from, to, amount, kind, token)` on each registered
/// module.
pub fn transferred(
    e: &Env,
    from: AccountSnapshot,
    to: AccountSnapshot,
    amount: i128,
    kind: TransferKind,
    token: Address,
) {
    require_auth_from_bound_token(e, &token);

    let modules = get_modules_for_hook(e, ComplianceHook::Transferred);

    for module_address in modules.iter() {
        let client = ComplianceModuleClient::new(e, &module_address);
        client.on_transfer(&from, &to, &amount, &kind, &token);
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
/// * `to` - Snapshot of the receiver, as of before the mint.
/// * `amount` - The amount of tokens created.
/// * `token` - The address of the token contract that is performing the mint.
///
/// # Errors
///
/// * refer to [`require_auth_from_bound_token`]
///
/// # Cross-Contract Calls
///
/// Invokes `on_created(to, amount, token)` on each registered module.
pub fn created(e: &Env, to: AccountSnapshot, amount: i128, token: Address) {
    require_auth_from_bound_token(e, &token);

    let modules = get_modules_for_hook(e, ComplianceHook::Created);

    for module_address in modules.iter() {
        let client = ComplianceModuleClient::new(e, &module_address);
        client.on_created(&to, &amount, &token);
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
/// * `from` - Snapshot of the burned wallet, as of before the burn.
/// * `amount` - The amount of tokens destroyed.
/// * `token` - The address of the token contract that is performing the burn.
///
/// # Errors
///
/// * refer to [`require_auth_from_bound_token`]
///
/// # Cross-Contract Calls
///
/// Invokes `on_destroyed(from, amount, token)` on each registered module.
pub fn destroyed(e: &Env, from: AccountSnapshot, amount: i128, token: Address) {
    require_auth_from_bound_token(e, &token);

    let modules = get_modules_for_hook(e, ComplianceHook::Destroyed);

    for module_address in modules.iter() {
        let client = ComplianceModuleClient::new(e, &module_address);
        client.on_destroyed(&from, &amount, &token);
    }
}

// ################## HELPERS ##################

/// Helper function to check if the token contract is bound to this compliance
/// contract and require authorization from the token contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The address of the token contract to check.
///
/// # Errors
///
/// * `ComplianceError::TokenNotBound` - If the token contract is not bound to
///   this compliance contract.
pub fn require_auth_from_bound_token(e: &Env, token: &Address) {
    // Only the token contract should call this function
    token.require_auth();

    // Check if the token contract is bound to this compliance contract
    // Use is_token_bound for memory efficiency (loads one bucket at a time)
    if !is_token_bound(e, token) {
        panic_with_error!(e, ComplianceError::TokenNotBound);
    }
}
