#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::Currency};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The currency type that the faucet will dispense
		type Currency: Currency<Self::AccountId>;

		/// The amount that the faucet will dispense
		#[pallet::constant]
		type FaucetAmount: Get<BalanceOf<Self>>;

		/// The number of blocks that must pass between drips
		#[pallet::constant]
		type LockPeriod: Get<BlockNumberFor<Self>>;

		/// Weight information for extrinsics in this pallet
		type WeightInfo: WeightInfo;
	}

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type BlockNumberFor<T> = frame_system::pallet_prelude::BlockNumberFor<T>;

	#[pallet::storage]
	pub type LastDrip<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Tokens have been dispensed from the faucet
		FaucetCalled { who: T::AccountId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The user has requested tokens too recently
		DripTooSoon,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Request tokens from the faucet
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::drip())]
		pub fn drip(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();
			let last_drip = LastDrip::<T>::get(&who);

			ensure!(
				last_drip == 0u32.into() || current_block >= last_drip + T::LockPeriod::get(),
				Error::<T>::DripTooSoon
			);

			let amount = T::FaucetAmount::get();
			let _ = T::Currency::deposit_creating(&who, amount);

			LastDrip::<T>::insert(&who, current_block);

			Self::deposit_event(Event::FaucetCalled { who, amount });
			Ok(())
		}
	}
}
