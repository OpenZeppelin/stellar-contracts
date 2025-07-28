use soroban_sdk::{Address, Env};

use crate::fungible::{extensions::burnable::emit_burn, FTBase};

impl super::FungibleBurnable for FTBase {
    type Impl = FTBase;
    /// Destroys `amount` of tokens from `from`. Updates the total
    /// supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose tokens are destroyed.
    /// * `amount` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`FTBase::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    fn burn(e: &Env, from: &Address, amount: i128) {
        from.require_auth();
        FTBase::update(e, Some(from), None, amount);
        emit_burn(e, from, amount);
    }

    /// Destroys `amount` of tokens from `from` using the allowance mechanism.
    /// `amount` is then deducted from `spender` allowance.
    /// Updates the total supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its
    ///   allowance.
    /// * `from` - The account whose tokens are destroyed.
    /// * `amount` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`FTBase::spend_allowance`] errors.
    /// * refer to [`FTBase::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        spender.require_auth();
        FTBase::spend_allowance(e, from, spender, amount);
        FTBase::update(e, Some(from), None, amount);
        emit_burn(e, from, amount);
    }
}
