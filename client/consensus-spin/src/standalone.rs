// Copyright (C) Quantum Fusion Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

//! Standalone functions used within the implementation of SPIN.

use std::fmt::Debug;

use log::trace;

use codec::Codec;

use sc_client_api::UsageProvider;
use sp_api::{Core, ProvideRuntimeApi};
use sp_application_crypto::{AppCrypto, AppPublic};
use sp_blockchain::Result as CResult;
use sp_consensus::Error as ConsensusError;
use sp_consensus_slots::Slot;
use sp_core::crypto::{ByteArray, Pair};
use sp_keystore::KeystorePtr;
use sp_runtime::{
	traits::{Block as BlockT, Header, NumberFor, Zero},
	DigestItem,
};

pub use sc_consensus_slots::check_equivocation;

use super::{
	AuthorityId, CompatibilityMode, CompatibleDigestItem, SessionLength, SlotDuration, SpinApi,
	SpinAuxData, LOG_TARGET,
};

/// Get the slot duration for SPIN by reading from a runtime API at the best
/// block's state.
pub fn slot_duration<A, B, C>(client: &C) -> CResult<SlotDuration>
where
	A: Codec,
	B: BlockT,
	C: ProvideRuntimeApi<B> + UsageProvider<B>,
	C::Api: SpinApi<B, A>,
{
	slot_duration_at(client, client.usage_info().chain.best_hash)
}

/// Get the slot duration for SPIN by reading from a runtime API at a given
/// block's state.
pub fn slot_duration_at<A, B, C>(client: &C, block_hash: B::Hash) -> CResult<SlotDuration>
where
	A: Codec,
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C::Api: SpinApi<B, A>,
{
	client.runtime_api().slot_duration(block_hash).map_err(|err| err.into())
}

/// Get the slot author for given block along with authorities.
///
/// Session increment is assumed to be done by the runtime
pub fn slot_author<P: Pair>(
	slot: Slot,
	session_length: SessionLength,
	authorities: &[AuthorityId<P>],
) -> Option<&AuthorityId<P>> {
	if authorities.is_empty() {
		return None;
	}
	assert!(session_length != 0, "session_length can not be zero; qed");

	let session_idx = *slot / session_length as u64;

	let idx = session_idx as u64 % (authorities.len() as u64);
	assert!(
		idx <= usize::MAX as u64,
		"It is impossible to have a vector with length beyond the address space; qed",
	);

	let current_author = authorities.get(idx as usize).expect(
		"authorities not empty; index constrained to list length;this is a valid index; qed",
	);

	Some(current_author)
}

/// Attempt to claim a slot using a keystore.
///
/// This returns `None` if the slot author is not locally controlled, and `Some`
/// if it is, with the public key of the slot author.
pub async fn claim_slot<P: Pair>(
	slot: Slot,
	aux_data: &SpinAuxData<AuthorityId<P>>,
	keystore: &KeystorePtr,
) -> Option<P::Public> {
	let (authorities, session_length) = aux_data;
	let expected_author = slot_author::<P>(slot, *session_length, authorities);
	expected_author.and_then(|p| {
		// TODO: add SPIN key type
		if keystore.has_keys(&[(p.to_raw_vec(), sp_application_crypto::key_types::AURA)]) {
			Some(p.clone())
		} else {
			None
		}
	})
}

/// Produce the pre-runtime digest containing the slot info.
///
/// This is intended to be put into the block header prior to runtime execution,
/// so the runtime can read the slot in this way.
pub fn pre_digest<P: Pair>(slot: Slot) -> sp_runtime::DigestItem
where
	P::Signature: Codec,
{
	<DigestItem as CompatibleDigestItem<P::Signature>>::spin_pre_digest(slot)
}

/// Produce the seal digest item by signing the hash of a block.
///
/// Note that after this is added to a block header, the hash of the block will
/// change.
pub fn seal<Hash, P>(
	header_hash: &Hash,
	public: &P::Public,
	keystore: &KeystorePtr,
) -> Result<sp_runtime::DigestItem, ConsensusError>
where
	Hash: AsRef<[u8]>,
	P: Pair,
	P::Signature: Codec + TryFrom<Vec<u8>>,
	P::Public: AppPublic,
{
	let signature = keystore
		.sign_with(
			<AuthorityId<P> as AppCrypto>::ID,
			<AuthorityId<P> as AppCrypto>::CRYPTO_ID,
			public.as_slice(),
			header_hash.as_ref(),
		)
		.map_err(|e| ConsensusError::CannotSign(format!("{}. Key: {:?}", e, public)))?
		.ok_or_else(|| {
			ConsensusError::CannotSign(format!("Could not find key in keystore. Key: {:?}", public))
		})?;

	let signature = signature
		.clone()
		.try_into()
		.map_err(|_| ConsensusError::InvalidSignature(signature, public.to_raw_vec()))?;

	let signature_digest_item =
		<DigestItem as CompatibleDigestItem<P::Signature>>::spin_seal(signature);

	Ok(signature_digest_item)
}

