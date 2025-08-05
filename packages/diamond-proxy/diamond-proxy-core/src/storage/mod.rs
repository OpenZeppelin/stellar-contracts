use soroban_sdk::{symbol_short, Address, Env, Symbol};

use crate::Error;

pub struct SharedStorage<'a> {
    env: &'a Env,
}

pub struct PersistentSharedStorage<'a> {
    env: &'a Env,
}

pub struct InstanceSharedStorage<'a> {
    env: &'a Env,
}

pub struct TemporarySharedStorage<'a> {
    env: &'a Env,
}

// Macro to define shared storage methods for different storage types
macro_rules! define_shared_storage_methods {
    ($storage_type:ident, $get_name:expr, $set_name:expr, $del_name:expr) => {
        impl $storage_type<'_> {
            pub fn get<K, V>(&self, key: &K) -> Option<V>
            where
                V::Error: core::fmt::Debug,
                K: soroban_sdk::xdr::ToXdr + Clone,
                V: soroban_sdk::xdr::FromXdr,
            {
                use soroban_sdk::IntoVal;
                let storage = Storage::new(self.env.clone());
                let shared_storage_address = storage.get_shared_storage_address().unwrap();

                // Invoke the diamond contract to get the shared storage value. We must convert to bytes first
                let ret: Option<soroban_sdk::Bytes> = self.env.invoke_contract(
                    &shared_storage_address,
                    &Symbol::new(self.env, $get_name),
                    soroban_sdk::vec![self.env, key.clone().to_xdr(self.env).into_val(self.env),],
                );

                if let Some(bytes) = ret {
                    let repr = V::from_xdr(&self.env, &bytes);
                    if repr.is_err() {
                        self.env.panic_with_error(Error::InvalidXdrSerialization);
                    }

                    repr.ok()
                } else {
                    None
                }
            }

            pub fn set<K, V>(&self, key: &K, value: &V) -> Result<(), Error>
            where
                K: soroban_sdk::xdr::ToXdr + Clone,
                V: soroban_sdk::xdr::ToXdr + Clone,
            {
                use soroban_sdk::IntoVal;
                let storage = Storage::new(self.env.clone());
                let shared_storage_address = storage.get_shared_storage_address().unwrap();

                // Invoke the diamond contract to set the shared storage value
                let res: Result<(), Error> = self.env.invoke_contract(
                    &shared_storage_address,
                    &Symbol::new(self.env, $set_name),
                    soroban_sdk::vec![
                        self.env,
                        key.clone().to_xdr(self.env).into_val(self.env),
                        value.clone().to_xdr(self.env).into_val(self.env),
                    ],
                );

                res
            }

            pub fn delete<K>(&self, key: &K) -> Result<(), Error>
            where
                K: soroban_sdk::xdr::ToXdr + Clone,
            {
                use soroban_sdk::IntoVal;
                let storage = Storage::new(self.env.clone());
                let shared_storage_address = storage.get_shared_storage_address().unwrap();

                // Invoke the diamond contract to delete the shared storage value
                let res: Result<(), Error> = self.env.invoke_contract(
                    &shared_storage_address,
                    &Symbol::new(self.env, $del_name),
                    soroban_sdk::vec![self.env, key.clone().to_xdr(self.env).into_val(self.env),],
                );

                res
            }
        }
    };
}

// Apply the macro to each shared storage type
define_shared_storage_methods!(
    InstanceSharedStorage,
    "get_instance_shared_storage_at",
    "set_instance_shared_storage_at",
    "del_instance_shared_storage_at"
);
define_shared_storage_methods!(
    PersistentSharedStorage,
    "get_persistent_shared_storage_at",
    "set_persistent_shared_storage_at",
    "del_persistent_shared_storage_at"
);
define_shared_storage_methods!(
    TemporarySharedStorage,
    "get_temporary_shared_storage_at",
    "set_temporary_shared_storage_at",
    "del_temporary_shared_storage_at"
);

