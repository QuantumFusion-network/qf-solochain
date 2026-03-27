/// Migration to initialize the `pallet_bags_list` voter index from the existing `pallet_staking` validators and nominators.


use alloc::boxed::Box;
use frame_election_provider_support::SortedListProvider;
use frame_support::{
	traits::OnRuntimeUpgrade,
	weights::Weight,
};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

use crate::{Runtime, Staking, VoterList};

const LOG_TARGET: &str = "runtime::migrations::bags_list";

pub struct InitializeVoterList;

impl InitializeVoterList {
	fn expected_voter_count() -> u32 {
		pallet_staking::Validators::<Runtime>::count()
			.saturating_add(pallet_staking::Nominators::<Runtime>::count())
	}
}

impl OnRuntimeUpgrade for InitializeVoterList {
	fn on_runtime_upgrade() -> Weight {
		let expected = Self::expected_voter_count();
		let current = VoterList::count();

		if current == expected {
			log::info!(
				target: LOG_TARGET,
				"Skipping voter-list initialization; bags-list already contains {} voters",
				current,
			);
			return <Runtime as frame_system::Config>::DbWeight::get().reads(3);
		}

		log::info!(
			target: LOG_TARGET,
			"Initializing bags-list voter index from staking state: current={}, expected={}",
			current,
			expected,
		);

		let inserted = VoterList::unsafe_regenerate(
			pallet_staking::Validators::<Runtime>::iter()
				.map(|(validator, _)| validator)
				.chain(pallet_staking::Nominators::<Runtime>::iter().map(|(nominator, _)| nominator)),
			Box::new(|who| Some(Staking::weight_of(who))),
		);

		let final_count = VoterList::count();
		log::info!(
			target: LOG_TARGET,
			"Finished bags-list initialization: inserted={}, final_count={}",
			inserted,
			final_count,
		);

		// This migration rewrites the full list and can touch a large amount of storage.
		// We return a saturated value and rely on try-runtime checks before enactment.
		Weight::MAX
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let expected = Self::expected_voter_count();
		let current = VoterList::count();
		log::info!(
			target: LOG_TARGET,
			"Pre-upgrade voter-list state: current={}, expected={}",
			current,
			expected,
		);
		Ok((current, expected).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let (pre_count, expected): (u32, u32) = Decode::decode(&mut state.as_slice())
			.map_err(|_| "Failed to decode pre-upgrade voter-list state")?;
		let current = VoterList::count();
		let iter_count = VoterList::iter().count() as u32;

		log::info!(
			target: LOG_TARGET,
			"Post-upgrade voter-list state: before={}, current={}, iter_count={}, expected={}",
			pre_count,
			current,
			iter_count,
			expected,
		);

		ensure!(
			current == expected,
			"bags-list voter count mismatch after migration"
		);
		ensure!(
			iter_count == expected,
			"bags-list iterator length mismatch after migration"
		);

		Ok(())
	}
}