/// Errors in pre-digest lookup.
#[derive(Debug, thiserror::Error)]
pub enum PreDigestLookupError {
	/// Multiple SPIN pre-runtime headers
	#[error("Multiple SPIN pre-runtime headers")]
	MultipleHeaders,
	/// No SPIN pre-runtime digest found
	#[error("No SPIN pre-runtime digest found")]
	NoDigestFound,
}

/// Extract a pre-digest from a block header.
///
/// This fails if there is no pre-digest or there are multiple.
///
/// Returns the `slot` stored in the pre-digest or an error if no pre-digest was
/// found.
pub fn find_pre_digest<B: BlockT, Signature: Codec>(
	header: &B::Header,
) -> Result<Slot, PreDigestLookupError> {
	if header.number().is_zero() {
		return Ok(0.into());
	}

	let mut pre_digest: Option<Slot> = None;
	for log in header.digest().logs() {
		trace!(target: LOG_TARGET, "Checking log {:?}", log);
		match (CompatibleDigestItem::<Signature>::as_spin_pre_digest(log), pre_digest.is_some()) {
			(Some(_), true) => return Err(PreDigestLookupError::MultipleHeaders),
			(None, _) => trace!(target: LOG_TARGET, "Ignoring digest not meant for us"),
			(s, false) => pre_digest = s,
		}
	}
	pre_digest.ok_or_else(|| PreDigestLookupError::NoDigestFound)
}

/// Fetch the current set of authorities from the runtime at a specific block.
///
/// The compatibility mode and context block number informs this function
/// whether to initialize the hypothetical block created by the runtime API as
/// backwards compatibility for older chains.
pub fn fetch_authorities_with_compatibility_mode<A, B, C>(
	client: &C,
	parent_hash: B::Hash,
	context_block_number: NumberFor<B>,
	compatibility_mode: &CompatibilityMode<NumberFor<B>>,
) -> Result<SpinAuxData<A>, ConsensusError>
where
	A: Codec + Debug,
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C::Api: SpinApi<B, A>,
{
	let runtime_api = client.runtime_api();

	match compatibility_mode {
		CompatibilityMode::None => {},
		// Use `initialize_block` until we hit the block that should disable the mode.
		CompatibilityMode::UseInitializeBlock { until } =>
			if *until > context_block_number {
				runtime_api
					.initialize_block(
						parent_hash,
						&B::Header::new(
							context_block_number,
							Default::default(),
							Default::default(),
							parent_hash,
							Default::default(),
						),
					)
					.map_err(|_| ConsensusError::InvalidAuthoritiesSet)?;
			},
	}

	runtime_api
		.aux_data(parent_hash)
		.ok()
		.ok_or(ConsensusError::InvalidAuthoritiesSet)
}

/// Load the current set of authorities from a runtime at a specific block.
pub fn fetch_aux_data<A, B, C>(
	client: &C,
	parent_hash: B::Hash,
) -> Result<SpinAuxData<A>, ConsensusError>
where
	A: Codec + Debug,
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C::Api: SpinApi<B, A>,
{
	client
		.runtime_api()
		.aux_data(parent_hash)
		.ok()
		.ok_or(ConsensusError::InvalidAuthoritiesSet)
}

/// Errors in slot and seal verification.
#[derive(Debug, thiserror::Error)]
pub enum SealVerificationError<Header> {
	/// Header is deferred to the future.
	#[error("Header slot is in the future")]
	Deferred(Header, Slot),

	/// The header has no seal digest.
	#[error("Header is unsealed.")]
	Unsealed,

	/// The header has a malformed seal.
	#[error("Header has a malformed seal")]
	BadSeal,

	/// The header has a bad signature.
	#[error("Header has a bad signature")]
	BadSignature,

	/// No slot author found.
	#[error("No slot author for provided slot")]
	SlotAuthorNotFound,

