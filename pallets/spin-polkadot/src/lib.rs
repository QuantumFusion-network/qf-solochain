#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use codec::{Decode, Encode};
use finality_grandpa::Message as GrandpaMessage;
use frame_support::{ensure, pallet_prelude::*};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_consensus_grandpa::{
	self, AuthorityId, AuthorityList, AuthorityWeight, GrandpaJustification, SetId,
};
use sp_runtime::traits::Header as HeaderT;
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	vec::Vec,
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Stored configuration for the GRANDPA authority set that signs FastChain finality proofs.
	#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
	pub struct AuthoritySetData {
		pub set_id: SetId,
		pub authorities: AuthorityList,
	}

	/// Metadata about the best FastChain block accepted on the parachain.
	#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
	pub struct FinalizedTarget<BlockNumber, Hash> {
		pub number: BlockNumber,
		pub hash: Hash,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Current GRANDPA authority set information.
	#[pallet::storage]
	pub type FastchainAuthoritySet<T: Config> = StorageValue<_, AuthoritySetData>;

	/// Highest fastchain block known to be finalized on the parachain.
	#[pallet::storage]
	pub type LastFinalized<T: Config> =
		StorageValue<_, FinalizedTarget<BlockNumberFor<T>, <HeaderFor<T> as HeaderT>::Hash>>;

	// TODO(zotho): Add MEL bound https://github.com/QuantumFusion-network/spec/issues/629
	/// The most recent justification bytes accepted. This is informational only.
	#[pallet::storage]
	pub type LastJustification<T: Config> = StorageValue<_, GrandpaJustification<HeaderFor<T>>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The parachain accepted a new FastChain finality proof.
		FinalityProofAccepted {
			who: T::AccountId,
			number: BlockNumberFor<T>,
			hash: <HeaderFor<T> as HeaderT>::Hash,
		},
		/// The GRANDPA authority set was updated.
		AuthoritySetUpdated { set_id: SetId, authorities: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		AuthoritySetNotInitialized,
		AuthoritySetMismatch,
		MalformedProof,
		EmptyAuthoritySet,
		UnknownAuthority,
		BadSignature,
		InsufficientWeight,
		AlreadyFinalized,
		UnsupportedBlockNumber,
		MismatchedTargets,
		NoPrecommits,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set (or refresh) the GRANDPA authority set that the parachain trusts for FastChain.
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

			FastchainAuthoritySet::<T>::put(AuthoritySetData {
				set_id,
				authorities: authorities.clone(),
			});

			Self::deposit_event(Event::AuthoritySetUpdated {
				set_id,
				authorities: authorities.len() as u32,
			});
			Ok(())
		}

		/// Submit a SCALE-encoded `GrandpaJustification` produced by the FastChain node.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 2))]
		pub fn submit_finality_proof(
			origin: OriginFor<T>,
			expected_set_id: SetId,
			proof: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let authority_set =
				FastchainAuthoritySet::<T>::get().ok_or(Error::<T>::AuthoritySetNotInitialized)?;
			ensure!(authority_set.set_id == expected_set_id, Error::<T>::AuthoritySetMismatch);
			ensure!(!authority_set.authorities.is_empty(), Error::<T>::EmptyAuthoritySet);

			let justification = GrandpaJustification::<HeaderFor<T>>::decode(&mut &proof[..])
				.map_err(|_| Error::<T>::MalformedProof)?;

			ensure!(!justification.commit.precommits.is_empty(), Error::<T>::NoPrecommits);

			let target_hash = justification.commit.target_hash;
			let target_number: BlockNumberFor<T> = justification
				.commit
				.target_number
				.try_into()
				.map_err(|_| Error::<T>::UnsupportedBlockNumber)?;

			if let Some(last) = LastFinalized::<T>::get() {
				ensure!(target_number > last.number, Error::<T>::AlreadyFinalized);
			}

			let weight_map: BTreeMap<AuthorityId, AuthorityWeight> =
				authority_set.authorities.iter().cloned().collect();

			let total_weight: u128 =
				authority_set.authorities.iter().map(|(_, weight)| u128::from(*weight)).sum();
			ensure!(total_weight > 0, Error::<T>::EmptyAuthoritySet);

			let mut seen = BTreeSet::new();
			let mut signed_weight: u128 = 0;

			for signed in &justification.commit.precommits {
				ensure!(
					signed.precommit.target_hash == target_hash
						&& signed.precommit.target_number == justification.commit.target_number,
					Error::<T>::MismatchedTargets
				);

				let signature_ok = sp_consensus_grandpa::check_message_signature(
					&GrandpaMessage::Precommit(signed.precommit.clone()),
					&signed.id,
					&signed.signature,
					justification.round,
					authority_set.set_id,
				);
				ensure!(signature_ok, Error::<T>::BadSignature);

				if seen.insert(signed.id.clone()) {
					let weight = weight_map.get(&signed.id).ok_or(Error::<T>::UnknownAuthority)?;
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
