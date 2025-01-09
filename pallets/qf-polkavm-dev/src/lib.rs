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

pub mod weights;
pub use weights::*;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
    // Import various useful types required by all FRAME pallets.
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use scale_info::{TypeInfo, prelude::vec::Vec};
    use sp_runtime::traits::Hash;

    use polkavm::{
        Config as PolkaVMConfig, Engine, Instance, Linker, Module as PolkaVMModule, ProgramBlob,
    };

    type CodeHash<T> = <T as frame_system::Config>::Hash;
    type CodeVec<T> = BoundedVec<u8, <T as Config>::MaxCodeLen>;

    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub(super) struct BlobMetadata<T: Config> {
        owner: T::AccountId,
        version: u64,
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
        /// A type representing the weights required by the dispatchables of this pallet.
        type WeightInfo: WeightInfo;

        /// The maximum length of a contract code in bytes.
        ///
        /// The value should be chosen carefully taking into the account the overall memory limit
        /// your runtime has, as well as the [maximum allowed callstack
        /// depth](#associatedtype.CallStack). Look into the `integrity_test()` for some insights.
        #[pallet::constant]
        type MaxCodeLen: Get<u32>;
    }

    #[pallet::storage]
    pub(super) type Code<T: Config> = StorageMap<_, Blake2_128Concat, CodeHash<T>, CodeVec<T>>;

    #[pallet::storage]
    pub(super) type CalculationResult<T: Config> =
        StorageMap<_, Blake2_128Concat, (CodeHash<T>, T::AccountId), u32>;

    #[pallet::storage]
    pub(super) type CodeMetadata<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlobMetadata<T>>;

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
        Calculated {
            /// The account who set the new value.
            who: T::AccountId,
            address: CodeHash<T>,
            /// The new value set.
            result: u32,
        },
        ProgramBlobUploaded {
            /// The account who uploaded ProgramBlob.
            who: T::AccountId,
            address: CodeHash<T>,
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
            if blob_metadata.version != 0 {
                Code::<T>::remove(T::Hashing::hash_of(&blob_metadata))
            }
            blob_metadata.version = blob_metadata
                .version
                .checked_add(1)
                .ok_or(Error::<T>::IntegerOverflow)?;
            let address = T::Hashing::hash_of(&blob_metadata);

            Code::<T>::insert(address, &raw_blob);
            CodeMetadata::<T>::insert(&who, blob_metadata);

            Self::deposit_event(Event::ProgramBlobUploaded {
                who,
                address,
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
            blob_address: CodeHash<T>,
            a: u32,
            b: u32,
            op: u8,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            let raw_blob = Code::<T>::get(blob_address)
                .ok_or(Error::<T>::ProgramBlobNotFound)?
                .into_inner();

            let result = match op {
                0 => Self::sum(a, b, raw_blob)?,
                1 => Self::sub(a, b, raw_blob)?,
                2 => Self::mul(a, b, raw_blob)?,
                _ => Err(Error::<T>::InvalidOperation)?,
            };

            CalculationResult::<T>::insert((&blob_address, &who), result);

            // Emit an event.
            Self::deposit_event(Event::Calculated {
                who,
                address: blob_address,
                result,
            });

            // Return a successful `DispatchResult`
            Ok(())
        }
    }

    trait Calculator {
        fn sum(a: u32, b: u32, raw_blob: Vec<u8>) -> Result<u32, DispatchError>;
        fn sub(a: u32, b: u32, raw_blob: Vec<u8>) -> Result<u32, DispatchError>;
        fn mul(a: u32, b: u32, raw_blob: Vec<u8>) -> Result<u32, DispatchError>;
    }

    impl<T: Config> Calculator for Pallet<T> {
        fn sum(a: u32, b: u32, raw_blob: Vec<u8>) -> Result<u32, DispatchError> {
            a.checked_add(b).ok_or(Error::<T>::InvalidOperands)?;

            // Grab the function and call it.
            let result = Self::instantiate(Self::prepare(raw_blob)?)?
                .call_typed_and_get_result::<u32, (u32, u32)>(&mut (), "add_numbers", (a, b))
                .map_err(|_| Error::<T>::PolkaVMModuleExecutionFailed)?;

            Ok(result)
        }

        fn sub(a: u32, b: u32, raw_blob: Vec<u8>) -> Result<u32, DispatchError> {
            a.checked_sub(b).ok_or(Error::<T>::InvalidOperands)?;

            // Grab the function and call it.
            let result = Self::instantiate(Self::prepare(raw_blob)?)?
                .call_typed_and_get_result::<u32, (u32, u32)>(&mut (), "sub_numbers", (a, b))
                .map_err(|_| Error::<T>::PolkaVMModuleExecutionFailed)?;

            Ok(result)
        }

        fn mul(a: u32, b: u32, raw_blob: Vec<u8>) -> Result<u32, DispatchError> {
            a.checked_mul(b).ok_or(Error::<T>::InvalidOperands)?;

            // Grab the function and call it.
            let result = Self::instantiate(Self::prepare(raw_blob)?)?
                .call_typed_and_get_result::<u32, (u32, u32)>(&mut (), "mul_numbers", (a, b))
                .map_err(|_| Error::<T>::PolkaVMModuleExecutionFailed)?;

            Ok(result)
        }
    }

    trait ModuleLoader {
        fn prepare(raw_blob: Vec<u8>) -> Result<PolkaVMModule, DispatchError>;
        fn instantiate(module: PolkaVMModule) -> Result<Instance, DispatchError>;
    }

    impl<T: Config> ModuleLoader for Pallet<T> {
        fn prepare(raw_blob: Vec<u8>) -> Result<PolkaVMModule, DispatchError> {
            let blob = ProgramBlob::parse(raw_blob[..].into())
                .map_err(|_| Error::<T>::ProgramBlobParsingFailed)?;

            let config =
                PolkaVMConfig::from_env().map_err(|_| Error::<T>::PolkaVMConfigurationFailed)?;
            let engine =
                Engine::new(&config).map_err(|_| Error::<T>::PolkaVMEngineCreationFailed)?;
            let module = PolkaVMModule::from_blob(&engine, &Default::default(), blob)
                .map_err(|_| Error::<T>::PolkaVMModuleCreationFailed)?;

            Ok(module)
        }

        fn instantiate(module: PolkaVMModule) -> Result<Instance, DispatchError> {
            // High-level API.
            let linker: Linker = Linker::new();

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
}
