// Copyright (C) Quantum Fusion Network, 2025.
// Copyright (C) Parity Technologies (UK) Ltd., until 2025.
// SPDX-License-Identifier: Apache-2.0

//! Module implementing the logic for verifying and importing SPIN blocks.

use crate::{
	aux_data, standalone::SealVerificationError, AuthorityId, CompatibilityMode, Error,
	SpinAuxData, LOG_TARGET,
};
use codec::Codec;
use log::{debug, info, trace};
use prometheus_endpoint::Registry;
use qfp_consensus_spin::{inherents::SpinInherentData, SpinApi};
use sc_client_api::{backend::AuxStore, BlockOf, UsageProvider};
use sc_consensus::{
	block_import::{BlockImport, BlockImportParams, ForkChoiceStrategy},
	import_queue::{BasicQueue, BoxJustificationImport, DefaultImportQueue, Verifier},
};
use sc_consensus_slots::{check_equivocation, CheckedHeader, InherentDataProviderExt};
use sc_telemetry::{telemetry, TelemetryHandle, CONSENSUS_DEBUG, CONSENSUS_TRACE};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::Error as ConsensusError;
use sp_consensus_slots::Slot;
use sp_core::crypto::Pair;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider as _};
use sp_runtime::{
	traits::{Block as BlockT, Header, NumberFor},
	DigestItem,
};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};

/// check a header has been signed by the right key. If the slot is too far in
/// the future, an error will be returned. If it's successful, returns the
/// pre-header and the digest item containing the seal.
///
/// This digest item will always return `Some` when used with `as_spin_seal`.
fn check_header<C, B: BlockT, P: Pair>(
	client: &C,
	slot_now: Slot,
	header: B::Header,
	hash: B::Hash,
	aux_data: &SpinAuxData<AuthorityId<P>>,
	check_for_equivocation: CheckForEquivocation,
) -> Result<CheckedHeader<B::Header, (Slot, DigestItem)>, Error<B>>
where
	P::Public: Codec,
	P::Signature: Codec,
	C: sc_client_api::backend::AuxStore,
{
	let check_result =
		crate::standalone::check_header_slot_and_seal::<B, P>(slot_now, header, aux_data);

	match check_result {
		Ok((header, slot, seal)) => {
			let (authorities, session_index) = aux_data;
			let expected_author =
				crate::standalone::slot_author::<P>(slot, *session_index, &authorities);
			let should_equiv_check = check_for_equivocation.check_for_equivocation();
			if let (true, Some(expected)) = (should_equiv_check, expected_author) {
				if let Some(equivocation_proof) =
					check_equivocation(client, slot_now, slot, &header, expected)
						.map_err(Error::Client)?
				{
					info!(
						target: LOG_TARGET,
						"Slot author is equivocating at slot {} with headers {:?} and {:?}",
						slot,
						equivocation_proof.first_header.hash(),
						equivocation_proof.second_header.hash(),
					);
				}
			}

			Ok(CheckedHeader::Checked(header, (slot, seal)))
		},
		Err(SealVerificationError::Deferred(header, slot)) =>
			Ok(CheckedHeader::Deferred(header, slot)),
		Err(SealVerificationError::Unsealed) => Err(Error::HeaderUnsealed(hash)),
		Err(SealVerificationError::BadSeal) => Err(Error::HeaderBadSeal(hash)),
		Err(SealVerificationError::BadSignature) => Err(Error::BadSignature(hash)),
		Err(SealVerificationError::SlotAuthorNotFound) => Err(Error::SlotAuthorNotFound),
		Err(SealVerificationError::InvalidPreDigest(e)) => Err(Error::from(e)),
	}
}

/// A verifier for SPIN blocks.
pub struct SpinVerifier<C, P, CIDP, N> {
	client: Arc<C>,
	create_inherent_data_providers: CIDP,
	check_for_equivocation: CheckForEquivocation,
	telemetry: Option<TelemetryHandle>,
	compatibility_mode: CompatibilityMode<N>,
	_phantom: PhantomData<fn() -> P>,
}

impl<C, P, CIDP, N> SpinVerifier<C, P, CIDP, N> {
	pub(crate) fn new(
		client: Arc<C>,
		create_inherent_data_providers: CIDP,
		check_for_equivocation: CheckForEquivocation,
		telemetry: Option<TelemetryHandle>,
		compatibility_mode: CompatibilityMode<N>,
	) -> Self {
		Self {
			client,
			create_inherent_data_providers,
			check_for_equivocation,
			telemetry,
			compatibility_mode,
			_phantom: PhantomData,
		}
	}
}