pub trait SharedStorageImpl {
    /// Returns the shared storage handle provided it is initialized. The handle proxies
    /// initialized requests to the shared storage contract passed into a facet demarcated
    /// by #[facet]. Since the shared storage contract must be invoked across a network boundary,
    /// keys and values must implement `soroban_sdk::xdr::ToXdr` and `soroban_sdk::xdr::FromXdr`
    /// to allow serialization.
    ///
    /// # Panics
    /// Panics if the shared storage is not initialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn get_env() -> Env { unimplemented!() }
    /// # fn deploy_diamond() -> Address { unimplemented!() }
    /// use soroban_sdk::{Env, Symbol, vec, Address};
    /// use stellar_diamond_proxy_core::storage::SharedStorageImpl;
    /// use crate::stellar_diamond_proxy_core::utils::DiamondProxyExecutor;
    ///
    /// mod first_facet {
    ///     use soroban_sdk::{Env, Symbol, vec};
    ///     use stellar_facet_macro::facet;
    ///     use stellar_diamond_proxy_core::storage::SharedStorageImpl;
    ///     use stellar_diamond_proxy_core::utils::DiamondProxyExecutor;
    ///
    ///     #[soroban_sdk::contract]
    ///     struct MyFirstFacet;
    ///
    ///     #[soroban_sdk::contractimpl]
    ///     impl MyFirstFacet {
    ///         fn add_one(env: Env) {
    ///             let key = Symbol::new(&env, "counter");
    ///             let storage = env.shared_storage();
    ///             let instance_shared_storage = storage.instance();
    ///             let current_value = instance_shared_storage.get::<_, u32>(&key).unwrap_or(0);
    ///             let new_value = current_value.saturating_add(1);
    ///             instance_shared_storage.set(&key, &new_value);
    ///         }
    ///     }
    /// }
    ///
    /// mod second_facet {
    ///     use soroban_sdk::{Env, Symbol, vec};
    ///     use stellar_facet_macro::facet;
    ///     use stellar_diamond_proxy_core::storage::SharedStorageImpl;
    ///     use stellar_diamond_proxy_core::utils::DiamondProxyExecutor;
    ///
    ///     #[soroban_sdk::contract]
    ///     struct MySecondFacet;
    ///
    ///     #[soroban_sdk::contractimpl]
    ///     impl MySecondFacet {
    ///         fn add_two(env: Env) {
    ///             let key = Symbol::new(&env, "counter");
    ///             let storage = env.shared_storage();
    ///             let instance_shared_storage = storage.instance();
    ///             let current_value = instance_shared_storage.get::<_, u32>(&key).unwrap_or(0);
    ///             let new_value = current_value.saturating_add(2);
    ///             instance_shared_storage.set(&key, &new_value);
    ///         }
    ///     }
    /// }
    ///
    /// // Assuming the diamond proxy is initialized and the facets are deployed via diamond_cut,
    /// // you can now call in your application: add_one, add_two, and get_counter
    /// let env = get_env();
    ///
    /// let diamond_proxy_address = &deploy_diamond();
    ///
    /// env.facet_execute::<()>(diamond_proxy_address, &Symbol::new(&env, "add_one"), vec![&env]).unwrap();
    /// let current = env.facet_execute::<u32>(diamond_proxy_address, &Symbol::new(&env, "get_counter"), vec![&env]).unwrap();
    /// assert_eq!(current, 1);
    /// env.facet_execute::<()>(diamond_proxy_address, &Symbol::new(&env, "add_two"), vec![&env]).unwrap();
    /// let current = env.facet_execute::<u32>(diamond_proxy_address, &Symbol::new(&env, "get_counter"), vec![&env]).unwrap();
    /// assert_eq!(current, 3);
    ///
    /// // In general, you may use the shared storage similar to accessing ordinary storage:
    /// let shared_storage = env.shared_storage();
    /// let instance_shared_storage = shared_storage.instance();
    /// let persistent_shared_storage = shared_storage.persistent();
    /// let temporary_shared_storage = shared_storage.temporary();
    ///
    /// // Define some key that is will be referenced by each facet
    /// let key = soroban_sdk::Symbol::new(&env, "counter");
    /// let value = 100u32;
    ///
    /// // Call get on each storage
    /// instance_shared_storage.get::<_, u32>(&key);
    /// persistent_shared_storage.get::<_, u32>(&key);
    /// temporary_shared_storage.get::<_, u32>(&key);
    ///
    /// // Call set on each storage
    /// instance_shared_storage.set(&key, &value);
    /// persistent_shared_storage.set(&key, &value);
    /// temporary_shared_storage.set(&key, &value);
    ///
    /// // Call delete on each storage
    /// instance_shared_storage.delete(&key);
    /// persistent_shared_storage.delete(&key);
    /// temporary_shared_storage.delete(&key);
    /// ```
    fn shared_storage(&self) -> SharedStorage<'_>;
}

impl SharedStorageImpl for Env {
    fn shared_storage(&self) -> SharedStorage<'_> {
        if Storage::new(self.clone())
            .get_shared_storage_address()
            .is_some()
        {
            SharedStorage { env: self }
        } else {
            self.panic_with_error(Error::SharedStorageNotInitialized)
        }
    }
}

