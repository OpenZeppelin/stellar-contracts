use soroban_sdk::{Address, Env, String};

use crate::{Balance, TokenId};

/// Based on the Extension, some default behavior of [`crate::NonFungibleToken`]
/// might have to be overridden. This is a helper trait that allows us this
/// override mechanism that favors the DevX.
///
/// One can also override the `NonFungibleToken` trait directly, but the reason
/// we have another trait for the same methods, is to provide the default
/// implementations in an easier way for the end developer.
///
/// Due to the macro scope limitations, currently `#[contractimpl]` macro cannot
/// find the implemented trait's default methods. All the methods have to be
/// specified under the scope of `#[contractimpl]` macro, otherwise, the missing
/// methods (default ones) won't be available for the generated client.
///
/// This means, the end-developer cannot leave the methods with default
/// implementations out, and must provide these default implementations in the
/// `impl Trait for Contract` block as well. For different extensions, the
/// required overrides are different, and we want to alleviate the burden of
/// remembering for which method the end-developer need to use the default
/// implementation, and for which method the end-developer need to override.
///
/// By introducing this abstraction, we allow the end-developer to implement
/// every method of the `NonFungibleToken` trait using
/// `Self::ContractType::{function_name}`, which will in turn use either the
/// overridden or the base variant according to the extension, provided the by
/// `ContractOverrides` trait implementation for the respective ContractType.
///
/// Example:
///
/// ```rust
/// impl NonFungibleToken for ExampleContract {
///     type ContractType = Consecutive;
///
///     fn balance(e: &Env, owner: Address) -> Balance {
///         Self::ContractType::balance(e, owner)
///     }
///
///     fn owner_of(e: &Env, token_id: TokenId) -> Address {
///         Self::ContractType::owner_of(e, token_id)
///     }
///
///     fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
///         Self::ContractType::transfer(e, &from, &to, token_id);
///     }
///
///     fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
///         Self::ContractType::transfer_from(e, &spender, &from, &to, token_id);
///     }
///
///     /* and so on */
/// }
/// ```
///
/// or the end-developer can use the type directly (in this case `Consecutive`)
/// instead of referring to it as `Self::ContractType`:
///
/// ```rust
/// /// ```rust
/// impl NonFungibleToken for ExampleContract {
///     type ContractType = Consecutive;
///
///     fn balance(e: &Env, owner: Address) -> Balance {
///         Consecutive::balance(e, owner)
///     }
///
///     fn owner_of(e: &Env, token_id: TokenId) -> Address {
///         Consecutive::owner_of(e, token_id)
///     }
///
///     fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
///         Consecutive:transfer(e, &from, &to, token_id);
///     }
///
///     fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
///         Consecutive::transfer_from(e, &spender, &from, &to, token_id);
///     }
///
///     /* and so on */
/// }
/// ```
pub trait ContractOverrides {
    fn balance(e: &Env, owner: Address) -> Balance {
        Base::balance(e, &owner)
    }

    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        Base::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        Base::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        Base::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        Base::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Base::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> {
        Base::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Base::is_approved_for_all(e, &owner, &operator)
    }

    fn name(e: &Env) -> String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol(e)
    }

    fn token_uri(e: &Env, token_id: TokenId) -> String {
        Base::token_uri(e, token_id)
    }
}

/// Default marker type
pub struct Base;

// No override required for the `Base` contract type.
impl ContractOverrides for Base {}