impl<C, P, CIDP, N> SpinVerifier<C, P, CIDP, N>
where
	CIDP: Send,
{
	async fn check_inherents<B: BlockT>(
		&self,
		block: B,
		at_hash: B::Hash,
		inherent_data: sp_inherents::InherentData,
		create_inherent_data_providers: CIDP::InherentDataProviders,
	) -> Result<(), Error<B>>
	where
		C: ProvideRuntimeApi<B>,
		C::Api: BlockBuilderApi<B>,
		CIDP: CreateInherentDataProviders<B, ()>,
	{
		let inherent_res = self
			.client
			.runtime_api()
			.check_inherents(at_hash, block, inherent_data)
			.map_err(|e| Error::Client(e.into()))?;

		if !inherent_res.ok() {
			for (i, e) in inherent_res.into_errors() {
				match create_inherent_data_providers.try_handle_error(&i, &e).await {
					Some(res) => res.map_err(Error::Inherent)?,
					None => return Err(Error::UnknownInherentError(i)),
				}
			}
		}

		Ok(())
	}
}

#[async_trait::async_trait]
impl<B: BlockT, C, P, CIDP> Verifier<B> for SpinVerifier<C, P, CIDP, NumberFor<B>>
where
	C: ProvideRuntimeApi<B> + Send + Sync + sc_client_api::backend::AuxStore,
	C::Api: BlockBuilderApi<B> + SpinApi<B, AuthorityId<P>> + ApiExt<B>,
	P: Pair,
	P::Public: Codec + Debug,
	P::Signature: Codec,
	CIDP: CreateInherentDataProviders<B, ()> + Send + Sync,
	CIDP::InherentDataProviders: InherentDataProviderExt + Send + Sync,
{
	async fn verify(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		// Skip checks that include execution, if being told so or when importing only
		// state.
		//
		// This is done for example when gap syncing and it is expected that the block
		// after the gap was checked/chosen properly, e.g. by warp syncing to this
		// block using a finality proof. Or when we are importing state only and can
		// not verify the seal.
		if block.with_state() || block.state_action.skip_execution_checks() {
			// When we are importing only the state of a block, it will be the best block.
			block.fork_choice = Some(ForkChoiceStrategy::Custom(block.with_state()));

			return Ok(block);
		}

		let hash = block.header.hash();
		let parent_hash = *block.header.parent_hash();
		let aux_data = aux_data(
			self.client.as_ref(),
			parent_hash,
			*block.header.number(),
			&self.compatibility_mode,
		)
		.map_err(|e| format!("Could not fetch authorities at {:?}: {}", parent_hash, e))?;

		let create_inherent_data_providers = self
			.create_inherent_data_providers
			.create_inherent_data_providers(parent_hash, ())
			.await
			.map_err(|e| Error::<B>::Client(sp_blockchain::Error::Application(e)))?;

		let mut inherent_data = create_inherent_data_providers
			.create_inherent_data()
			.await
			.map_err(Error::<B>::Inherent)?;

		let slot_now = create_inherent_data_providers.slot();

		// we add one to allow for some small drift.
		// FIXME #1019 in the future, alter this queue to allow deferring of
		// headers
		let checked_header = check_header::<C, B, P>(
			&self.client,
			slot_now + 1,
			block.header,
			hash,
			&aux_data,
			self.check_for_equivocation,
		)
		.map_err(|e| e.to_string())?;
		match checked_header {
			CheckedHeader::Checked(pre_header, (slot, seal)) => {
				// if the body is passed through, we need to use the runtime
				// to check that the internally-set timestamp in the inherents
				// actually matches the slot set in the seal.
				if let Some(inner_body) = block.body.take() {
					let new_block = B::new(pre_header.clone(), inner_body);

					inherent_data.spin_replace_inherent_data(slot);

					// skip the inherents verification if the runtime API is old or not expected to
					// exist.
					if self
						.client
						.runtime_api()
						.has_api_with::<dyn BlockBuilderApi<B>, _>(parent_hash, |v| v >= 2)
						.map_err(|e| e.to_string())?
					{
						self.check_inherents(
							new_block.clone(),
							parent_hash,
							inherent_data,
							create_inherent_data_providers,
						)
						.await
						.map_err(|e| e.to_string())?;
					}

					let (_, inner_body) = new_block.deconstruct();
					block.body = Some(inner_body);
				}

				trace!(target: LOG_TARGET, "Checked {:?}; importing.", pre_header);
				telemetry!(
					self.telemetry;
					CONSENSUS_TRACE;
					"spin.checked_and_importing";
					"pre_header" => ?pre_header,
				);

				block.header = pre_header;
				block.post_digests.push(seal);
				block.fork_choice = Some(ForkChoiceStrategy::LongestChain);
				block.post_hash = Some(hash);

				Ok(block)
			},
			CheckedHeader::Deferred(a, b) => {
				debug!(target: LOG_TARGET, "Checking {:?} failed; {:?}, {:?}.", hash, a, b);
				telemetry!(
					self.telemetry;
					CONSENSUS_DEBUG;
					"spin.header_too_far_in_future";
					"hash" => ?hash,
					"a" => ?a,
					"b" => ?b,
				);
				Err(format!("Header {:?} rejected: too far in the future", hash))
			},
		}
	}
}

