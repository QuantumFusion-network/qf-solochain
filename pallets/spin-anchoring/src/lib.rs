#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
use frame_system::pallet_prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RelayerOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(core::marker::PhantomData<T>);

	/// Highest fast-chain block number that is securely anchored.
	#[pallet::storage]
	pub type SecureUpTo<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

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
			T::RelayerOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			let prev = SecureUpTo::<T>::get();
			if up_to > prev {
				SecureUpTo::<T>::put(up_to);
				Self::deposit_event(Event::<T>::SecureFinalityAdvanced { up_to });
			}

			Ok(())
		}
	}
}
