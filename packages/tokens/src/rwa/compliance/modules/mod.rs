use soroban_sdk::{contractclient, Address, Env, String};

/// Trait for compliance modules that can be registered with the modular
/// compliance system.
///
/// Modules are separate contracts from the core compliance contract. Each
/// module implements the hooks it needs and can maintain its own storage,
/// access control, and business logic.
///
/// # General Workflow
///
/// 1. Token contract calls `set_compliance_address` to store the compliance
///    contract address.
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
///   - Transferred/Created/Destroyed: Potentially state-modifying hooks called
///     after the token action
///   - CanTransfer/CanCreate: Validation hooks called before the token action
///
/// # Security Note
///
/// If a hook modifies state, it should typically only be called by the
/// compliance contract. `set_compliance_address` and `get_compliance_address`
/// are intended to support that pattern.
///
/// If a hook is read-only, it can be safely exposed more broadly and those
/// methods can use simple or dummy implementations.
#[contractclient(name = "ComplianceModuleClient")]
pub trait ComplianceModule {
    /// Called when tokens are transferred (for Transfer hook).
    fn on_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address);

    /// Called when tokens are created or minted (for Created hook).
    fn on_created(e: &Env, to: Address, amount: i128, token: Address);

    /// Called when tokens are destroyed or burned (for Destroyed hook).
    fn on_destroyed(e: &Env, from: Address, amount: i128, token: Address);

    /// Called to check whether a transfer should be allowed.
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128, token: Address) -> bool;

    /// Called to check whether a mint operation should be allowed.
    fn can_create(e: &Env, to: Address, amount: i128, token: Address) -> bool;

    /// Returns the module name for identification purposes.
    fn name(e: &Env) -> String;

    /// Returns the address of the compliance contract.
    fn get_compliance_address(e: &Env) -> Address;

    /// Sets the address of the compliance contract.
    fn set_compliance_address(e: &Env, compliance: Address);
}