impl SharedStorage<'_> {
    /// Shared storage for data that can stay in the ledger forever until deleted.
    ///
    /// Persistent entries might expire and be removed from the ledger if they run out
    /// of the rent balance. However, expired entries can be restored and
    /// they cannot be recreated. This means these entries
    /// behave 'as if' they were stored in the ledger forever.
    ///
    /// This should be used for data that requires persistency, such as token
    /// balances, user properties etc.
    pub fn persistent(&self) -> PersistentSharedStorage {
        PersistentSharedStorage { env: self.env }
    }

    /// Shared storage for a **small amount** of persistent data associated with
    /// the current contract's instance.
    ///
    /// Storing a small amount of frequently used data in instance storage is
    /// likely cheaper than storing it separately in Persistent storage.
    ///
    /// Instance storage is tightly coupled with the contract instance: it will
    /// be loaded from the ledger every time the contract instance itself is
    /// loaded. It also won't appear in the ledger footprint. *All*
    /// the data stored in the instance storage is read from ledger every time
    /// the contract is used and it doesn't matter whether contract uses the
    /// storage or not.
    ///
    /// This has the same lifetime properties as Persistent storage, i.e.
    /// the data semantically stays in the ledger forever and can be
    /// expired/restored.
    ///
    /// The amount of data that can be stored in the instance storage is limited
    /// by the ledger entry size (a network-defined parameter). It is
    /// in the order of 100 KB serialized.
    ///
    /// This should be used for small data directly associated with the current
    /// contract, such as its admin, configuration settings, tokens the contract
    /// operates on etc. Do not use this with any data that can scale in
    /// unbounded fashion (such as user balances).
    pub fn instance(&self) -> InstanceSharedStorage {
        InstanceSharedStorage { env: self.env }
    }

    /// Shared storage for data that may stay in ledger only for a limited amount of
    /// time.
    ///
    /// Temporary storage is cheaper than Persistent storage.
    ///
    /// Temporary entries will be removed from the ledger after their lifetime
    /// ends. Removed entries can be created again, potentially with different
    /// values.
    ///
    /// This should be used for data that needs to only exist for a limited
    /// period of time, such as oracle data, claimable balances, offer, etc.    
    pub fn temporary(&self) -> TemporarySharedStorage {
        TemporarySharedStorage { env: self.env }
    }
}

const OWNER: Symbol = symbol_short!("owner");
const INITIALIZED: Symbol = symbol_short!("init");
const SHARED_STORAGE_ADDRESS: Symbol = symbol_short!("__shaddr");
const DIAMOND_PROXY_ADDRESS: Symbol = symbol_short!("__dmaddr");

pub struct Storage {
    env: Env,
}

impl Storage {
    pub fn new(env: Env) -> Self {
        Self { env }
    }

    // Core storage functionality
    pub fn get_owner(&self) -> Option<Address> {
        self.env.storage().persistent().get(&OWNER)
    }

    pub fn set_owner(&self, owner: &Address) {
        self.env.storage().persistent().set(&OWNER, owner);
    }

    /// Set the shared storage address in storage. Used only by facets
    pub fn set_shared_storage_address(&self, address: &Address) {
        self.env
            .storage()
            .persistent()
            .set(&SHARED_STORAGE_ADDRESS, address);
    }

    /// Get the shared storage address from storage. Used only by facets
    pub fn get_shared_storage_address(&self) -> Option<Address> {
        self.env.storage().persistent().get(&SHARED_STORAGE_ADDRESS)
    }

    /// Set the diamond proxy address in storage. Used for security validation
    pub fn set_diamond_proxy_address(&self, address: &Address) {
        self.env
            .storage()
            .persistent()
            .set(&DIAMOND_PROXY_ADDRESS, address);
    }

    /// Get the diamond proxy address from storage. Used for security validation
    pub fn get_diamond_proxy_address(&self) -> Option<Address> {
        self.env.storage().persistent().get(&DIAMOND_PROXY_ADDRESS)
    }

    pub fn is_initialized(&self) -> bool {
        self.env
            .storage()
            .persistent()
            .get(&INITIALIZED)
            .unwrap_or(false)
    }

    pub fn set_initialized(&self) {
        self.env.storage().persistent().set(&INITIALIZED, &true);
    }

    pub fn require_uninitialized(&self) {
        if self.is_initialized() {
            panic!("Error: {:?}", Error::AlreadyInitialized)
        }
    }
}
