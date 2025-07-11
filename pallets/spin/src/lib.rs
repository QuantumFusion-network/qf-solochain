// Copyright (C) Quantum Fusion Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::{DisabledValidators, FindAuthor, Get, OnTimestampSet, OneSessionHandler},
	BoundedSlice, BoundedVec, ConsensusEngineId, Parameter,
};
use log;
use qfp_consensus_spin::{
	AuthorityIndex, ConsensusLog, SessionLength as SessionLengthT, Slot, SpinAuxData,
	SPIN_ENGINE_ID,
};
use sp_runtime::{
	generic::DigestItem,
	traits::{IsMember, Member, SaturatedConversion, Saturating, Zero},
	RuntimeAppPublic,
};

mod mock;
mod tests;

pub use pallet::*;

const LOG_TARGET: &str = "runtime::spin";

/// A slot duration provider which infers the slot duration from the
/// [`pallet_timestamp::Config::MinimumPeriod`] by multiplying it by two, to
/// ensure that authors have the majority of their slot to author within.
///
/// This was the default behavior of the SPIN pallet and may be used for
/// backwards compatibility.
pub struct MinimumPeriodTimesTwo<T>(core::marker::PhantomData<T>);

impl<T: pallet_timestamp::Config> Get<T::Moment> for MinimumPeriodTimesTwo<T> {
	fn get() -> T::Moment {
		<T as pallet_timestamp::Config>::MinimumPeriod::get().saturating_mul(2u32.into())
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		/// The maximum number of authorities that the pallet can hold.
		type MaxAuthorities: Get<u32>;

		/// A way to check whether a given validator is disabled and should not
		/// be authoring blocks. Blocks authored by a disabled validator will
		/// lead to a panic as part of this module's initialization.
		type DisabledValidators: DisabledValidators;

		/// Whether to allow block authors to create multiple blocks per slot.
		///
		/// If this is `true`, the pallet will allow slots to stay the same
		/// across sequential blocks. If this is `false`, the pallet will
		/// require that subsequent blocks always have higher slots than
		/// previous ones.
		///
		/// Regardless of the setting of this storage value, the pallet will
		/// always enforce the invariant that slots don't move backwards as
		/// the chain progresses.
		///
		/// The typical value for this should be 'false' unless this pallet is
		/// being augmented by another pallet which enforces some limitation
		/// on the number of blocks authors can create using the same slot.
		type AllowMultipleBlocksPerSlot: Get<bool>;

		/// The slot duration SPIN should run with, expressed in milliseconds.
		/// The effective value of this type should not change while the chain
		/// is running.
		///
		/// For backwards compatibility either use [`MinimumPeriodTimesTwo`] or
		/// a const.
		#[pallet::constant]
		type SlotDuration: Get<<Self as pallet_timestamp::Config>::Moment>;

		/// Default session length in blocks.
		#[pallet::constant]
		type DefaultSessionLength: Get<SessionLengthT>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(core::marker::PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			if let Some(new_slot) = Self::current_slot_from_digests() {
				let current_slot = CurrentSlot::<T>::get();

				if T::AllowMultipleBlocksPerSlot::get() {
					assert!(current_slot <= new_slot, "Slot must not decrease");
				} else {
					assert!(current_slot < new_slot, "Slot must increase");
				}

				CurrentSlot::<T>::put(new_slot);

				if let Some(n_authorities) = <Authorities<T>>::decode_len() {
					let authority_index = *new_slot % n_authorities as u64;
					if T::DisabledValidators::is_disabled(authority_index as u32) {
						panic!(
							"Validator with index {:?} is disabled and should not be attempting to author blocks.",
							authority_index,
						);
					}
				}

				// TODO [#3398] Generate offence report for all authorities that skipped their
				// slots.

				T::DbWeight::get().reads_writes(2, 1)
			} else {
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			Self::do_try_state()
		}
	}

	/// The current authority set.
	#[pallet::storage]
	pub type Authorities<T: Config> =
		StorageValue<_, BoundedVec<T::AuthorityId, T::MaxAuthorities>, ValueQuery>;

	/// The current slot of this block.
	///
	/// This will be set in `on_initialize`.
	#[pallet::storage]
	pub type CurrentSlot<T: Config> = StorageValue<_, Slot, ValueQuery>;

	/// Session length in blocks
	///
	/// Selected leader produces blocks for `SessionLength` blocks.
	#[pallet::storage]
	pub type SessionLength<T: Config> =
		StorageValue<_, SessionLengthT, ValueQuery, T::DefaultSessionLength>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub authorities: Vec<T::AuthorityId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_authorities(&self.authorities);
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Zero session length.
		SessionLengthZero,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New session length set.
		NewSessionLength(SessionLengthT),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set new session length.
		///
		/// Origin must be root.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(0, 1))]
		pub fn set_session_length(
			origin: OriginFor<T>,
			session_len: SessionLengthT,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Ensure the session length is not zero.
			ensure!(!session_len.is_zero(), Error::<T>::SessionLengthZero);

			SessionLength::<T>::put(session_len);

			Self::deposit_event(Event::NewSessionLength(session_len));

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Change authorities.
	///
	/// The storage will be applied immediately.
	/// And SPIN consensus log will be appended to block's log.
	///
	/// This is a no-op if `new` is empty.
	pub fn change_authorities(new: BoundedVec<T::AuthorityId, T::MaxAuthorities>) {
		if new.is_empty() {
			log::warn!(target: LOG_TARGET, "Ignoring empty authority change.");

			return;
		}

		<Authorities<T>>::put(&new);

		let log = DigestItem::Consensus(
			SPIN_ENGINE_ID,
			ConsensusLog::AuthoritiesChange(new.into_inner()).encode(),
		);
		<frame_system::Pallet<T>>::deposit_log(log);
	}

	/// Initial authorities.
	///
	/// The storage will be applied immediately.
	///
	/// The authorities length must be equal or less than T::MaxAuthorities.
	pub fn initialize_authorities(authorities: &[T::AuthorityId]) {
		if !authorities.is_empty() {
			assert!(<Authorities<T>>::get().is_empty(), "Authorities are already initialized!");
			let bounded = <BoundedSlice<'_, _, T::MaxAuthorities>>::try_from(authorities)
				.expect("Initial authority set must be less than T::MaxAuthorities");
			<Authorities<T>>::put(bounded);
		}
	}

	/// Return current authorities length.
	pub fn authorities_len() -> usize {
		Authorities::<T>::decode_len().unwrap_or(0)
	}

	/// Get the current slot from the pre-runtime digests.
	fn current_slot_from_digests() -> Option<Slot> {
		let digest = frame_system::Pallet::<T>::digest();
		let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
		for (id, mut data) in pre_runtime_digests {
			if id == SPIN_ENGINE_ID {
				return Slot::decode(&mut data).ok();
			}
		}

		None
	}

	/// Determine the SPIN slot-duration based on the Timestamp module
	/// configuration.
	pub fn slot_duration() -> T::Moment {
		T::SlotDuration::get()
	}

	/// Auxiliary data for SPIN.
	pub fn aux_data() -> SpinAuxData<T::AuthorityId> {
		let authorities = Authorities::<T>::get();
		let session_length = SessionLength::<T>::get();
		(authorities.into_inner(), session_length)
	}

	/// Ensure the correctness of the state of this pallet.
	///
	/// This should be valid before or after each state transition of this
	/// pallet.
	///
	/// # Invariants
	///
	/// ## `CurrentSlot`
	///
	/// If we don't allow for multiple blocks per slot, then the current slot
	/// must be less than the maximal slot number. Otherwise, it can be
	/// arbitrary.
	///
	/// ## `Authorities`
	///
	/// * The authorities must be non-empty.
	/// * The current authority cannot be disabled.
	/// * The number of authorities must be less than or equal to `T::MaxAuthorities`. This however,
	///   is guarded by the type system.
	#[cfg(any(test, feature = "try-runtime"))]
	pub fn do_try_state() -> Result<(), sp_runtime::TryRuntimeError> {
		// We don't have any guarantee that we are already after `on_initialize` and
		// thus we have to check the current slot from the digest or take the last
		// known slot.
		let current_slot =
			Self::current_slot_from_digests().unwrap_or_else(|| CurrentSlot::<T>::get());

		// Check that the current slot is less than the maximal slot number, unless we
		// allow for multiple blocks per slot.
		if !T::AllowMultipleBlocksPerSlot::get() {
			frame_support::ensure!(
				current_slot < u64::MAX,
				"Current slot has reached maximum value and cannot be incremented further.",
			);
		}

		let authorities_len =
			<Authorities<T>>::decode_len().ok_or("Failed to decode authorities length")?;

		// Check that the authorities are non-empty.
		frame_support::ensure!(!authorities_len.is_zero(), "Authorities must be non-empty.");

		// Check that the current authority is not disabled.
		let authority_index = *current_slot % authorities_len as u64;
		frame_support::ensure!(
			!T::DisabledValidators::is_disabled(authority_index as u32),
			"Current validator is disabled and should not be attempting to author blocks.",
		);

		Ok(())
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		Self::initialize_authorities(&authorities);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		// instant changes
		if changed {
			let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
			let last_authorities = Authorities::<T>::get();
			if last_authorities != next_authorities {
				if next_authorities.len() as u32 > T::MaxAuthorities::get() {
					log::warn!(
						target: LOG_TARGET,
						"next authorities list larger than {}, truncating",
						T::MaxAuthorities::get(),
					);
				}
				let bounded = <BoundedVec<_, T::MaxAuthorities>>::truncate_from(next_authorities);
				Self::change_authorities(bounded);
			}
		}
	}

	fn on_disabled(i: u32) {
		let log = DigestItem::Consensus(
			SPIN_ENGINE_ID,
			ConsensusLog::<T::AuthorityId>::OnDisabled(i as AuthorityIndex).encode(),
		);

		<frame_system::Pallet<T>>::deposit_log(log);
	}
}

impl<T: Config> FindAuthor<u32> for Pallet<T> {
	fn find_author<'a, I>(digests: I) -> Option<u32>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		for (id, mut data) in digests.into_iter() {
			if id == SPIN_ENGINE_ID {
				let slot = Slot::decode(&mut data).ok()?;
				let author_index = *slot % Self::authorities_len() as u64;
				return Some(author_index as u32);
			}
		}

		None
	}
}

