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
	pub type Relayer<T: Config> = StorageValue<_, T::AccountId>;

	/// Highest fast-chain block number that is securely anchored.
	#[pallet::storage]
	pub type SecureUpTo<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub relayer: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Relayer::<T>::set(self.relayer.clone());
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
			match ensure_signed_or_root(origin)? {
				Some(signer) => ensure!(Relayer::<T>::get() == Some(signer), BadOrigin),
				None => {}, // root is allowed
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
		pub fn set_relayer(origin: OriginFor<T>, new_relayer: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			Relayer::<T>::put(new_relayer);
			Ok(())
		}
	}
}