	/// Header has no valid slot pre-digest.
	#[error("Header has no valid slot pre-digest")]
	InvalidPreDigest(PreDigestLookupError),
}

/// Check a header has been signed by the right key. If the slot is too far in
/// the future, an error will be returned. If it's successful, returns the
/// pre-header (i.e. without the seal), the slot, and the digest item containing
/// the seal.
///
/// Note that this does not check for equivocations, and [`check_equivocation`]
/// is recommended for that purpose.
///
/// This digest item will always return `Some` when used with `as_SPIN_seal`.
pub fn check_header_slot_and_seal<B: BlockT, P: Pair>(
	slot_now: Slot,
	mut header: B::Header,
	aux_data: &SpinAuxData<AuthorityId<P>>,
) -> Result<(B::Header, Slot, DigestItem), SealVerificationError<B::Header>>
where
	P::Signature: Codec,
	P::Public: Codec + PartialEq + Clone,
{
	let (authorities, session_length) = aux_data;
	let seal = header.digest_mut().pop().ok_or(SealVerificationError::Unsealed)?;

	let sig = seal.as_spin_seal().ok_or(SealVerificationError::BadSeal)?;

	let slot = find_pre_digest::<B, P::Signature>(&header)
		.map_err(SealVerificationError::InvalidPreDigest)?;

	if slot > slot_now {
		header.digest_mut().push(seal);
		return Err(SealVerificationError::Deferred(header, slot));
	} else {
		// check the signature is valid under the expected authority and
		// chain state.
		let expected_author = slot_author::<P>(slot, *session_length, &authorities)
			.ok_or(SealVerificationError::SlotAuthorNotFound)?;

		let pre_hash = header.hash();

		if P::verify(&sig, pre_hash.as_ref(), expected_author) {
			Ok((header, slot, seal))
		} else {
			Err(SealVerificationError::BadSignature)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_keyring::sr25519::Keyring;

	#[test]
	fn test_claim_slot_session_handling() {
		let authorities = vec![
			Keyring::Alice.public().into(),
			Keyring::Bob.public().into(),
			Keyring::Charlie.public().into(),
			Keyring::Dave.public().into(),
			Keyring::Eve.public().into(),
		];
		let session_length = 3;

		let slot = 0.into();
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Alice.public().into())
		);
		let slot = 1.into();
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Alice.public().into())
		);
		let slot = 2.into();
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Alice.public().into())
		);
		let slot = 3.into();
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Bob.public().into())
		);
		let slot = 4.into();
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Bob.public().into())
		);

		let slot = 30.into();
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Alice.public().into())
		);

		let slot = 311.into();
		// session_idx is 103, which is 3 % 5 = 3
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Dave.public().into())
		);

		let slot = u64::MAX.into();
		// session_idx is 6148914691236517203, which is 0 % 5 = 0
		assert_eq!(
			slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
				slot,
				session_length,
				&authorities
			),
			Some(&Keyring::Alice.public().into())
		);
	}

	#[test]
	#[should_panic]
	fn test_claim_slot_session_handling_zero_session_length() {
		let authorities = vec![
			Keyring::Alice.public().into(),
			Keyring::Bob.public().into(),
			Keyring::Charlie.public().into(),
			Keyring::Dave.public().into(),
			Keyring::Eve.public().into(),
		];
		let session_length = 0;

		let slot = 0.into();
		slot_author::<qfp_consensus_spin::sr25519::AuthorityPair>(
			slot,
			session_length,
			&authorities,
		);
	}

	// #[test]
	// fn authorities_call_works() {
	//     let client = substrate_test_runtime_client::new();

	//     assert_eq!(client.chain_info().best_number, 0);
	//     assert_eq!(
	//         fetch_authorities_with_compatibility_mode(
	//             &client,
	//             client.chain_info().best_hash,
	//             1,
	//             &CompatibilityMode::None
	//         )
	//         .unwrap(),
	//         vec![
	//             Keyring::Alice.public().into(),
	//             Keyring::Bob.public().into(),
	//             Keyring::Charlie.public().into()
	//         ]
	//     );

	//     assert_eq!(
	//         fetch_aux_data(&client, client.chain_info().best_hash).unwrap(),
	//         vec![
	//             Keyring::Alice.public().into(),
	//             Keyring::Bob.public().into(),
	//             Keyring::Charlie.public().into()
	//         ]
	//     );
	// }
}
