use soroban_sdk::{contracterror, contractevent, contracttrait, contracttype, Address, Env, Vec};

use crate::rwa::utils::token_binder::TokenBinder;

pub mod modules;
pub mod storage;

#[cfg(test)]
mod test;

/// Hook types for modular compliance system.
///
/// One hook exists per token operation, invoked after the operation's state
/// changes are applied but within the same transaction. A module enforces its
/// policy by panicking from the hook, which reverts the entire operation
/// atomically, and records whatever bookkeeping it needs otherwise.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ComplianceHook {
    /// Called when tokens are transferred from one wallet to another.
    /// Modules registered for this hook can reject the transfer (by
    /// panicking) and update their state based on transfer events.
    Transferred,

    /// Called when tokens are created/minted to a wallet. Modules registered
    /// for this hook can reject the mint (by panicking) and update their
    /// state based on minting events.
    Created,

    /// Called when tokens are destroyed/burned from a wallet. Modules
    /// registered for this hook can reject the burn (by panicking) and update
    /// their state based on burning events.
    Destroyed,
}

/// Describes who initiated a transfer and under what authority, so each
/// compliance module can decide whether its policy applies.
///
/// Privileged operations (`forced_transfer`, `recover_balance`) deliberately
/// bypass investor-facing policy: a sanctions rule should not block a
/// court-ordered seizure or an account recovery the admin is consciously
/// executing. At the same time, bookkeeping modules must still observe the
/// movement or their records drift from reality. Passing the kind into the
/// hook makes that decision explicit and per-module: a policy module exempts
/// the privileged kinds from its checks, while an accounting module updates
/// its books for every kind.
///
/// The two privileged kinds differ in what happens to the tokens. A
/// [`TransferKind::Forced`] transfer is a seizure: the tokens leave the
/// holder, so wallet-bound module state (e.g. a lock schedule) is consumed
/// along with them. A [`TransferKind::Recovery`] transfer is a wallet
/// migration: the same investor continues on a new wallet, so wallet-bound
/// state should move to the destination rather than disappear. A module
/// that treats the two identically should do so as an explicit decision,
/// not as a fallback.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferKind {
    /// The holder moves its own tokens.
    Standard,
    /// A delegate moves the holder's tokens via `transfer_from`; carries the
    /// delegate (spender) address.
    Delegated(Address),
    /// A privileged operation seizes the tokens (`forced_transfer`): the
    /// tokens leave the holder for another party. Policy modules should
    /// generally exempt this kind; bookkeeping must still be applied, and
    /// wallet-bound restrictions are consumed with the departing tokens.
    Forced,
    /// A privileged operation migrates a lost wallet's balance to the same
    /// investor's new wallet (`recover_balance`). Policy modules should
    /// generally exempt this kind; bookkeeping must still be applied, and
    /// wallet-bound state (e.g. lock schedules) should move to the
    /// destination wallet.
    Recovery,
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
/// amount. The hooks run after the operation's state changes, so the snapshot
/// is what gives a module a stable pre-operation view to validate against.
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
    /// This function calls all modules registered for the `Transferred` hook.
    /// A module rejects the transfer by panicking, which reverts the whole
    /// operation; otherwise it updates its own state as needed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - Snapshot of the sender, as of before the transfer.
    /// * `to` - Snapshot of the receiver, as of before the transfer.
    /// * `amount` - The amount of tokens involved in the transfer.
    /// * `kind` - Who initiated the transfer and under what authority; see
    ///   [`TransferKind`].
    /// * `token` - The address of the token contract that is performing the
    ///   transfer.
    fn transferred(
        e: &Env,
        from: AccountSnapshot,
        to: AccountSnapshot,
        amount: i128,
        kind: TransferKind,
        token: Address,
    ) {
        storage::transferred(e, from, to, amount, kind, token)
    }

    /// Called whenever tokens are created on a wallet.
    ///
    /// This function calls all modules registered for the `Created` hook.
    /// A module rejects the mint by panicking, which reverts the whole
    /// operation; otherwise it updates its own state as needed.
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
    /// A module rejects the burn by panicking, which reverts the whole
    /// operation; otherwise it updates its own state as needed.
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
