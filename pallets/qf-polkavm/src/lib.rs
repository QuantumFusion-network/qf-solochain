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

extern crate alloc;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

mod polkavm;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::*;

use codec::Codec;

use frame_support::pallet_prelude::*;
use sp_std::prelude::*;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use alloc::collections::btree_map::BTreeMap;
	use frame_support::{
		dispatch::PostDispatchInfo,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::Preservation,
		},
	};
	use frame_system::pallet_prelude::*;
	use num_derive::{FromPrimitive, ToPrimitive};
	use num_traits::ToPrimitive;
	use scale_info::TypeInfo;
	use sp_runtime::traits::{Hash, SaturatedConversion, TrailingZeroInput};

	use polkavm::{
		CallError, Caller, Config as PolkaVMConfig, Engine, GasMeteringKind, Instance, Linker,
		Module as PolkaVMModule, ModuleConfig as PolkaVMModuleConfig, ProgramBlob, State,
	};

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
	type CodeHash<T> = <T as frame_system::Config>::Hash;
	type CodeVec<T> = BoundedVec<u8, <T as Config>::MaxCodeLen>;
	pub(super) type CodeVersion = u64;
	pub(super) type CodeStorageSlot<T> = BoundedVec<u8, <T as Config>::StorageSize>;
	pub(super) type StorageKey<T> = BoundedVec<u8, <T as Config>::MaxStorageKeySize>;
	pub(super) type CodeStorageKey<T> =
		(<T as frame_system::Config>::AccountId, CodeVersion, StorageKey<T>);

	#[derive(Clone)]
	pub(super) enum MutatingStorageOperationType {
		Set,
		Delete,
	}

	pub(super) type MutatingStorageOperation<T> =
		(MutatingStorageOperationType, CodeStorageKey<T>, Option<CodeStorageSlot<T>>);

	#[derive(Debug, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub(super) struct BlobMetadata<T: Config> {
		pub owner: T::AccountId,
		pub version: CodeVersion,
	}

	#[derive(Debug, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq)]
	pub struct UploadResult<AccountId> {
		pub contract_address: AccountId,
		pub version: CodeVersion,
	}

	#[derive(Debug, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq)]
	pub struct ExecResult {
		pub result: Option<u64>,
		pub not_enough_gas: bool,
		pub trap: bool,
		pub gas_before: u32,
		pub gas_after: i64,
	}

	struct InstanceCallResult {
		result: Option<u64>,
		gas_after: i64,
		not_enough_gas: bool,
		trap: bool
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
		type MaxCodeVersion: Get<u64>;

		#[pallet::constant]
		type MaxUserDataLen: Get<u32>;

		#[pallet::constant]
		type MaxGasLimit: Get<u64>;

		#[pallet::constant]
		type MaxStorageSlots: Get<u32>;

		#[pallet::constant]
		type MaxStorageKeySize: Get<u32>;

		#[pallet::constant]
		type MaxLogLen: Get<u32>;

		#[pallet::constant]
		type MinGasPrice: Get<u64>;

		#[pallet::constant]
		type MinStorageDepositLimit: Get<u64>;

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
	pub(super) type Code<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (CodeVec<T>, CodeVersion)>;

	#[pallet::storage]
	pub(super) type ExecutionResult<T: Config> =
		StorageMap<_, Blake2_128Concat, (T::AccountId, CodeVersion, T::AccountId), ExecResult>;

	#[pallet::storage]
	pub(super) type CodeMetadata<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlobMetadata<T>>;

	#[pallet::storage]
	pub(super) type CodeAddress<T: Config> =
		StorageMap<_, Blake2_128Concat, (T::AccountId, CodeVersion), T::AccountId>;

	#[pallet::storage]
	pub(super) type CodeStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, CodeStorageKey<T>, CodeStorageSlot<T>>;

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
			version: CodeVersion,
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
			version: CodeVersion,
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

		// PolkaVM errors
		ProgramBlobIsTooLarge,
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
		StorageDepositLimitIsTooLow,

		/// Performing the requested transfer failed. Probably because there isn't enough
		/// free balance in the sender's account.
		TransferFailed,

		CodeVersionIsTooBig,
		UserDataIsTooLarge,
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
				.map_err(|_| Error::<T>::ProgramBlobIsTooLarge)?;

			let module = Self::prepare(raw_blob[..].into())?;
			let exports = module
				.exports()
				.map(|export| export.symbol().clone().into_inner().to_vec())
				.collect();

			let mut blob_metadata = match CodeMetadata::<T>::get(&who) {
				Some(meta) => meta,
				None => BlobMetadata { owner: who.clone(), version: 0 },
			};
			blob_metadata.version =
				blob_metadata.version.checked_add(1).ok_or(Error::<T>::IntegerOverflow)?;
			ensure!(
				blob_metadata.version <= <T as Config>::MaxCodeVersion::get(),
				Error::<T>::CodeVersionIsTooBig
			);
			let contract_address =
				Self::contract_address(&who, &T::Hashing::hash_of(&blob_metadata));

			let version = blob_metadata.version;
			Code::<T>::insert(&contract_address, (&raw_blob, &version));
			CodeAddress::<T>::insert((&who, &version), &contract_address);
			CodeMetadata::<T>::insert(&who, blob_metadata);

			Self::deposit_event(Event::ProgramBlobUploaded {
				who,
				contract_address,
				version,
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
			data: Vec<u8>,
			gas_limit: Weight,
			storage_deposit_limit: u64,
			gas_price: u64,
		) -> DispatchResultWithPostInfo {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			Self::check_execute_args(&data, gas_limit, storage_deposit_limit, gas_price)?;

			let gas_before: u32 = gas_limit.ref_time().try_into().map_err(|_| Error::<T>::IntegerOverflow)?;

			let (raw_blob, version) = Code::<T>::get(&contract_address)
				.map(|(blob, version)| (blob.into_inner(), version))
				.ok_or(Error::<T>::ProgramBlobNotFound)?;

			let mut state = Self::init_state(who.clone(), contract_address.clone(), version, data)?;

			let InstanceCallResult { result, gas_after, not_enough_gas, trap } = Self::do_execute(&mut state, gas_before, raw_blob)?;

			if !not_enough_gas && !trap {
				state.mutating_operations.iter().for_each(|op| match op {
					(MutatingStorageOperationType::Delete, key, _) => CodeStorage::<T>::remove(key),
					(MutatingStorageOperationType::Set, key, value) =>
						if let Some(data) = value {
							CodeStorage::<T>::insert(key, data)
						},
				})
			}

			ExecutionResult::<T>::insert(
				(&contract_address, version, &who),
				ExecResult { result, not_enough_gas, trap, gas_before, gas_after },
			);

			// Emit an event.
			Self::deposit_event(Event::ExecutionResult {
				who,
				contract_address,
				version,
				result,
				not_enough_gas,
				trap,
				gas_before,
				gas_after,
			});

			let normalized_gas_after =
				if gas_after < 0 { 0u64 } else { gas_after as u64 };

			Ok(PostDispatchInfo {
				actual_weight: Some(Weight::from_all(gas_limit.ref_time() - normalized_gas_after)),
				pays_fee: Pays::Yes,
			})
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn bare_upload(
			origin: T::AccountId,
			program_blob: Vec<u8>,
		) -> Result<UploadResult<T::AccountId>, DispatchError> {
			log::debug!(
				target: "runtime::qf-polkavm", "bare_upload(origin: {:?}, program_blob.len(): {:?})",
				origin,
				program_blob.len()
			);

			Ok(UploadResult { contract_address: origin.clone(), version: 0 })
		}

		pub fn bare_execute(
			origin: T::AccountId,
			contract_address: T::AccountId,
			data: Vec<u8>,
			gas_limit: Weight,
			storage_deposit_limit: u64,
			gas_price: u64,
		) -> Result<ExecResult, DispatchError> {
			log::debug!(
				target: "runtime::qf-polkavm", "bare_execute(origin: {:?}, contract_address: {:?}, data.len(): {:?}, gas_limit: {:?}, storage_deposit_limit: {:?}, gas_price: {:?})",
				origin,
				contract_address,
				data.len(),
				gas_limit,
				storage_deposit_limit,
				gas_price,
			);

			Self::check_execute_args(&data, gas_limit, storage_deposit_limit, gas_price)?;

			let gas_before: u32 = gas_limit.ref_time().try_into().map_err(|_| Error::<T>::IntegerOverflow)?;

			let (raw_blob, version) = Code::<T>::get(&contract_address)
				.map(|(blob, version)| (blob.into_inner(), version))
				.ok_or(Error::<T>::ProgramBlobNotFound)?;

			let mut state = Self::init_state(origin, contract_address.clone(), version, data)?;

			let InstanceCallResult { result, gas_after, not_enough_gas, trap } = Self::do_execute(&mut state, gas_before, raw_blob)?;

			Ok(ExecResult {
				result,
				not_enough_gas,
				trap,
				gas_before,
				gas_after,
			})
		}

		fn check_execute_args(
			data: &[u8],
			gas_limit: Weight,
			storage_deposit_limit: u64,
			gas_price: u64,
		) -> Result<(), DispatchError> {
			let max_gas_limit = <T as Config>::MaxGasLimit::get()
				.try_into()
				.map_err(|_| Error::<T>::IntegerOverflow)?;

			ensure!(gas_limit.ref_time() <= max_gas_limit, Error::<T>::GasLimitIsTooHigh);

			ensure!(gas_price >= <T as Config>::MinGasPrice::get(), Error::<T>::GasPriceIsTooLow);

			ensure!(
				storage_deposit_limit >= <T as Config>::MinStorageDepositLimit::get(),
				Error::<T>::StorageDepositLimitIsTooLow
			);

			ensure!(
				data.len() <=
					<T as Config>::MaxUserDataLen::get()
						.try_into()
						.map_err(|_| Error::<T>::IntegerOverflow)?,
				Error::<T>::UserDataIsTooLarge
			);

			Ok(())
		}

		fn do_execute(
			state: &mut State<T>,
			gas: u32,
			blob: Vec<u8>,
		) -> Result<InstanceCallResult, DispatchError> {
			let mut instance = Self::instantiate(Self::prepare(blob)?)?;
			instance.set_gas(gas.into());

			sp_runtime::print("====== BEFORE CALL ======");

			let result = instance.call_typed_and_get_result::<u64, ()>(state, "main", ());

			let (result, not_enough_gas, trap) = match result {
				Err(CallError::NotEnoughGas) => (None, true, false),
				Err(CallError::Trap) => (None, false, true),
				Err(_) => Err(Error::<T>::PolkaVMModuleExecutionFailed)?,
				Ok(res) => (Some(res), false, false),
			};

			sp_runtime::print("====== AFTER CALL ======");

			Ok(InstanceCallResult { result, gas_after: instance.gas(), not_enough_gas, trap })
		}

		fn init_state(
			who: T::AccountId,
			contract_address: T::AccountId,
			version: CodeVersion,
			data: Vec<u8>,
		) -> Result<State<T>, DispatchError> {
			let max_storage_size = <T as Config>::StorageSize::get()
				.try_into()
				.map_err(|_| Error::<T>::IntegerOverflow)?;

			let max_storage_key_size = <T as Config>::MaxStorageKeySize::get()
				.try_into()
				.map_err(|_| Error::<T>::IntegerOverflow)?;

			let max_storage_slot_idx = <T as Config>::StorageSize::get()
				.checked_sub(1)
				.ok_or(Error::<T>::IntegerOverflow)?;

			let max_log_len = <T as Config>::MaxLogLen::get()
				.try_into()
				.map_err(|_| Error::<T>::IntegerOverflow)?;

			let state = State {
				addresses: [contract_address.clone(), who.clone()].to_vec(),
				data,
				mutating_operations: [].to_vec(),
				raw_storage: BTreeMap::new(),
				code_version: version,
				max_storage_size,
				max_storage_key_size,
				max_storage_slot_idx,
				max_log_len,
				transfer: |from: T::AccountId, to: T::AccountId, value: u32| -> u64 {
					if !value.is_zero() && from != to {
						if let Err(_) =
							T::Currency::transfer(&from, &to, value.into(), Preservation::Preserve)
						{
							return 1;
						}
					}
					0
				},
				print: |m: Vec<u8>| -> u64 {
					let msg = alloc::string::String::from_utf8_lossy(&m);
					let msg_log = alloc::format!("polkavm: {msg}");
					sp_runtime::print(msg_log.as_str());
					return 0;
				},
				balance: |address: T::AccountId| -> u64 {
					T::Currency::balance(&address).saturated_into()
				},
				block_number: || -> u64 {
					frame_system::Pallet::<T>::block_number().saturated_into()
				},
				account_id: || -> u64 { 0 },
				caller: || -> u64 { 1 },
				get: |contract_address: T::AccountId,
				      version: CodeVersion,
				      key: StorageKey<T>|
				 -> Option<Vec<u8>> {
					CodeStorage::<T>::get((contract_address, version, key)).map(|d| d.to_vec())
				},
			};

			return Ok(state)
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
			#[derive(Debug, FromPrimitive, ToPrimitive)]
			enum HostFunctionError {
				MaxLogLenExceeded = 1005,
				FailedToReadLogBuffer = 1006,
				IndexOutOfBounds = 1007,
				FailedToWriteVmMemory = 1008,
			}

			// High-level API.
			let mut linker: Linker<T> = Linker::<T>::new();

			linker
				.define_typed("transfer", |caller: Caller<T>, balance: u32| -> u64 {
					let from = match caller.user_data.addresses.get(0) {
						Some(from) => from.clone(),
						None =>
							return HostFunctionError::IndexOutOfBounds.to_u64().expect("a number"),
					};

					let to = match caller.user_data.addresses.get(1) {
						Some(to) => to.clone(),
						None =>
							return HostFunctionError::IndexOutOfBounds.to_u64().expect("a number"),
					};

					(caller.user_data.transfer)(from, to, balance)
				})
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("balance", |caller: Caller<T>| -> u64 {
					(caller.user_data.balance)(caller.user_data.addresses[0].clone())
				})
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("balance_of", |caller: Caller<T>| -> u64 {
					(caller.user_data.balance)(caller.user_data.addresses[1].clone())
				})
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("print", |caller: Caller<T>, msg_pointer: u32, len: u32| -> u64 {
					let user_data = caller.user_data;
					if len as usize > user_data.max_log_len {
						return HostFunctionError::MaxLogLenExceeded.to_u64().expect("a number");
					}

					let raw_data = match caller.instance.read_memory(msg_pointer, len) {
						Ok(raw_data) => raw_data,
						Err(_) =>
							return HostFunctionError::FailedToReadLogBuffer
								.to_u64()
								.expect("a number"),
					};

					(user_data.print)(raw_data)
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
				.define_typed("caller", |caller: Caller<T>| -> u64 { (caller.user_data.caller)() })
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("storage_size", |caller: Caller<T>| -> u64 {
					caller.user_data.max_storage_size as u64
				})
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("get_address_len", |caller: Caller<T>, address_idx: u32| -> u64 {
					let address = match caller.user_data.addresses.get(address_idx as usize) {
						Some(address) => address.clone(),
						None =>
							return HostFunctionError::IndexOutOfBounds.to_u64().expect("a number"),
					};
					let raw_data: Vec<u8> = address.encode();

					raw_data.len() as u64
				})
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed(
					"get_address",
					|caller: Caller<T>, address_idx: u32, write_pointer: u32| -> u64 {
						let address = match caller.user_data.addresses.get(address_idx as usize) {
							Some(address) => address.clone(),
							None =>
								return HostFunctionError::IndexOutOfBounds
									.to_u64()
									.expect("a number"),
						};
						let raw_data: Vec<u8> = address.encode();

						match caller.instance.write_memory(write_pointer, &raw_data) {
							Err(_) =>
								HostFunctionError::FailedToWriteVmMemory.to_u64().expect("a number"),
							Ok(_) => 0,
						}
					},
				)
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("get_user_data", |caller: Caller<T>, pointer: u32| -> u64 {
					match caller.instance.write_memory(pointer, &caller.user_data.data) {
						Err(_) =>
							HostFunctionError::FailedToWriteVmMemory.to_u64().expect("a number"),
						Ok(_) => 0,
					}
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
								Err(_) => return 1010,
							};

							let result = match caller.user_data.raw_storage.get(&(
								caller.user_data.addresses[0].clone(),
								caller.user_data.code_version,
								storage_key.clone(),
							)) {
								Some(Some(r)) => Some(r.to_vec()),
								Some(None) => None,
								None => (caller.user_data.get)(
									caller.user_data.addresses[0].clone(),
									caller.user_data.code_version,
									storage_key,
								),
							};

							if let Some(chunk) = result {
								match caller.instance.write_memory(pointer, &chunk) {
									Err(_) => return 1011,
									Ok(_) => return 0,
								}
							}
							return 0;
						} else {
							return 1012;
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
							if let Ok(mut raw_data) = caller
								.instance
								.read_memory(buffer, caller.user_data.max_storage_size as u32)
							{
								let mut storage_key = BoundedVec::with_bounded_capacity(
									caller.user_data.max_storage_key_size as usize,
								);
								match storage_key.try_append(&mut raw_storage_key) {
									Ok(_) => (),
									Err(_) => return 1020,
								}

								let mut data = BoundedVec::with_bounded_capacity(
									caller.user_data.max_storage_size as usize,
								);
								match data.try_append(&mut raw_data) {
									Ok(_) => (),
									Err(_) => return 1021,
								}

								caller.user_data.mutating_operations.push((
									MutatingStorageOperationType::Set,
									(
										caller.user_data.addresses[0].clone(),
										caller.user_data.code_version,
										storage_key.clone(),
									),
									Some(data.clone()),
								));
								caller.user_data.raw_storage.insert(
									(
										caller.user_data.addresses[0].clone(),
										caller.user_data.code_version,
										storage_key,
									),
									Some(data),
								);

								return 0;
							} else {
								1022
							}
						} else {
							1023
						}
					},
				)
				.map_err(|_| Error::<T>::HostFunctionDefinitionFailed)?;

			linker
				.define_typed("delete", |caller: Caller<T>, storage_key_pointer: u32| -> u64 {
					if let Ok(mut raw_storage_key) = caller
						.instance
						.read_memory(storage_key_pointer, caller.user_data.max_storage_key_size)
					{
						let mut storage_key = BoundedVec::with_bounded_capacity(
							caller.user_data.max_storage_key_size as usize,
						);
						match storage_key.try_append(&mut raw_storage_key) {
							Ok(_) => (),
							Err(_) => return 31,
						};

						caller.user_data.mutating_operations.push((
							MutatingStorageOperationType::Delete,
							(
								caller.user_data.addresses[0].clone(),
								caller.user_data.code_version,
								storage_key.clone(),
							),
							None,
						));
						caller.user_data.raw_storage.insert(
							(
								caller.user_data.addresses[0].clone(),
								caller.user_data.code_version,
								storage_key,
							),
							None,
						);

						return 0;
					} else {
						32
					}
				})
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

sp_api::decl_runtime_apis! {
	/// The API used to dry-run smart contract interactions.
	pub trait QfPolkavmApi<AccountId, Balance> where
		AccountId: Codec,
		Balance: Codec,
	{
		/// Upload new smart contract.
		///
		/// See [`crate::Pallet::bare_upload`].
		fn upload(origin: AccountId, program_blob: Vec<u8>) -> Result<UploadResult<AccountId>, DispatchError>;

		/// Execute a given smart contract from a specified account.
		///
		/// See [`crate::Pallet::bare_execute`].
		fn execute(
			origin: AccountId,
			contract_address: AccountId,
			data: Vec<u8>,
			gas_limit: Option<Weight>,
			storage_deposit_limit: u64,
			gas_price: u64,
		) -> Result<ExecResult, DispatchError>;
	}
}
