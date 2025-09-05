// Copyright (C) QF Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
use frame_system::pallet_prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(core::marker::PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn secure_up_to)]
	/// Highest fast-chain block number that is securely anchored.
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
			ensure_root(origin)?;

			let prev = SecureUpTo::<T>::get();
			if up_to > prev {
				SecureUpTo::<T>::put(up_to);
				Self::deposit_event(Event::<T>::SecureFinalityAdvanced { up_to });
			}

			Ok(())
		}
	}
}
