#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode};
use finality_grandpa::Message as GrandpaMessage;
use frame_support::{ensure, pallet_prelude::*, BoundedVec};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_consensus_grandpa::{self, AuthorityList, SetId};
use sp_runtime::traits::Header as HeaderT;
use sp_std::collections::btree_set::BTreeSet;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// TODO(zotho): pick sane limits for our network
	pub const MAX_VOTES_ANCESTRIES: u32 = 512;

	/// Identical to `sp_consensus_grandpa::GrandpaJustification` but with bounded
	/// `votes_ancestries` vector.
	#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
	pub struct BoundedGrandpaJustification<H: HeaderT> {
		pub round: u64,
		pub commit: sp_consensus_grandpa::Commit<H>,
		pub votes_ancestries: BoundedVec<H, ConstU32<MAX_VOTES_ANCESTRIES>>,
	}

	impl<H: HeaderT> core::fmt::Debug for BoundedGrandpaJustification<H> {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.debug_struct("BoundedGrandpaJustification")
				.field("round", &self.round)
				.field("commit", &"<commit>") // TODO: replace the placeholder
				.field("votes_ancestries", &self.votes_ancestries.len())
				.finish()
		}
	}

	/// Stored configuration for the GRANDPA authority set that signs fastchain finality proofs.
	#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
	pub struct AuthoritySetData {
		pub set_id: SetId,
		pub authorities: AuthorityList,
	}

	/// Metadata about the best fastchain block accepted on the parachain.
	#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
	pub struct FinalizedTarget<BlockNumber, Hash> {
		pub number: BlockNumber,
		pub hash: Hash,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The header type for the anchored chain.
		type AnchoredChainHeader: HeaderT + TypeInfo;
	}

	// TODO(zotho): remove `without_storage_info`
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// TODO(zotho): Add MEL bound for `AuthoritySetData` https://github.com/QuantumFusion-network/spec/issues/629
	/// Current GRANDPA authority set information.
	#[pallet::storage]
	pub type FastchainAuthoritySet<T: Config> = StorageValue<_, AuthoritySetData>;

	/// Highest fastchain block known to be finalized on the parachain.
	#[pallet::storage]
	pub type LastFinalized<T: Config> =
		StorageValue<_, FinalizedTarget<<T::AnchoredChainHeader as HeaderT>::Number, <T::AnchoredChainHeader as HeaderT>::Hash>>;

	// TODO(zotho): Add MEL bound https://github.com/QuantumFusion-network/spec/issues/629
	/// The most recent justification bytes accepted. This is informational only.
	#[pallet::storage]
	pub type LastJustification<T: Config> =
		StorageValue<_, BoundedGrandpaJustification<T::AnchoredChainHeader>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The parachain accepted a new fastchain finality proof.
		FinalityProofAccepted {
			who: T::AccountId,
			number: <T::AnchoredChainHeader as HeaderT>::Number,
			hash: <T::AnchoredChainHeader as HeaderT>::Hash,
		},
		/// The GRANDPA authority set was updated.
		AuthoritySetUpdated { set_id: SetId, authorities: u64 },
	}

	#[pallet::error]
	pub enum Error<T> {
		AuthoritySetNotInitialized,
		AuthoritySetMismatch,
		EmptyAuthoritySet,
		UnknownAuthority,
		BadSignature,
		InsufficientWeight,
		AlreadyFinalized,
		UnsupportedBlockNumber,
		MismatchedTargets,
		NoPrecommits,
		ComputationOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set (or refresh) the GRANDPA authority set that the parachain trusts for fastchain.
		///
		/// The call must be dispatched by `Root`.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_authority_set(
			origin: OriginFor<T>,
			set_id: SetId,
			authorities: AuthorityList,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(!authorities.is_empty(), Error::<T>::EmptyAuthoritySet);

			let authorities_len =
				u64::try_from(authorities.len()).map_err(|_| Error::<T>::ComputationOverflow)?;

			FastchainAuthoritySet::<T>::put(AuthoritySetData { set_id, authorities });

			Self::deposit_event(Event::AuthoritySetUpdated {
				set_id,
				authorities: authorities_len,
			});
			Ok(())
		}

		/// Submit a `GrandpaJustification` produced by the fastchain node.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
		pub fn submit_finality_proof(
			origin: OriginFor<T>,
			expected_set_id: SetId,
			justification: BoundedGrandpaJustification<T::AnchoredChainHeader>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(!justification.commit.precommits.is_empty(), Error::<T>::NoPrecommits);

			let authority_set =
				FastchainAuthoritySet::<T>::get().ok_or(Error::<T>::AuthoritySetNotInitialized)?;
			ensure!(!authority_set.authorities.is_empty(), Error::<T>::EmptyAuthoritySet);
			ensure!(authority_set.set_id == expected_set_id, Error::<T>::AuthoritySetMismatch);

			// TODO(zotho): how do we verify that `target_hash` is hash of `target_number` block?
			let target_hash = justification.commit.target_hash;
			let target_number = justification.commit.target_number;

			if let Some(last) = LastFinalized::<T>::get() {
				ensure!(target_number > last.number, Error::<T>::AlreadyFinalized);
			}

			// TODO(zotho): do we have to compute the total every time?
			let mut total_weight: u128 = 0;
			for &(_, weight) in &authority_set.authorities {
				total_weight = total_weight
					.checked_add(u128::from(weight))
					.ok_or(Error::<T>::ComputationOverflow)?;
			}
			ensure!(total_weight > 0, Error::<T>::EmptyAuthoritySet);

			let mut seen = BTreeSet::new();
			let mut signed_weight: u128 = 0;

			for signed in &justification.commit.precommits {
				ensure!(
					signed.precommit.target_hash == target_hash &&
						signed.precommit.target_number == justification.commit.target_number,
					Error::<T>::MismatchedTargets
				);

				let signature_ok = sp_consensus_grandpa::check_message_signature(
					&GrandpaMessage::Precommit(signed.precommit.clone()),
					&signed.id,
					&signed.signature,
					justification.round,
					authority_set.set_id,
				)
				.is_valid();
				ensure!(signature_ok, Error::<T>::BadSignature);

				// TODO(zotho): only first seen here. do we need filtering?
				if seen.insert(signed.id.clone()) {
					let weight = authority_set
						.authorities
						.iter()
						.find_map(|(id, weight)| if *id == signed.id { Some(weight) } else { None })
						.ok_or(Error::<T>::UnknownAuthority)?;
					signed_weight = signed_weight.saturating_add(u128::from(*weight));
				}
			}

			ensure!(
				signed_weight.saturating_mul(3) >= total_weight.saturating_mul(2),
				Error::<T>::InsufficientWeight
			);

			LastFinalized::<T>::put(FinalizedTarget { number: target_number, hash: target_hash });
			LastJustification::<T>::put(justification);

			Self::deposit_event(Event::FinalityProofAccepted {
				who,
				number: target_number,
				hash: target_hash,
			});
			Ok(())
		}
	}
}