/// We can not implement `FindAuthor` twice, because the compiler does not know
/// if `u32 == T::AuthorityId` and thus, prevents us to implement the trait
/// twice.
#[doc(hidden)]
pub struct FindAccountFromAuthorIndex<T, Inner>(core::marker::PhantomData<(T, Inner)>);

impl<T: Config, Inner: FindAuthor<u32>> FindAuthor<T::AuthorityId>
	for FindAccountFromAuthorIndex<T, Inner>
{
	fn find_author<'a, I>(digests: I) -> Option<T::AuthorityId>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		let i = Inner::find_author(digests)?;

		let validators = Authorities::<T>::get();
		validators.get(i as usize).cloned()
	}
}

/// Find the authority ID of the SPIN authority who authored the current block.
pub type SpinAuthorId<T> = FindAccountFromAuthorIndex<T, Pallet<T>>;

impl<T: Config> IsMember<T::AuthorityId> for Pallet<T> {
	fn is_member(authority_id: &T::AuthorityId) -> bool {
		Authorities::<T>::get().iter().any(|id| id == authority_id)
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
	fn on_timestamp_set(moment: T::Moment) {
		let slot_duration = Self::slot_duration();
		assert!(!slot_duration.is_zero(), "SPIN slot duration cannot be zero.");

		let timestamp_slot = moment / slot_duration;
		let timestamp_slot = Slot::from(timestamp_slot.saturated_into::<u64>());

		assert_eq!(
			CurrentSlot::<T>::get(),
			timestamp_slot,
			"Timestamp slot must match `CurrentSlot`. This likely means that the configured block \
			time in the node and/or rest of the runtime is not compatible with SPIN's \
			`SlotDuration`",
		);
	}
}
