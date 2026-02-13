#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
use frame_system::pallet_prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
		DefaultNoBound,
	};
	use sp_runtime::traits::BadOrigin;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(core::marker::PhantomData<T>);

	#[pallet::storage]
	pub type RelayerOrigin<T: Config> = StorageValue<_, T::AccountId>;

	/// Highest fast-chain block number that is securely anchored.
	#[pallet::storage]
	pub type SecureUpTo<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub relayer_origin: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			RelayerOrigin::<T>::set(self.relayer_origin.clone());
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Secure finality advanced to `up_to`.
		SecureFinalityAdvanced { up_to: BlockNumberFor<T> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Call when anchor verification completes
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn note_anchor_verified(
			origin: OriginFor<T>,
			up_to: BlockNumberFor<T>,
		) -> DispatchResult {
			let signer = ensure_signed_or_root(origin)?;
			if let Some(signer) = signer {
				let relayer_origin = RelayerOrigin::<T>::get();
				ensure!(Some(signer) == relayer_origin, BadOrigin);
			}

			let prev = SecureUpTo::<T>::get();
			if up_to > prev {
				SecureUpTo::<T>::put(up_to);
				Self::deposit_event(Event::<T>::SecureFinalityAdvanced { up_to });
			}

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_relayer_origin(
			origin: OriginFor<T>,
			new_relayer_origin: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			RelayerOrigin::<T>::put(new_relayer_origin);
			Ok(())
		}
	}
}
