//! # PolkaVM Pallet
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet. It is typically used in beginner tutorials or in Substrate template
//! nodes as a starting point for creating a new pallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pallet contains basic examples of:
//! - declaring a storage item that stores a single `u32` value
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! Learn more about FRAME macros [here](https://docs.substrate.io/reference/frame-macros/).
//!
//! ### Pallet Sections
//!
//! The pallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

mod polkavm;

pub mod weights;
pub use weights::*;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
    // Import various useful types required by all FRAME pallets.
    use super::*;
    use frame_support::{
        dispatch::PostDispatchInfo,
        pallet_prelude::*,
        traits::{
            fungible::{Inspect, Mutate},
            tokens::Preservation,
        },
    };
    use frame_system::pallet_prelude::*;
    use scale_info::{TypeInfo, prelude::vec::Vec};
    use sp_runtime::traits::{Hash, SaturatedConversion, TrailingZeroInput};

    use polkavm::{
        CallError, Caller, Config as PolkaVMConfig, Engine, GasMeteringKind, Instance, Linker,
        Module as PolkaVMModule, ModuleConfig as PolkaVMModuleConfig, ProgramBlob, State,
    };

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
    type CodeHash<T> = <T as frame_system::Config>::Hash;
    type CodeVec<T> = BoundedVec<u8, <T as Config>::MaxCodeLen>;
    type CodeStorageSlot<T> = BoundedVec<u8, <T as Config>::StorageSize>;
    pub type StorageKey<T> = BoundedVec<u8, <T as Config>::MaxStorageKeySize>;

    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub(super) struct BlobMetadata<T: Config> {
        owner: T::AccountId,
        version: u64,
    }

    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
    pub(super) struct ExecResult {
        result: Option<u64>,
        not_enough_gas: bool,
        trap: bool,
        gas_before: u32,
        gas_after: i64,
    }

    // The `Pallet` struct serves as a placeholder to implement traits, methods and dispatchables
    // (`Call`s) in this pallet.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    ///
    /// All our types and constants a pallet depends on must be declared here.
    /// These types are defined generically and made concrete when the pallet is declared in the
    /// `runtime/src/lib.rs` file of your chain.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The maximum length of a contract code in bytes.
        ///
        /// The value should be chosen carefully taking into the account the overall memory limit
        /// your runtime has, as well as the [maximum allowed callstack
        /// depth](#associatedtype.CallStack). Look into the `integrity_test()` for some insights.
        #[pallet::constant]
        type MaxCodeLen: Get<u32>;

        #[pallet::constant]
        type MaxGasLimit: Get<u32>;

        #[pallet::constant]
        type MaxStorageSlots: Get<u32>;

        #[pallet::constant]
        type MaxStorageKeySize: Get<u32>;

        #[pallet::constant]
        type MinGasPrice: Get<u64>;

        #[pallet::constant]
        type StorageSize: Get<u32>;

        #[pallet::constant]
        type StorageSlotPrice: Get<u128>;

        /// The fungible
        type Currency: Inspect<Self::AccountId> + Mutate<Self::AccountId>;

        /// A type representing the weights required by the dispatchables of this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    pub(super) type Code<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, CodeVec<T>>;

    #[pallet::storage]
    pub(super) type ExecutionResult<T: Config> =
        StorageMap<_, Blake2_128Concat, (T::AccountId, T::AccountId), ExecResult>;

    #[pallet::storage]
    pub(super) type CodeMetadata<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlobMetadata<T>>;

    #[pallet::storage]
    pub(super) type CodeAddress<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId>;

    #[pallet::storage]
    pub(super) type CodeStorage<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (T::AccountId, T::AccountId, StorageKey<T>),
        CodeStorageSlot<T>,
    >;

    /// Events that functions in this pallet can emit.
    ///
    /// Events are a simple means of indicating to the outside world (such as dApps, chain explorers
    /// or other users) that some notable update in the runtime has occurred. In a FRAME pallet, the
    /// documentation for each event field and its parameters is added to a node's metadata so it
    /// can be used by external interfaces or tools.
    ///
    ///	The `generate_deposit` macro generates a function on `Pallet` called `deposit_event` which
    /// will convert the event type of your pallet into `RuntimeEvent` (declared in the pallet's
    /// [`Config`] trait) and deposit it using [`frame_system::Pallet::deposit_event`].
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A user has successfully set a new value.
        ExecutionResult {
            /// The account who set the new value.
            who: T::AccountId,
            // The smart contract account.
            contract_address: T::AccountId,
            /// The new value set.
            result: Option<u64>,
            not_enough_gas: bool,
            trap: bool,
            gas_before: u32,
            gas_after: i64,
        },
        ProgramBlobUploaded {
            /// The account who uploaded ProgramBlob.
            who: T::AccountId,
            // The smart contract account.
            contract_address: T::AccountId,
            // List of function in the smart contract.
            exports: Vec<Vec<u8>>,
        },
    }

    /// Errors that can be returned by this pallet.
    ///
    /// Errors tell users that something went wrong so it's important that their naming is
    /// informative. Similar to events, error documentation is added to a node's metadata so it's
    /// equally important that they have helpful documentation associated with them.
    ///
    /// This type of runtime error can be up to 4 bytes in size should you want to return additional
    /// information.
    #[pallet::error]
    pub enum Error<T> {
        IntegerOverflow,
        ProgramBlobNotFound,
        InvalidOperation,
        InvalidOperands,

        // PolkaVM errors
        ProgramBlobTooLarge,
        ProgramBlobParsingFailed,
        PolkaVMConfigurationFailed,
        PolkaVMEngineCreationFailed,
        PolkaVMModuleCreationFailed,
        HostFunctionDefinitionFailed,
        PolkaVMModuleExecutionFailed,
        PolkaVMModuleInstantiationFailed,
        PolkaVMModulePreInstantiationFailed,
        PolkaVMNotEnoughGas,
        PolkaVMTrap,
        GasLimitIsTooHigh,
        GasPriceIsTooLow,

        /// Performing the requested transfer failed. Probably because there isn't enough
        /// free balance in the sender's account.
        TransferFailed,
    }

    /// The pallet's dispatchable functions ([`Call`]s).
    ///
    /// Dispatchable functions allows users to interact with the pallet and invoke state changes.
    /// These functions materialize as "extrinsics", which are often compared to transactions.
    /// They must always return a `DispatchResult` and be annotated with a weight and call index.
    ///
    /// The [`call_index`] macro is used to explicitly
    /// define an index for calls in the [`Call`] enum. This is useful for pallets that may
    /// introduce new dispatchables over time. If the order of a dispatchable changes, its index
    /// will also change which will break backwards compatibility.
    ///
    /// The [`weight`] macro is used to assign a weight to each call.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::upload())]
        pub fn upload(origin: OriginFor<T>, mut program_blob: Vec<u8>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            let max_len = <T as Config>::MaxCodeLen::get()
                .try_into()
                .map_err(|_| Error::<T>::IntegerOverflow)?;
            let mut raw_blob = BoundedVec::with_bounded_capacity(max_len);
            raw_blob
                .try_append(&mut program_blob)
                .map_err(|_| Error::<T>::ProgramBlobTooLarge)?;

            let module = Self::prepare(raw_blob[..].into())?;
            let exports = module
                .exports()
                .map(|export| export.symbol().clone().into_inner().to_vec())
                .collect();

            let mut blob_metadata = match CodeMetadata::<T>::get(&who) {
                Some(meta) => meta,
                None => BlobMetadata {
                    owner: who.clone(),
                    version: 0,
                },
            };
            let old_contract_address =
                Self::contract_address(&who, &T::Hashing::hash_of(&blob_metadata));
            let old_version = blob_metadata.version;
            blob_metadata.version = blob_metadata
                .version
                .checked_add(1)
                .ok_or(Error::<T>::IntegerOverflow)?;
            let contract_address =
                Self::contract_address(&who, &T::Hashing::hash_of(&blob_metadata));

            if old_version != 0 {
                Code::<T>::remove(old_contract_address)
            }
            Code::<T>::insert(&contract_address, &raw_blob);
            CodeAddress::<T>::insert(&who, &contract_address);
            CodeMetadata::<T>::insert(&who, blob_metadata);

            Self::deposit_event(Event::ProgramBlobUploaded {
                who,
                contract_address,
                exports,
            });

            Ok(())
        }

        /// An example dispatchable that takes a single u32 value as a parameter, writes the value
        /// to storage and emits an event.
        ///
        /// It checks that the _origin_ for this call is _Signed_ and returns a dispatch
        /// error if it isn't. Learn more about origins here: <https://docs.substrate.io/build/origins/>
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::execute())]
        pub fn execute(
            origin: OriginFor<T>,
            contract_address: T::AccountId,
            to: T::AccountId,
            value: BalanceOf<T>,
            op: u32,
            gas_limit: u32,
            gas_price: u64,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            ensure!(op <= 5, Error::<T>::InvalidOperation);

            let max_gas_limit = <T as Config>::MaxGasLimit::get()
                .try_into()
                .map_err(|_| Error::<T>::IntegerOverflow)?;
            ensure!(gas_limit <= max_gas_limit, Error::<T>::GasLimitIsTooHigh);

            let max_storage_size = <T as Config>::StorageSize::get()
                .try_into()
                .map_err(|_| Error::<T>::IntegerOverflow)?;

            let max_storage_key_size = <T as Config>::MaxStorageKeySize::get()
                .try_into()
                .map_err(|_| Error::<T>::IntegerOverflow)?;

            let max_storage_slot_idx = <T as Config>::StorageSize::get()
                .checked_sub(1)
                .ok_or(Error::<T>::IntegerOverflow)?;

            ensure!(gas_price >= <T as Config>::MinGasPrice::get(), Error::<T>::GasPriceIsTooLow);

            let raw_blob = Code::<T>::get(&contract_address)
                .ok_or(Error::<T>::ProgramBlobNotFound)?
                .into_inner();

            let mut instance = Self::instantiate(Self::prepare(raw_blob)?)?;
            instance.set_gas(gas_limit.into());

            let mut state = State::new(
                [contract_address.clone(), who.clone(), to].to_vec(),
                [value].to_vec(),
                [
                    104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33, 33, 33,
                ]
                .to_vec(),
                max_storage_size,
                max_storage_key_size,
                max_storage_slot_idx,
                |from: T::AccountId, to: T::AccountId, value: BalanceOf<T>| -> u64 {
                    if !value.is_zero() && from != to {
                        if let Err(_) =
                            T::Currency::transfer(&from, &to, value, Preservation::Preserve)
                        {
                            return 1;
                        }
                    }
                    0
                },
                |m: Vec<u8>| -> u64 {
                    sp_runtime::print(&*m);
                    return 0;
                },
                |address: T::AccountId| -> u64 { T::Currency::balance(&address).saturated_into() },
                |address: T::AccountId| -> u64 { T::Currency::balance(&address).saturated_into() },
                || -> u64 { frame_system::Pallet::<T>::block_number().saturated_into() },
                || -> u64 { 0 },
                || -> u64 { 1 },
                |contract_address: T::AccountId,
                 caller_address: T::AccountId,
                 key: StorageKey<T>|
                 -> Option<Vec<u8>> {
                    CodeStorage::<T>::get((contract_address, caller_address, key))
                        .map(|d| d.to_vec())
                },
                |contract_address: T::AccountId,
                 caller_address: T::AccountId,
                 key: StorageKey<T>,
                 max_storage_size: usize,
                 mut data: Vec<u8>|
                 -> u64 {
                    let mut buffer = BoundedVec::with_bounded_capacity(max_storage_size);
                    if let Ok(_) = buffer.try_append(&mut data) {
                        CodeStorage::<T>::insert((contract_address, caller_address, key), buffer);
                        0
                    } else {
                        1
                    }
                },
            );

            sp_runtime::print("====== BEFORE CALL ======");

            let result =
                instance.call_typed_and_get_result::<u64, (u32,)>(&mut state, "main", (op,));

            let (result, not_enough_gas, trap) = match result {
                Err(CallError::NotEnoughGas) => (None, true, false),
                Err(CallError::Trap) => (None, false, true),
                Err(_) => Err(Error::<T>::PolkaVMModuleExecutionFailed)?,
                Ok(res) => (Some(res), false, false),
            };

            sp_runtime::print("====== AFTER CALL ======");

            ExecutionResult::<T>::insert(
                (&contract_address, &who),
                ExecResult {
                    result,
                    not_enough_gas,
                    trap,
                    gas_before: gas_limit,
                    gas_after: instance.gas(),
                },
            );

            // Emit an event.
            Self::deposit_event(Event::ExecutionResult {
                who,
                contract_address,
                result,
                not_enough_gas,
                trap,
                gas_before: gas_limit,
                gas_after: instance.gas(),
            });

            let normalized_gas_after = if instance.gas() < 0 {
                0u64
            } else {
                instance.gas() as u64
            };

            Ok(PostDispatchInfo {
                actual_weight: Some(Weight::from_all(
                    (u64::from(gas_limit) - normalized_gas_after) * gas_price,
                )),
                pays_fee: Pays::Yes,
            })
        }
    }

    trait ModuleLoader {
        type T: Config;

        fn prepare(raw_blob: Vec<u8>) -> Result<PolkaVMModule, DispatchError>;
        fn instantiate(module: PolkaVMModule) -> Result<Instance<Self::T>, DispatchError>;
    }

    impl<T: Config> ModuleLoader for Pallet<T> {
        type T = T;

        fn prepare(raw_blob: Vec<u8>) -> Result<PolkaVMModule, DispatchError> {
            let blob = ProgramBlob::parse(raw_blob[..].into())
                .map_err(|_| Error::<T>::ProgramBlobParsingFailed)?;

            let config =
                PolkaVMConfig::from_env().map_err(|_| Error::<T>::PolkaVMConfigurationFailed)?;
            let engine =
                Engine::new(&config).map_err(|_| Error::<T>::PolkaVMEngineCreationFailed)?;

            let mut module_config = PolkaVMModuleConfig::new();
            module_config.set_gas_metering(Some(GasMeteringKind::Sync));
            let module = PolkaVMModule::from_blob(&engine, &module_config, blob)
                .map_err(|_| Error::<T>::PolkaVMModuleCreationFailed)?;

            Ok(module)
        }

        fn instantiate(module: PolkaVMModule) -> Result<Instance<T>, DispatchError> {
            // High-level API.
            let mut linker: Linker<T> = Linker::<T>::new();

            linker
                .define_typed(
                    "transfer",
                    |caller: Caller<T>, address_idx: u32, balance_idx: u32| -> u64 {
                        (caller.user_data.transfer)(
                            caller.user_data.addresses[0].clone(),
                            caller.user_data.addresses[address_idx as usize].clone(),
                            caller.user_data.balances[balance_idx as usize].clone(),
                        )
                    },
                )
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("balance", |caller: Caller<T>| -> u64 {
                    (caller.user_data.balance)(caller.user_data.addresses[0].clone())
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("balance_of", |caller: Caller<T>| -> u64 {
                    (caller.user_data.balance)(caller.user_data.addresses[2].clone())
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("print", |caller: Caller<T>| -> u64 {
                    (caller.user_data.print)(caller.user_data.log_message.clone())
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("block_number", |caller: Caller<T>| -> u64 {
                    (caller.user_data.block_number)()
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("account_id", |caller: Caller<T>| -> u64 {
                    (caller.user_data.account_id)()
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("caller", |caller: Caller<T>| -> u64 {
                    (caller.user_data.caller)()
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed("storage_size", |caller: Caller<T>| -> u64 {
                    caller.user_data.max_storage_size as u64
                })
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed(
                    "get",
                    |caller: Caller<T>, storage_key_pointer: u32, pointer: u32| -> u64 {
                        if let Ok(mut raw_storage_key) = caller
                            .instance
                            .read_memory(storage_key_pointer, caller.user_data.max_storage_key_size)
                        {
                            let mut storage_key = BoundedVec::with_bounded_capacity(
                                caller.user_data.max_storage_key_size as usize,
                            );
                            match storage_key.try_append(&mut raw_storage_key) {
                                Ok(_) => (),
                                Err(_) => return 1,
                            };

                            let result = (caller.user_data.get)(
                                caller.user_data.addresses[0].clone(),
                                caller.user_data.addresses[1].clone(),
                                storage_key,
                            );
                            if let Some(chunk) = result {
                                match caller.instance.write_memory(pointer, &chunk) {
                                    Err(_) => return 1,
                                    Ok(_) => return 0,
                                }
                            }
                            return 0;
                        } else {
                            return 1;
                        }
                    },
                )
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            linker
                .define_typed(
                    "set",
                    |caller: Caller<T>, storage_key_pointer: u32, buffer: u32| -> u64 {
                        if let Ok(mut raw_storage_key) = caller
                            .instance
                            .read_memory(storage_key_pointer, caller.user_data.max_storage_key_size)
                        {
                            if let Ok(data) = caller
                                .instance
                                .read_memory(buffer, caller.user_data.max_storage_size as u32)
                            {
                                let mut storage_key = BoundedVec::with_bounded_capacity(
                                    caller.user_data.max_storage_key_size as usize,
                                );
                                match storage_key.try_append(&mut raw_storage_key) {
                                    Ok(_) => (),
                                    Err(_) => return 1,
                                };

                                (caller.user_data.insert)(
                                    caller.user_data.addresses[0].clone(),
                                    caller.user_data.addresses[1].clone(),
                                    storage_key,
                                    caller.user_data.max_storage_size,
                                    data,
                                );
                                return 0;
                            } else {
                                return 1;
                            }
                        } else {
                            1
                        }
                    },
                )
                .map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

            // Link the host functions with the module.
            let instance_pre = linker
                .instantiate_pre(&module)
                .map_err(|_| Error::<T>::PolkaVMModulePreInstantiationFailed)?;

            // Instantiate the module.
            let instance = instance_pre
                .instantiate()
                .map_err(|_| Error::<T>::PolkaVMModuleInstantiationFailed)?;

            Ok(instance)
        }
    }

    pub trait AddressGenerator<T: Config> {
        /// The address of a contract based on the given instantiate parameters.
        ///
        /// Changing the formular for an already deployed chain is fine as long as no collisions
        /// with the old formular. Changes only affect existing contracts.
        fn contract_address(
            deploying_address: &T::AccountId,
            code_hash: &CodeHash<T>,
        ) -> T::AccountId;
    }

    impl<T: Config> AddressGenerator<T> for Pallet<T> {
        /// Formula: `hash("contract_addr_v1" ++ deploying_address ++ code_hash)`
        fn contract_address(
            deploying_address: &T::AccountId,
            code_hash: &CodeHash<T>,
        ) -> T::AccountId {
            let entropy =
                (b"contract_addr_v1", deploying_address, code_hash).using_encoded(T::Hashing::hash);
            Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
                .expect("infinite length input; no invalid inputs for type; qed")
        }
    }
}
