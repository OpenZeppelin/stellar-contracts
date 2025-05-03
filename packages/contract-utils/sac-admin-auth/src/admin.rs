use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    token::StellarAssetClient,
    Address, BytesN, Env, Symbol, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    UnknownSigner = 1,
    InvalidContext = 2,
    InvalidFn = 3,
    Unauthorized = 4,
}

#[contracttype]
#[derive(Clone)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
pub enum SacDataKey {
    SAC,
    Owner,
}

#[contract]
pub struct SacAdmin;

#[contractimpl]
impl SacAdmin {
    pub fn __constructor(e: Env, sac: Address, owner: Address) {
        e.storage().instance().set(&SacDataKey::SAC, &sac);
        e.storage().instance().set(&SacDataKey::Owner, &owner);
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        // access control: only owner can call mint
        let owner: Address = e.storage().instance().get(&SacDataKey::Owner).unwrap();
        owner.require_auth();

        // mint on sac
        let sac_address: Address = e.storage().instance().get(&SacDataKey::SAC).unwrap();
        let token_client = StellarAssetClient::new(&e, &sac_address);
        token_client.mint(&to, &amount);
    }

    pub fn set_admin(_e: Env, _new_admin: Address) {
        todo!()
    }

    pub fn set_authorized(_e: Env, _id: Address, _authorize: bool) {
        todo!()
    }

    pub fn clawback(_e: Env, _from: Address, _amount: i128) {
        todo!()
    }
}

#[contractimpl]
impl CustomAccountInterface for SacAdmin {
    type Signature = Signature;
    type Error = Error;

    // This is the 'entry point' of the account contract and every account
    // contract has to implement it. `require_auth` calls for the Address of
    // this contract will result in calling this `__check_auth` function with
    // the appropriate arguments.
    #[allow(non_snake_case)]
    fn __check_auth(
        e: Env,
        _signature_payload: Hash<32>,
        _signature: Self::Signature,
        auth_context: Vec<Context>,
    ) -> Result<(), Error> {
        // TODO: authenticate

        for context in auth_context.iter() {
            let contract_context = match context {
                Context::Contract(c) => c,
                _ => return Err(Error::InvalidContext),
            };

            let sac_address: Address = e.storage().instance().get(&SacDataKey::SAC).unwrap();
            if contract_context.contract != sac_address {
                return Err(Error::InvalidContext);
            }

            if contract_context.fn_name == Symbol::new(&e, "mint") {
                // pub_key is BytesN !
                // ensure_has_role("minter", sig.pub_key)
                let _amount = contract_context.args.get(1).unwrap();
                // ensure_has_limit(amount, sig.pub_key)
            } else if contract_context.fn_name == Symbol::new(&e, "clawback") {
                // ensure_has_role("clawback", signature.pub_key)
            } else {
                return Err(Error::Unauthorized);
            }
        }

        Ok(())
    }
}
