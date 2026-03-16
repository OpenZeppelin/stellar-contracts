use soroban_sdk::{contracttrait, Address, Env, String};

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
///
///
/// No default implementations are provided for the methods of this trait.
/// [`ComplianceModule`] is designed to be implemented by multiple independent
/// contracts, each with its own storage layout, access control, and business
/// logic. A meaningful default is therefore not possible.
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
