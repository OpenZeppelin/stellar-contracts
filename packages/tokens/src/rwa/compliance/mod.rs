use soroban_sdk::{Address, Env};

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
/// [`Compliance`] trait's purpose is to provide the ability/flexibility to
/// implement business-specific compliance logic, hence there is no default
/// implementation provided for it.
pub trait Compliance {
    /// Called whenever tokens are transferred from one wallet to another.
    ///
    /// This function can be used to update state variables of the compliance
    /// contract. This function should be called ONLY by the token contract
    /// bound to the compliance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the transfer.
    fn transferred(e: &Env, from: &Address, to: &Address, amount: i128);

    /// Called whenever tokens are created on a wallet.
    ///
    /// This function can be used to update state variables of the compliance
    /// contract. This function should be called ONLY by the token contract
    /// bound to the compliance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the minting.
    fn created(e: &Env, to: &Address, amount: i128);

    /// Called whenever tokens are destroyed from a wallet.
    ///
    /// This function can be used to update state variables of the compliance
    /// contract. This function should be called ONLY by the token contract
    /// bound to the compliance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address on which tokens are burnt.
    /// * `amount` - The amount of tokens involved in the burn.
    fn destroyed(e: &Env, from: &Address, amount: i128);

    /// Checks that the transfer is compliant.
    ///
    /// Default compliance should always return true. This is a READ ONLY
    /// function and should not be used to increment counters, emit events,
    /// etc.
    ///
    /// This function will call all checks implemented on compliance.
    /// If all checks return true, the function returns true,
    /// otherwise returns false.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The amount of tokens involved in the transfer.
    fn can_transfer(e: &Env, from: &Address, to: &Address, amount: i128) -> bool;
}
