#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use core::prelude::rust_2021::*;
#[macro_use]
extern crate core;
extern crate compiler_builtins as _;
mod contract {
    //! Fungible Pausable Example Contract.
    //! This contract showcases how to integrate various OpenZeppelin modules to
    //! build a fully SEP-41-compliant fungible token. It includes essential
    //! features such as an emergency stop mechanism and controlled token minting by
    //! the owner.
    //!
    //! To meet SEP-41 compliance, the contract must implement both
    //! [`openzeppelin_fungible_token::fungible::FungibleToken`] and
    //! [`openzeppelin_fungible_token::burnable::FungibleBurnable`].
    use openzeppelin_fungible_token::{
        self as fungible, burnable::FungibleBurnable, impl_token_interface,
        mintable::FungibleMintable, FungibleToken,
    };
    use openzeppelin_pausable::{self as pausable, Pausable};
    use openzeppelin_pausable_macros::when_not_paused;
    use soroban_sdk::{
        contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env,
        String, Symbol,
    };
    pub const OWNER: Symbol = {
        #[allow(deprecated)]
        const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("OWNER");
        SYMBOL
    };
    pub struct ExampleContract;
    ///ExampleContractArgs is a type for building arg lists for functions defined in "ExampleContract".
    pub struct ExampleContractArgs;
    ///ExampleContractClient is a client for calling the contract defined in "ExampleContract".
    pub struct ExampleContractClient<'a> {
        pub env: soroban_sdk::Env,
        pub address: soroban_sdk::Address,
        #[doc(hidden)]
        _phantom: core::marker::PhantomData<&'a ()>,
    }
    impl<'a> ExampleContractClient<'a> {
        pub fn new(env: &soroban_sdk::Env, address: &soroban_sdk::Address) -> Self {
            Self {
                env: env.clone(),
                address: address.clone(),
                _phantom: core::marker::PhantomData,
            }
        }
    }
    pub enum ExampleContractError {
        Unauthorized = 1,
    }
    pub static __SPEC_XDR_TYPE_EXAMPLECONTRACTERROR: [u8; 64usize] =
        ExampleContractError::spec_xdr();
    impl ExampleContractError {
        pub const fn spec_xdr() -> [u8; 64usize] {
            * b"\0\0\0\x04\0\0\0\0\0\0\0\0\0\0\0\x14ExampleContractError\0\0\0\x01\0\0\0\0\0\0\0\x0cUnauthorized\0\0\0\x01"
        }
    }
    impl TryFrom<soroban_sdk::Error> for ExampleContractError {
        type Error = soroban_sdk::Error;
        #[inline(always)]
        fn try_from(error: soroban_sdk::Error) -> Result<Self, soroban_sdk::Error> {
            if error.is_type(soroban_sdk::xdr::ScErrorType::Contract) {
                let discriminant = error.get_code();
                Ok(match discriminant {
                    1u32 => Self::Unauthorized,
                    _ => return Err(error),
                })
            } else {
                Err(error)
            }
        }
    }
    impl TryFrom<&soroban_sdk::Error> for ExampleContractError {
        type Error = soroban_sdk::Error;
        #[inline(always)]
        fn try_from(error: &soroban_sdk::Error) -> Result<Self, soroban_sdk::Error> {
            <_ as TryFrom<soroban_sdk::Error>>::try_from(*error)
        }
    }
    impl From<ExampleContractError> for soroban_sdk::Error {
        #[inline(always)]
        fn from(val: ExampleContractError) -> soroban_sdk::Error {
            <_ as From<&ExampleContractError>>::from(&val)
        }
    }
    impl From<&ExampleContractError> for soroban_sdk::Error {
        #[inline(always)]
        fn from(val: &ExampleContractError) -> soroban_sdk::Error {
            match val {
                ExampleContractError::Unauthorized => soroban_sdk::Error::from_contract_error(1u32),
            }
        }
    }
    impl TryFrom<soroban_sdk::InvokeError> for ExampleContractError {
        type Error = soroban_sdk::InvokeError;
        #[inline(always)]
        fn try_from(error: soroban_sdk::InvokeError) -> Result<Self, soroban_sdk::InvokeError> {
            match error {
                soroban_sdk::InvokeError::Abort => Err(error),
                soroban_sdk::InvokeError::Contract(code) => Ok(match code {
                    1u32 => Self::Unauthorized,
                    _ => return Err(error),
                }),
            }
        }
    }
    impl TryFrom<&soroban_sdk::InvokeError> for ExampleContractError {
        type Error = soroban_sdk::InvokeError;
        #[inline(always)]
        fn try_from(error: &soroban_sdk::InvokeError) -> Result<Self, soroban_sdk::InvokeError> {
            <_ as TryFrom<soroban_sdk::InvokeError>>::try_from(*error)
        }
    }
    impl From<ExampleContractError> for soroban_sdk::InvokeError {
        #[inline(always)]
        fn from(val: ExampleContractError) -> soroban_sdk::InvokeError {
            <_ as From<&ExampleContractError>>::from(&val)
        }
    }
    impl From<&ExampleContractError> for soroban_sdk::InvokeError {
        #[inline(always)]
        fn from(val: &ExampleContractError) -> soroban_sdk::InvokeError {
            match val {
                ExampleContractError::Unauthorized => soroban_sdk::InvokeError::Contract(1u32),
            }
        }
    }
    impl soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val> for ExampleContractError {
        type Error = soroban_sdk::ConversionError;
        #[inline(always)]
        fn try_from_val(
            env: &soroban_sdk::Env,
            val: &soroban_sdk::Val,
        ) -> Result<Self, soroban_sdk::ConversionError> {
            use soroban_sdk::TryIntoVal;
            let error: soroban_sdk::Error = val.try_into_val(env)?;
            error.try_into().map_err(|_| soroban_sdk::ConversionError)
        }
    }
    impl soroban_sdk::TryFromVal<soroban_sdk::Env, ExampleContractError> for soroban_sdk::Val {
        type Error = soroban_sdk::ConversionError;
        #[inline(always)]
        fn try_from_val(
            env: &soroban_sdk::Env,
            val: &ExampleContractError,
        ) -> Result<Self, soroban_sdk::ConversionError> {
            let error: soroban_sdk::Error = val.into();
            Ok(error.into())
        }
    }
    impl ExampleContract {
        pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
            fungible::metadata::set_metadata(
                e,
                18,
                String::from_str(e, "My Token"),
                String::from_str(e, "TKN"),
            );
            fungible::mintable::mint(e, &owner, initial_supply);
            e.storage().instance().set(&OWNER, &owner);
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN___CONSTRUCTOR: [u8; 84usize] =
        ExampleContract::spec_xdr___constructor();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr___constructor() -> [u8; 84usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\r__constructor\0\0\0\0\0\0\x02\0\0\0\0\0\0\0\x05owner\0\0\0\0\0\0\x13\0\0\0\0\0\0\0\x0einitial_supply\0\0\0\0\0\x0b\0\0\0\0"
        }
    }
    impl<'a> ExampleContractClient<'a> {}
    impl ExampleContractArgs {
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn __constructor<'i>(
            owner: &'i Address,
            initial_supply: &'i i128,
        ) -> (&'i Address, &'i i128) {
            (owner, initial_supply)
        }
    }
    #[doc(hidden)]
    pub mod ____constructor {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).__constructor` instead"
        )]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::__constructor(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).__constructor` instead"
        )]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1)
        }
        use super::*;
    }
    impl Pausable for ExampleContract {
        fn paused(e: &Env) -> bool {
            pausable::paused(e)
        }
        fn pause(e: &Env, caller: Address) {
            let owner: Address = e
                .storage()
                .instance()
                .get(&OWNER)
                .expect("owner should be set");
            if owner != caller {
                {
                    e.panic_with_error(ExampleContractError::Unauthorized);
                }
            }
            pausable::pause(e, &caller);
        }
        fn unpause(e: &Env, caller: Address) {
            let owner: Address = e
                .storage()
                .instance()
                .get(&OWNER)
                .expect("owner should be set");
            if owner != caller {
                {
                    e.panic_with_error(ExampleContractError::Unauthorized);
                }
            }
            pausable::unpause(e, &caller);
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_PAUSED: [u8; 32usize] = ExampleContract::spec_xdr_paused();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_paused() -> [u8; 32usize] {
            *b"\0\0\0\0\0\0\0\0\0\0\0\x06paused\0\0\0\0\0\0\0\0\0\x01\0\0\0\x01"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_PAUSE: [u8; 48usize] = ExampleContract::spec_xdr_pause();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_pause() -> [u8; 48usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x05pause\0\0\0\0\0\0\x01\0\0\0\0\0\0\0\x06caller\0\0\0\0\0\x13\0\0\0\0"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_UNPAUSE: [u8; 48usize] = ExampleContract::spec_xdr_unpause();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_unpause() -> [u8; 48usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x07unpause\0\0\0\0\x01\0\0\0\0\0\0\0\x06caller\0\0\0\0\0\x13\0\0\0\0"
        }
    }
    impl<'a> ExampleContractClient<'a> {
        pub fn paused(&self) -> bool {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("paused");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn try_paused(
            &self,
        ) -> Result<
            Result<
                bool,
                <bool as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("paused");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn pause(&self, caller: &Address) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("pause");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(&self.env, [caller.into_val(&self.env)]),
            );
            res
        }
        pub fn try_pause(
            &self,
            caller: &Address,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("pause");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(&self.env, [caller.into_val(&self.env)]),
            );
            res
        }
        pub fn unpause(&self, caller: &Address) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("unpause");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(&self.env, [caller.into_val(&self.env)]),
            );
            res
        }
        pub fn try_unpause(
            &self,
            caller: &Address,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("unpause");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(&self.env, [caller.into_val(&self.env)]),
            );
            res
        }
    }
    impl ExampleContractArgs {
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn paused<'i>() -> () {
            ()
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn pause<'i>(caller: &'i Address) -> (&'i Address,) {
            (caller,)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn unpause<'i>(caller: &'i Address) -> (&'i Address,) {
            (caller,)
        }
    }
    #[doc(hidden)]
    pub mod __paused {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).paused` instead")]
        pub fn invoke_raw(env: soroban_sdk::Env) -> soroban_sdk::Val {
            use super::Pausable;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::paused(&env),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).paused` instead")]
        pub extern "C" fn invoke_raw_extern(env: soroban_sdk::Env) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __pause {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).pause` instead")]
        pub fn invoke_raw(env: soroban_sdk::Env, arg_0: soroban_sdk::Val) -> soroban_sdk::Val {
            use super::Pausable;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::pause(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).pause` instead")]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __unpause {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).unpause` instead")]
        pub fn invoke_raw(env: soroban_sdk::Env, arg_0: soroban_sdk::Val) -> soroban_sdk::Val {
            use super::Pausable;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::unpause(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).unpause` instead")]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0)
        }
        use super::*;
    }
    impl FungibleToken for ExampleContract {
        fn total_supply(e: &Env) -> i128 {
            fungible::total_supply(e)
        }
        fn balance(e: &Env, account: Address) -> i128 {
            fungible::balance(e, &account)
        }
        fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
            fungible::allowance(e, &owner, &spender)
        }
        fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
            openzeppelin_pausable::when_not_paused(e);
            {
                fungible::transfer(e, &from, &to, amount);
            }
        }
        fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
            openzeppelin_pausable::when_not_paused(e);
            {
                fungible::transfer_from(e, &spender, &from, &to, amount);
            }
        }
        fn approve(
            e: &Env,
            owner: Address,
            spender: Address,
            amount: i128,
            live_until_ledger: u32,
        ) {
            fungible::approve(e, &owner, &spender, amount, live_until_ledger);
        }
        fn decimals(e: &Env) -> u32 {
            fungible::metadata::decimals(e)
        }
        fn name(e: &Env) -> String {
            fungible::metadata::name(e)
        }
        fn symbol(e: &Env) -> String {
            fungible::metadata::symbol(e)
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_TOTAL_SUPPLY: [u8; 36usize] = ExampleContract::spec_xdr_total_supply();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_total_supply() -> [u8; 36usize] {
            *b"\0\0\0\0\0\0\0\0\0\0\0\x0ctotal_supply\0\0\0\0\0\0\0\x01\0\0\0\x0b"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_BALANCE: [u8; 52usize] = ExampleContract::spec_xdr_balance();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_balance() -> [u8; 52usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x07balance\0\0\0\0\x01\0\0\0\0\0\0\0\x07account\0\0\0\0\x13\0\0\0\x01\0\0\0\x0b"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_ALLOWANCE: [u8; 76usize] = ExampleContract::spec_xdr_allowance();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_allowance() -> [u8; 76usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\tallowance\0\0\0\0\0\0\x02\0\0\0\0\0\0\0\x05owner\0\0\0\0\0\0\x13\0\0\0\0\0\0\0\x07spender\0\0\0\0\x13\0\0\0\x01\0\0\0\x0b"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_TRANSFER: [u8; 80usize] = ExampleContract::spec_xdr_transfer();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_transfer() -> [u8; 80usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x08transfer\0\0\0\x03\0\0\0\0\0\0\0\x04from\0\0\0\x13\0\0\0\0\0\0\0\x02to\0\0\0\0\0\x13\0\0\0\0\0\0\0\x06amount\0\0\0\0\0\x0b\0\0\0\0"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_TRANSFER_FROM: [u8; 108usize] =
        ExampleContract::spec_xdr_transfer_from();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_transfer_from() -> [u8; 108usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\rtransfer_from\0\0\0\0\0\0\x04\0\0\0\0\0\0\0\x07spender\0\0\0\0\x13\0\0\0\0\0\0\0\x04from\0\0\0\x13\0\0\0\0\0\0\0\x02to\0\0\0\0\0\x13\0\0\0\0\0\0\0\x06amount\0\0\0\0\0\x0b\0\0\0\0"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_APPROVE: [u8; 120usize] = ExampleContract::spec_xdr_approve();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_approve() -> [u8; 120usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x07approve\0\0\0\0\x04\0\0\0\0\0\0\0\x05owner\0\0\0\0\0\0\x13\0\0\0\0\0\0\0\x07spender\0\0\0\0\x13\0\0\0\0\0\0\0\x06amount\0\0\0\0\0\x0b\0\0\0\0\0\0\0\x11live_until_ledger\0\0\0\0\0\0\x04\0\0\0\0"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_DECIMALS: [u8; 32usize] = ExampleContract::spec_xdr_decimals();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_decimals() -> [u8; 32usize] {
            *b"\0\0\0\0\0\0\0\0\0\0\0\x08decimals\0\0\0\0\0\0\0\x01\0\0\0\x04"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_NAME: [u8; 28usize] = ExampleContract::spec_xdr_name();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_name() -> [u8; 28usize] {
            *b"\0\0\0\0\0\0\0\0\0\0\0\x04name\0\0\0\0\0\0\0\x01\0\0\0\x10"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_SYMBOL: [u8; 32usize] = ExampleContract::spec_xdr_symbol();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_symbol() -> [u8; 32usize] {
            *b"\0\0\0\0\0\0\0\0\0\0\0\x06symbol\0\0\0\0\0\0\0\0\0\x01\0\0\0\x10"
        }
    }
    impl<'a> ExampleContractClient<'a> {
        pub fn total_supply(&self) -> i128 {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{ soroban_sdk::Symbol::new(&self.env, "total_supply") },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn try_total_supply(
            &self,
        ) -> Result<
            Result<
                i128,
                <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{ soroban_sdk::Symbol::new(&self.env, "total_supply") },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn balance(&self, account: &Address) -> i128 {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("balance");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(&self.env, [account.into_val(&self.env)]),
            );
            res
        }
        pub fn try_balance(
            &self,
            account: &Address,
        ) -> Result<
            Result<
                i128,
                <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("balance");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(&self.env, [account.into_val(&self.env)]),
            );
            res
        }
        pub fn allowance(&self, owner: &Address, spender: &Address) -> i128 {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("allowance");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [owner.into_val(&self.env), spender.into_val(&self.env)],
                ),
            );
            res
        }
        pub fn try_allowance(
            &self,
            owner: &Address,
            spender: &Address,
        ) -> Result<
            Result<
                i128,
                <i128 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("allowance");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [owner.into_val(&self.env), spender.into_val(&self.env)],
                ),
            );
            res
        }
        pub fn transfer(&self, from: &Address, to: &Address, amount: &i128) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("transfer");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        from.into_val(&self.env),
                        to.into_val(&self.env),
                        amount.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn try_transfer(
            &self,
            from: &Address,
            to: &Address,
            amount: &i128,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("transfer");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        from.into_val(&self.env),
                        to.into_val(&self.env),
                        amount.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn transfer_from(
            &self,
            spender: &Address,
            from: &Address,
            to: &Address,
            amount: &i128,
        ) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{ soroban_sdk::Symbol::new(&self.env, "transfer_from") },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        spender.into_val(&self.env),
                        from.into_val(&self.env),
                        to.into_val(&self.env),
                        amount.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn try_transfer_from(
            &self,
            spender: &Address,
            from: &Address,
            to: &Address,
            amount: &i128,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{ soroban_sdk::Symbol::new(&self.env, "transfer_from") },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        spender.into_val(&self.env),
                        from.into_val(&self.env),
                        to.into_val(&self.env),
                        amount.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn approve(
            &self,
            owner: &Address,
            spender: &Address,
            amount: &i128,
            live_until_ledger: &u32,
        ) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("approve");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        owner.into_val(&self.env),
                        spender.into_val(&self.env),
                        amount.into_val(&self.env),
                        live_until_ledger.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn try_approve(
            &self,
            owner: &Address,
            spender: &Address,
            amount: &i128,
            live_until_ledger: &u32,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("approve");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        owner.into_val(&self.env),
                        spender.into_val(&self.env),
                        amount.into_val(&self.env),
                        live_until_ledger.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn decimals(&self) -> u32 {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("decimals");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn try_decimals(
            &self,
        ) -> Result<
            Result<
                u32,
                <u32 as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("decimals");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn name(&self) -> String {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("name");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn try_name(
            &self,
        ) -> Result<
            Result<
                String,
                <String as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("name");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn symbol(&self) -> String {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("symbol");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
        pub fn try_symbol(
            &self,
        ) -> Result<
            Result<
                String,
                <String as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error,
            >,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("symbol");
                    SYMBOL
                },
                ::soroban_sdk::Vec::new(&self.env),
            );
            res
        }
    }
    impl ExampleContractArgs {
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn total_supply<'i>() -> () {
            ()
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn balance<'i>(account: &'i Address) -> (&'i Address,) {
            (account,)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn allowance<'i>(
            owner: &'i Address,
            spender: &'i Address,
        ) -> (&'i Address, &'i Address) {
            (owner, spender)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn transfer<'i>(
            from: &'i Address,
            to: &'i Address,
            amount: &'i i128,
        ) -> (&'i Address, &'i Address, &'i i128) {
            (from, to, amount)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn transfer_from<'i>(
            spender: &'i Address,
            from: &'i Address,
            to: &'i Address,
            amount: &'i i128,
        ) -> (&'i Address, &'i Address, &'i Address, &'i i128) {
            (spender, from, to, amount)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn approve<'i>(
            owner: &'i Address,
            spender: &'i Address,
            amount: &'i i128,
            live_until_ledger: &'i u32,
        ) -> (&'i Address, &'i Address, &'i i128, &'i u32) {
            (owner, spender, amount, live_until_ledger)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn decimals<'i>() -> () {
            ()
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn name<'i>() -> () {
            ()
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn symbol<'i>() -> () {
            ()
        }
    }
    #[doc(hidden)]
    pub mod __total_supply {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).total_supply` instead"
        )]
        pub fn invoke_raw(env: soroban_sdk::Env) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::total_supply(&env),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).total_supply` instead"
        )]
        pub extern "C" fn invoke_raw_extern(env: soroban_sdk::Env) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __balance {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).balance` instead")]
        pub fn invoke_raw(env: soroban_sdk::Env, arg_0: soroban_sdk::Val) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::balance(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).balance` instead")]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __allowance {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).allowance` instead"
        )]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::allowance(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).allowance` instead"
        )]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __transfer {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).transfer` instead"
        )]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::transfer(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_2),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).transfer` instead"
        )]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1, arg_2)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __transfer_from {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).transfer_from` instead"
        )]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
            arg_3: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::transfer_from(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_2),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_3),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).transfer_from` instead"
        )]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
            arg_3: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1, arg_2, arg_3)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __approve {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).approve` instead")]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
            arg_3: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::approve(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_2),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_3),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).approve` instead")]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
            arg_3: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1, arg_2, arg_3)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __decimals {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).decimals` instead"
        )]
        pub fn invoke_raw(env: soroban_sdk::Env) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::decimals(&env),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).decimals` instead"
        )]
        pub extern "C" fn invoke_raw_extern(env: soroban_sdk::Env) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __name {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).name` instead")]
        pub fn invoke_raw(env: soroban_sdk::Env) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::name(&env),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).name` instead")]
        pub extern "C" fn invoke_raw_extern(env: soroban_sdk::Env) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __symbol {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).symbol` instead")]
        pub fn invoke_raw(env: soroban_sdk::Env) -> soroban_sdk::Val {
            use super::FungibleToken;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::symbol(&env),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).symbol` instead")]
        pub extern "C" fn invoke_raw_extern(env: soroban_sdk::Env) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env)
        }
        use super::*;
    }
    impl FungibleBurnable for ExampleContract {
        fn burn(e: &Env, from: Address, amount: i128) {
            openzeppelin_pausable::when_not_paused(e);
            {
                fungible::burnable::burn(e, &from, amount)
            }
        }
        fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
            openzeppelin_pausable::when_not_paused(e);
            {
                fungible::burnable::burn_from(e, &spender, &from, amount)
            }
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_BURN: [u8; 60usize] = ExampleContract::spec_xdr_burn();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_burn() -> [u8; 60usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x04burn\0\0\0\x02\0\0\0\0\0\0\0\x04from\0\0\0\x13\0\0\0\0\0\0\0\x06amount\0\0\0\0\0\x0b\0\0\0\0"
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_BURN_FROM: [u8; 88usize] = ExampleContract::spec_xdr_burn_from();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_burn_from() -> [u8; 88usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\tburn_from\0\0\0\0\0\0\x03\0\0\0\0\0\0\0\x07spender\0\0\0\0\x13\0\0\0\0\0\0\0\x04from\0\0\0\x13\0\0\0\0\0\0\0\x06amount\0\0\0\0\0\x0b\0\0\0\0"
        }
    }
    impl<'a> ExampleContractClient<'a> {
        pub fn burn(&self, from: &Address, amount: &i128) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("burn");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [from.into_val(&self.env), amount.into_val(&self.env)],
                ),
            );
            res
        }
        pub fn try_burn(
            &self,
            from: &Address,
            amount: &i128,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("burn");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [from.into_val(&self.env), amount.into_val(&self.env)],
                ),
            );
            res
        }
        pub fn burn_from(&self, spender: &Address, from: &Address, amount: &i128) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("burn_from");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        spender.into_val(&self.env),
                        from.into_val(&self.env),
                        amount.into_val(&self.env),
                    ],
                ),
            );
            res
        }
        pub fn try_burn_from(
            &self,
            spender: &Address,
            from: &Address,
            amount: &i128,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("burn_from");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [
                        spender.into_val(&self.env),
                        from.into_val(&self.env),
                        amount.into_val(&self.env),
                    ],
                ),
            );
            res
        }
    }
    impl ExampleContractArgs {
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn burn<'i>(from: &'i Address, amount: &'i i128) -> (&'i Address, &'i i128) {
            (from, amount)
        }
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn burn_from<'i>(
            spender: &'i Address,
            from: &'i Address,
            amount: &'i i128,
        ) -> (&'i Address, &'i Address, &'i i128) {
            (spender, from, amount)
        }
    }
    #[doc(hidden)]
    pub mod __burn {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).burn` instead")]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleBurnable;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::burn(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).burn` instead")]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1)
        }
        use super::*;
    }
    #[doc(hidden)]
    pub mod __burn_from {
        use super::*;
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).burn_from` instead"
        )]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleBurnable;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::burn_from(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_2),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(
            note = "use `ExampleContractClient::new(&env, &contract_id).burn_from` instead"
        )]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
            arg_2: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1, arg_2)
        }
        use super::*;
    }
    impl FungibleMintable for ExampleContract {
        fn mint(e: &Env, account: Address, amount: i128) {
            openzeppelin_pausable::when_not_paused(e);
            {
                let owner: Address = e
                    .storage()
                    .instance()
                    .get(&OWNER)
                    .expect("owner should be set");
                owner.require_auth();
                fungible::mintable::mint(e, &account, amount);
            }
        }
    }
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #[allow(non_upper_case_globals)]
    pub static __SPEC_XDR_FN_MINT: [u8; 64usize] = ExampleContract::spec_xdr_mint();
    impl ExampleContract {
        #[allow(non_snake_case)]
        pub const fn spec_xdr_mint() -> [u8; 64usize] {
            * b"\0\0\0\0\0\0\0\0\0\0\0\x04mint\0\0\0\x02\0\0\0\0\0\0\0\x07account\0\0\0\0\x13\0\0\0\0\0\0\0\x06amount\0\0\0\0\0\x0b\0\0\0\0"
        }
    }
    impl<'a> ExampleContractClient<'a> {
        pub fn mint(&self, account: &Address, amount: &i128) -> () {
            use core::ops::Not;
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("mint");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [account.into_val(&self.env), amount.into_val(&self.env)],
                ),
            );
            res
        }
        pub fn try_mint(
            &self,
            account: &Address,
            amount: &i128,
        ) -> Result<
            Result<(), <() as soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>::Error>,
            Result<soroban_sdk::Error, soroban_sdk::InvokeError>,
        > {
            use soroban_sdk::{IntoVal, FromVal};
            let res = self.env.try_invoke_contract(
                &self.address,
                &{
                    #[allow(deprecated)]
                    const SYMBOL: soroban_sdk::Symbol = soroban_sdk::Symbol::short("mint");
                    SYMBOL
                },
                ::soroban_sdk::Vec::from_array(
                    &self.env,
                    [account.into_val(&self.env), amount.into_val(&self.env)],
                ),
            );
            res
        }
    }
    impl ExampleContractArgs {
        #[inline(always)]
        #[allow(clippy::unused_unit)]
        pub fn mint<'i>(account: &'i Address, amount: &'i i128) -> (&'i Address, &'i i128) {
            (account, amount)
        }
    }
    #[doc(hidden)]
    pub mod __mint {
        use super::*;
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).mint` instead")]
        pub fn invoke_raw(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            use super::FungibleMintable;
            <_ as soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>::into_val(
                #[allow(deprecated)]
                &<super::ExampleContract>::mint(
                    &env,
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_0),
                    ),
                    <_ as soroban_sdk::unwrap::UnwrapOptimized>::unwrap_optimized(
                        <_ as soroban_sdk::TryFromValForContractFn<
                            soroban_sdk::Env,
                            soroban_sdk::Val,
                        >>::try_from_val_for_contract_fn(&env, &arg_1),
                    ),
                ),
                &env,
            )
        }
        #[deprecated(note = "use `ExampleContractClient::new(&env, &contract_id).mint` instead")]
        pub extern "C" fn invoke_raw_extern(
            env: soroban_sdk::Env,
            arg_0: soroban_sdk::Val,
            arg_1: soroban_sdk::Val,
        ) -> soroban_sdk::Val {
            #[allow(deprecated)]
            invoke_raw(env, arg_0, arg_1)
        }
        use super::*;
    }
    impl soroban_sdk::token::TokenInterface for ExampleContract
    where
        ExampleContract: openzeppelin_fungible_token::FungibleToken
            + openzeppelin_fungible_token::burnable::FungibleBurnable,
    {
        fn balance(env: soroban_sdk::Env, id: soroban_sdk::Address) -> i128 {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::balance(&env, &id)
        }
        fn transfer(
            env: soroban_sdk::Env,
            from: soroban_sdk::Address,
            to: soroban_sdk::Address,
            amount: i128,
        ) {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::transfer(
                &env, &from, &to, &amount,
            )
        }
        fn transfer_from(
            env: soroban_sdk::Env,
            spender: soroban_sdk::Address,
            from: soroban_sdk::Address,
            to: soroban_sdk::Address,
            amount: i128,
        ) {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::transfer_from(
                &env, &spender, &from, &to, &amount,
            )
        }
        fn burn(env: soroban_sdk::Env, from: soroban_sdk::Address, amount: i128) {
            <ExampleContract as openzeppelin_fungible_token::burnable::FungibleBurnable>::burn(
                &env, &from, &amount,
            )
        }
        fn burn_from(
            env: soroban_sdk::Env,
            spender: soroban_sdk::Address,
            from: soroban_sdk::Address,
            amount: i128,
        ) {
            <ExampleContract as openzeppelin_fungible_token::burnable::FungibleBurnable>::burn_from(
                &env, &spender, &from, &amount,
            )
        }
        fn allowance(
            env: soroban_sdk::Env,
            owner: soroban_sdk::Address,
            spender: soroban_sdk::Address,
        ) -> i128 {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::allowance(
                &env, &owner, &spender,
            )
        }
        fn approve(
            env: soroban_sdk::Env,
            owner: soroban_sdk::Address,
            spender: soroban_sdk::Address,
            amount: i128,
            live_until_ledger: u32,
        ) {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::approve(
                &env,
                &owner,
                &spender,
                &amount,
                &live_until_ledger,
            )
        }
        fn decimals(env: soroban_sdk::Env) -> u32 {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::decimals(&env)
        }
        fn name(env: soroban_sdk::Env) -> soroban_sdk::String {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::name(&env)
        }
        fn symbol(env: soroban_sdk::Env) -> soroban_sdk::String {
            <ExampleContract as openzeppelin_fungible_token::FungibleToken>::symbol(&env)
        }
    }
}
mod contract_token_interface {}