/// Should we check for equivocation of a block author?
#[derive(Debug, Clone, Copy)]
pub enum CheckForEquivocation {
	/// Yes, check for equivocation.
	///
	/// This is the default setting for this.
	Yes,
	/// No, don't check for equivocation.
	No,
}

impl CheckForEquivocation {
	/// Should we check for equivocation?
	fn check_for_equivocation(self) -> bool {
		matches!(self, Self::Yes)
	}
}

impl Default for CheckForEquivocation {
	fn default() -> Self {
		Self::Yes
	}
}

/// Parameters of [`import_queue`].
pub struct ImportQueueParams<'a, Block: BlockT, I, C, S, CIDP> {
	/// The block import to use.
	pub block_import: I,
	/// The justification import.
	pub justification_import: Option<BoxJustificationImport<Block>>,
	/// The client to interact with the chain.
	pub client: Arc<C>,
	/// Something that can create the inherent data providers.
	pub create_inherent_data_providers: CIDP,
	/// The spawner to spawn background tasks.
	pub spawner: &'a S,
	/// The prometheus registry.
	pub registry: Option<&'a Registry>,
	/// Should we check for equivocation?
	pub check_for_equivocation: CheckForEquivocation,
	/// Telemetry instance used to report telemetry metrics.
	pub telemetry: Option<TelemetryHandle>,
	/// Compatibility mode that should be used.
	///
	/// If in doubt, use `Default::default()`.
	pub compatibility_mode: CompatibilityMode<NumberFor<Block>>,
}

/// Start an import queue for the SPIN consensus algorithm.
pub fn import_queue<P, Block, I, C, S, CIDP>(
	ImportQueueParams {
		block_import,
		justification_import,
		client,
		create_inherent_data_providers,
		spawner,
		registry,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	}: ImportQueueParams<Block, I, C, S, CIDP>,
) -> Result<DefaultImportQueue<Block>, sp_consensus::Error>
where
	Block: BlockT,
	C::Api: BlockBuilderApi<Block> + SpinApi<Block, AuthorityId<P>> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>,
	I: BlockImport<Block, Error = ConsensusError> + Send + Sync + 'static,
	P: Pair + 'static,
	P::Public: Codec + Debug,
	P::Signature: Codec,
	S: sp_core::traits::SpawnEssentialNamed,
	CIDP: CreateInherentDataProviders<Block, ()> + Sync + Send + 'static,
	CIDP::InherentDataProviders: InherentDataProviderExt + Send + Sync,
{
	let verifier = build_verifier::<P, _, _, _>(BuildVerifierParams {
		client,
		create_inherent_data_providers,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	});

	Ok(BasicQueue::new(verifier, Box::new(block_import), justification_import, spawner, registry))
}

/// Parameters of [`build_verifier`].
pub struct BuildVerifierParams<C, CIDP, N> {
	/// The client to interact with the chain.
	pub client: Arc<C>,
	/// Something that can create the inherent data providers.
	pub create_inherent_data_providers: CIDP,
	/// Should we check for equivocation?
	pub check_for_equivocation: CheckForEquivocation,
	/// Telemetry instance used to report telemetry metrics.
	pub telemetry: Option<TelemetryHandle>,
	/// Compatibility mode that should be used.
	///
	/// If in doubt, use `Default::default()`.
	pub compatibility_mode: CompatibilityMode<N>,
}

/// Build the [`SpinVerifier`]
pub fn build_verifier<P, C, CIDP, N>(
	BuildVerifierParams {
		client,
		create_inherent_data_providers,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	}: BuildVerifierParams<C, CIDP, N>,
) -> SpinVerifier<C, P, CIDP, N> {
	SpinVerifier::<_, P, _, _>::new(
		client,
		create_inherent_data_providers,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	)
}
